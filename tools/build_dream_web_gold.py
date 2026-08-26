#!/usr/bin/env python3
"""Build the reviewed Dream web audit and high-confidence population gold set."""

import json
from pathlib import Path

SOURCE = Path("target/dream-web-audit-v1.json")
AUDIT_OUT = Path("corpus/dream-web-audit-v1.json")
GOLD_OUT = Path("corpus/dream-web-gold-v1.json")

REMOVE = {19, 38, 39, 45, 80, 83, 88, 90, 92, 99}
REVIEW = {37, 52, 60, 64, 65, 72}

REASONS = {
    19: "creatureServer retrospective is unrelated to the later active-project implementation-status summary",
    38: "later active-project status is unrelated to creatureServer being the first non-guided project",
    39: "later active-project status is unrelated to the already-mothballed creatureServer project",
    45: "creatureServer history is unrelated to the later project logger addition",
    80: "ship-variant architecture and bullet/asteroid scene existence share only broad game context",
    83: "ship-variant architecture and asteroid spawn/update ordering are separate subsystems",
    88: "asteroid spawn/update ordering and multiplayer game-start status are separate subsystems",
    90: "creatureServer retrospective is unrelated to the later client input-packet implementation",
    92: "creatureServer retrospective is unrelated to the later WebSocket implementation",
    99: "asteroid collision polygons and ship collision-shape lookup share terminology without a demonstrated semantic dependency",
    37: "general later implementation-status summary may or may not usefully link to the earlier asteroid scope",
    52: "multiplayer game-start status and bullet/asteroid scene existence may be only broad same-game context",
    60: "multiplayer menu UI and gameplay input-packet sending may be only broad same-game context",
    64: "multiplayer menu UI and server Player response may be only broad networking context",
    65: "multiplayer menu UI and WebSocket establishment may be a real dependency or merely broad networking context",
    72: "planned ship types and bullet/asteroid scene existence may be only broad same-game context",
}

KIND_NOTES = {
    30: "strong factual candidate: calibration instruction directly targets the shared C-family adapter",
    47: "strong factual candidate: this is the packet the client is sending",
    56: "strong factual candidate: shared hue rules directly elaborate the identity-coloring decision",
    68: "strong factual candidate: byte-equivalence is a concrete preservation requirement",
    75: "strong factual candidate: helper instruction directly implements remote hue identity",
    76: "strong factual candidate: helper instruction directly implements shared hue rules",
    97: "strong factual candidate: viewport collision polygons directly support texture/collision-shape matching",
    102: "strong factual candidate: C workspace contents are governed by the working-repository location rule",
    103: "strong factual candidate: repository-location rule locates the workspace-mcp file",
}

MISSING_TROUBLESHOOTING = {
    "id": "web-missing-troubleshooting-style",
    "left": "6a4bca1977cb031206fc9456dbbedd8984d3fb71d665b559e23d817c5e69853b",
    "right": "850b5789cda57858c0c98c52d45d82e09b44e66ca79885ebda963ff7b53071af",
    "present": True,
}


def main():
    source = json.loads(SOURCE.read_text(encoding="utf-8"))
    if len(source) != 103:
        raise SystemExit(f"expected 103 source pairs, found {len(source)}")

    audit = []
    for index, row in enumerate(source, 1):
        judgment = "remove" if index in REMOVE else "review" if index in REVIEW else "keep"
        kinds = row["kinds"]
        audit.append(
            {
                "index": index,
                "left": row["a"],
                "left_title": row["a_title"],
                "right": row["b"],
                "right_title": row["b_title"],
                "current_kind": kinds[0] if len(kinds) == 1 else kinds,
                "judgment": judgment,
                "reason": REASONS.get(index),
                "kind_note": KIND_NOTES.get(index),
            }
        )

    AUDIT_OUT.write_text(
        json.dumps(
            {
                "format": "dream-web-audit-v1",
                "source_report": "dream-tune-full-20260826-codex-sol-low-v2",
                "standard": "Keep only semantically useful traversal relations; broad same-project/domain overlap alone is insufficient.",
                "summary": {
                    "pairs": len(audit),
                    "keep": sum(item["judgment"] == "keep" for item in audit),
                    "remove": sum(item["judgment"] == "remove" for item in audit),
                    "review": sum(item["judgment"] == "review" for item in audit),
                    "kind_review": len(KIND_NOTES),
                },
                "pairs": audit,
            },
            indent=2,
        ),
        encoding="utf-8",
    )

    final_pairs = [
        {
            "id": f"web-{item['index']:03d}",
            "left": item["left"],
            "right": item["right"],
            "present": item["judgment"] == "keep",
        }
        for item in audit
        if item["judgment"] != "review"
    ]
    final_pairs.append(MISSING_TROUBLESHOOTING)
    GOLD_OUT.write_text(
        json.dumps(
            {
                "format": "dream-web-gold-v1",
                "source": "manual full-web audit of the 46-Memory production Dream run",
                "notes": "Presence/absence only. Six borderline broad-context pairs are deliberately excluded; exact kind/direction remains owned by synthetic contracts and future population labels.",
                "excluded_review_indices": sorted(REVIEW),
                "pairs": [],
                "final_pairs": final_pairs,
                "remaining_extracted": 0,
                "lifecycle": [],
            },
            indent=2,
        ),
        encoding="utf-8",
    )
    print(f"audit={len(audit)} gold={len(final_pairs)} remove={len(REMOVE)} review={len(REVIEW)}")


if __name__ == "__main__":
    main()
