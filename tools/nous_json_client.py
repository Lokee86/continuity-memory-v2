import json
import time
import urllib.error
import urllib.request


class NousError(RuntimeError):
    pass


def _parse_json_text(text):
    text = text.strip()
    if text.startswith("```"):
        first = text.find("\n")
        last = text.rfind("```")
        if first >= 0 and last > first:
            text = text[first + 1:last].strip()
    return json.loads(text)


def _stream_content(response):
    content = []
    tool_arguments = {}
    usage = {}
    diagnostics = {"finish_reasons": [], "delta_keys": set(), "reasoning_chars": 0, "errors": []}
    for raw in response:
        line = raw.decode("utf-8", errors="replace").strip()
        if not line.startswith("data:"):
            continue
        data = line[5:].strip()
        if not data or data == "[DONE]":
            continue
        chunk = json.loads(data)
        if chunk.get("usage"):
            usage = chunk["usage"]
        if chunk.get("error"):
            diagnostics["errors"].append(str(chunk["error"])[:500])
        for choice in chunk.get("choices", []):
            if choice.get("finish_reason"):
                diagnostics["finish_reasons"].append(choice["finish_reason"])
            delta = choice.get("delta", {})
            diagnostics["delta_keys"].update(delta.keys())
            reasoning = delta.get("reasoning")
            if isinstance(reasoning, str):
                diagnostics["reasoning_chars"] += len(reasoning)
            text = delta.get("content")
            if isinstance(text, str):
                content.append(text)
            elif isinstance(text, list):
                for item in text:
                    if isinstance(item, dict) and isinstance(item.get("text"), str):
                        content.append(item["text"])
            for call in delta.get("tool_calls") or []:
                index = call.get("index", 0)
                function = call.get("function") or {}
                arguments = function.get("arguments")
                if isinstance(arguments, str):
                    tool_arguments.setdefault(index, []).append(arguments)
    diagnostics["delta_keys"] = sorted(diagnostics["delta_keys"])
    tool_text = "".join(tool_arguments[min(tool_arguments)]) if tool_arguments else ""
    return "".join(content), tool_text, usage, diagnostics


def complete_json(
    endpoint,
    api_key,
    model,
    system_prompt,
    payload,
    timeout=240,
    retries=3,
    reasoning_effort="low",
    schema_name=None,
    schema=None,
    structured_mode="response_format",
):
    function_name = schema_name or "structured_output"
    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": json.dumps(payload, ensure_ascii=False)},
        ],
        "stream": True,
        "include_reasoning": True,
        "max_tokens": 16384,
    }
    if structured_mode == "tool" and schema is not None:
        body["tools"] = [{
            "type": "function",
            "function": {
                "name": function_name,
                "description": "Return the required structured result.",
                "parameters": schema,
                "strict": True,
            },
        }]
        body["tool_choice"] = {"type": "function", "function": {"name": function_name}}
    else:
        response_format = {"type": "json_object"}
        if schema is not None:
            response_format = {
                "type": "json_schema",
                "json_schema": {
                    "name": function_name,
                    "strict": True,
                    "schema": schema,
                },
            }
        body["response_format"] = response_format
    if reasoning_effort:
        body["reasoning_effort"] = reasoning_effort
    encoded = json.dumps(body, ensure_ascii=False).encode("utf-8")
    last_error = None
    for attempt in range(retries + 1):
        request = urllib.request.Request(
            endpoint,
            data=encoded,
            method="POST",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
                "Accept": "text/event-stream",
                "User-Agent": "continuity-insomnia-tuning/1",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                text, tool_text, usage, diagnostics = _stream_content(response)
            structured_text = tool_text if structured_mode == "tool" else text
            if not structured_text.strip():
                raise NousError(f"stream contained no structured content: {diagnostics}")
            try:
                return _parse_json_text(structured_text), usage
            except json.JSONDecodeError as error:
                raise NousError(
                    f"invalid structured message content ({error}); diagnostics={diagnostics}; "
                    f"preview={structured_text[:500]!r}"
                ) from error
        except (
            urllib.error.HTTPError,
            urllib.error.URLError,
            TimeoutError,
            json.JSONDecodeError,
            NousError,
        ) as error:
            last_error = error
            retryable = not isinstance(error, urllib.error.HTTPError) or error.code in {
                408, 409, 429, 500, 502, 503, 504, 524
            }
            if attempt >= retries or not retryable:
                break
            if isinstance(error, urllib.error.HTTPError) and error.code == 429:
                delay = min(60, 10 * (2 ** attempt))
            else:
                delay = min(30, 2 ** attempt)
            time.sleep(delay)
    if isinstance(last_error, urllib.error.HTTPError):
        try:
            detail = last_error.read().decode("utf-8", errors="replace")[:1000]
        except Exception:
            detail = ""
        raise NousError(f"HTTP {last_error.code}: {detail}") from last_error
    raise NousError(str(last_error)) from last_error
