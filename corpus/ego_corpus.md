# Recent Conversation Content

Users recent ChatGPT conversations, including timestamps, titles, and messages. Use it to maintain continuity when relevant.Default timezone is -0700.User messages are delimited by ||||.Assistant messages are delimited by ::::.

1. 20260814T11:29 CVA Architecture Update:<<conversation too long; truncated>>||||# Context handoff — Continuity Memory v2

**Repository:** `C:\!bin\workspace\continuity-memory-v2`
**Branch:** `main`
**State:** dirty/uncommitted. The rebuild is still intentionally uncommitted; do not assume any of this is safely preserved in Git yet.

## Core architecture now

The governing model is:

```
CVA
└── global_version: u64
    └── Archive
        ├── archive_version: u64
        └── conversation/session-local ancestry

```

A `.cva` is **one physical file containing several purpose-built databases**, not one generalized database.

The **CVA-global version** is ordering only. It answers when mutations across future databases entered the shared file ordering. It is not a root, state identity, or semantic parent.

The **Archive-local version** is a dense whole-Archive watermark. `A=500` means “the Archive after its first 500 semantic mutations.” It is not ancestry either.

Actual conversation ancestry belongs to individual nodes:

```
conversation A:
A1 -> A2 -> A3

conversation B:
B1 -> B2 -> B3

```

Their writes can physically interleave without creating semantic relationships between them.

## Important correction made this session

The first history implementation incorrectly gave the entire Archive one parent-linked publication chain:

```
P1 -> P2 -> P3 -> P4

```

That was removed.

It would have forced unrelated concurrent conversations through a shared semantic Archive head.

Current semantic Archive writes instead receive:

```
ArchiveRecordVersion {
    global_version,
    archive_version,
    record: ChunkRef,
}

```

There is **no Archive-wide parent pointer**.

Conversation branching remains local.

## Session / branch behavior

Nodes are immutable and contain `conversation_id` plus `parent_id`.

Logical branch/session heads are append-only revisions of:

```
(conversation_id, branch_id) -> leaf_node_id

```

An existing branch head may only advance to a descendant.

If a user revives an old conversation point:

```
A1 -> A2 -> A3
       \
        A4' -> A5'

```

the correct operation is to create a **new branch/session identity** at `A2` and continue from there.

The Archive itself does not rewind. Other conversations do not move.

Historical branch-head lookup is implemented with:

```
branch_at(conversation_id, branch_id, archive_version)

```

## Persistent format

Physical CVA format remains development format `1.0`.

Relevant records:

```
CVAVERS1   global u64 version ticket

CVAAFMT2   current Archive development-format marker

CVAAREC1
    global_version
    archive_version
    semantic record ChunkRef

```

Archive semantic records currently include immutable nodes, branch-head revisions, and fragments.

Content bodies are content-addressed and do **not** independently advance Archive semantic versioning. The node referencing them does.

Current ordering overhead remains **72 bytes per semantic Archive mutation**:

```
24 bytes  global version ticket chunk
48 bytes  Archive record-version metadata chunk

```

## Migration policy

**There is no migration code. Keep it that way for now.**

The previous `CVAAFMT1` / `CVAAPUB1` / `CVAASEL1` development design is retired.

Old development CVAs are rejected rather than migrated or silently reinterpreted.

Migration should remain a separate future tool after the storage model is stable enough to deserve compatibility guarantees.

## Archive functionality implemented

Current rebuild has:

- physical CVA header/chunk create/open/read/append;
- global version clock;
- Archive-local version clock;
- SHA-256 content-body ded||||Hold it, where were you?||||Ok. what is our plan right now||||Ok... I got lost somewhere, last thing I remember talking to you about had to do with publication records and... I can't remember before that||||How is ArchiveRecordVersion able to recover/reset the stat of all the conversations in the archive?||||Ok, and that's the performance part we were addressing?||||Ok, and you were in the middle of instrumenting that to figure out what to actually change?||||Ok. Finish the instrumentation and get the info||||Ok, lets replace the current composite-string derived maps with compact Archive-owned lookup structures as you suggested.||||What? Have you commited anything for this yet?||||Good god. Deal with that, why the fuck would you do that?||||Well that's less than ideal. Is it at least stable and fucntioning?||||Alright, where were we then||||Alright, do it||||semantic?||||So we now have 1 scan that works for everything instead of 4 separate scans?||||what was the time reduction?||||and we have no cold measurement? That's probably what's actually important, opening warm CVAs isn't going to be a common operation, if it ever happens

2. 20260814T00:52 Prototype and Fundraising Clarity:<<conversation too long; truncated>>||||Ok, so... I feel||||So, what about the personalization part? Ego doesn't really fit into the "problem" model anywhere, it's useful, and it solves problems not listed, but its slightly different||||We already have a two tier problem section, we were just working on that||||Ok, what does our problems section look like so far. Reader's digest here||||that's the plan then||||Alright, so the next section is "how the product solves those problems"?||||Alright, what do we need to plan here?||||I think the solution section should stick to our rule of describing the final end product with a reference to the "Curren Implementation" and "Technical & Benchmark" sections||||No, near the end.||||The remaining question for the Solution section is mostly **how to organize the explanation**. I think the cleanest approach is to follow the same five problems in the same order, but write each as a solution rather than as an architecture/component list. That gives the reader an obvious problem → answer mapping without making CVA, Insomnia, Dream, and Ego the organizing principle.  

I like this idea, *but*... I guess it's a business document, "reading flow" isn't generally a consideration...||||Alright, so then how are we explaining our solutions

3. 20260814T12:55 Git Workspace Issue:<<conversation too long; truncated>>||||There were **two separate problems**: the actual Warlock repository situation we were trying to clean up, and then a Workspace/tooling failure that prevented me from safely completing it.

### What we were trying to fix

Warlock currently has two frontend histories that diverged for architectural reasons.

`main` is the current Warlock codebase and still uses the **React UI**. Separately, `feat/svelte-ui-migration` contains a fairly substantial **Svelte rebuild** that was created when we were planning to integrate Warlock with CTX differently. That architectural direction is no longer current, so there isn't much justification for maintaining two complete frontend implementations indefinitely.

But the Svelte work wasn't useless. It contains actual product improvements that either aren't present in React or appear better developed there. We had already identified things such as the **Homunculus UI**, better **current-repository selection/navigation**, disabling repository-dependent actions when no repo is selected, **URL/deep-link/history routing**, and potentially useful improvements around Pitlord, layout, and service abstractions.

So your instruction was essentially:

1. **Preserve the Svelte rebuild||||Diagnose that problem, you don't seem to be having an issue anywhere else||||Alright, let's get that resolved

4. 20260814T12:53 Branch · Prototype and Fundraising Clarity:<<conversation too long; truncated>>||||Ok, that's been recorded now. Let's||||The root core issue that our storage engine solves is that models have no access to prior history or context of any kind. Every fresh session is a completely fresh slate, which can be good, but most often is a giant pain in the ass. Our engine gives *any* model or platform RELIABLE access to your ingested former history.

This is the problem we've already proven we can fix||||So the problems we solve are

Reliable access to prior history and context
de-siloing/vendor lock-in

there's others that are escaping me right now.||||Ok, so... I feel like this is the place to get verbose and explain as much as possible? Not necessarily *over* explain everything, but is there a reason to leave the sub-problems out of this section?||||So, what about the personalization part? Ego doesn't really fit into the "problem" model anywhere, it's useful, and it solves problems not listed, but its slightly different||||We already have a two tier problem section, we were just working on that||||Ok, what does our problems section look like so far. Reader's digest here||||that's the plan then||||@Workspace Record that.||||You just recorded the outline?||||No||||That's what I wante

5. 20260814T13:09 Missing Obsidian Vault:||||So... I'm missing an obsidian vault. I thought it was the laughing skull one that had the most up to date info, but it doesn't appear to be. The one we're using is out-dated on everything except Continuity business plans. The one we're missing has Devlog outlines in it, this one does not.

6. 20260814T11 Branch · Svelte migration status:<<conversation too long; truncated>>||||I’m not going to claim completion from the interrupted tool output. The preservation operation was in progress, but I no longer have a reliable result showing whether the generated directories were successfully removed from staging or whether the commit was created.

The safe state is: **no Svelte source was intentionally deleted or rewritten**. The generated-output cleanup targeted only `build-svelte-check/`, `.arcana/`, `.grimoire/`, and `.lexicon/`. The actual Svelte rebuild remains in the worktree.

For the feature audit, the strongest improvements already identified are:

- **Homunculus UI — port.** The Svelte rebuild has a first-class Homunculus interface (`HomunculusView`, form, result display); React `main` does not appear to have an equivalent frontend. 
- **Current-repository navigation — port.** Svelte adds a persistent current-repo indicator, a “Change Repo” action, and disables repo-dependent tools until a repository is selected. 
- **URL/deep-link routing — likely port.** It carries `view`, `project`, document path||||Finish it please||||@Workspace Try to finish it now.||||Could you explain in detail what the problem is and what you were trying to fix

7. 20260814T11 Branch · Prototype and Fundraising Clarity:<<conversation too long; truncated>>||||So, just to be clear, we should DEFINITELY have a working prototype before we launch funding yeah? I mean we HAD one, but... I'm not sure what happened to it exactly, you kind of screwed that up.||||and part of the diligence is a coherent and complete business plan yeah?||||So, where do we start? ||||Alright, do the audit||||So we need to get rid of all the stale and duplicated information and have one story first?||||Alright. Let's do that then||||Let's walk through the whole thing instead of whatever you're trying to do here||||provenance and lifecycle?||||That just sounds so dumb "Memory is the mechanism"||||There's something about the way you're describing this that seems off... I'm not sure what it is||||We haven't mentioned de-siloing, I think that's the biggest thing, it's *provider* independent, and you can own the software, removing any sort of external third-party dependencies. Every other memory product requires at a minimum an embedded SQLite database, but most use external Postgres implementations or managed platforms. Independence is one of our biggest selling points I think, "buy our product, never talk to us again" is completely viable for us, much less so with basically every other memory product available||||Yes. The only thing I'd add, and I'm not sure how, or where, I'd add it, is that I've noticed we're truly building a system with no lock-in. Current memory "import" systems are... weak as fuck? I think might be the best way to put it. Like ChatGPT and Claude's "Import memories from another provider" option is that they give you a prompt to give to the robot to dump memory that can then be "transferred" to the new provider.||||We need to be careful about this too. Currently we can't actually support provider subscriptions directly. Ideally, in the end picture, we get integration with all of the major providers as a memory plugin or option. As it stands right now the only way to use Claude or ChatGpt with this is to export your conversation history and run the pipelines over it.

My point was that the system we're building doesn't obfuscate any part of the memory data from the user. Most provider systems obfuscate MOST of the memory data from users||||**Continuity takes AI memory out of provider silos and puts it under the user's control. It stores source history, memories, and the state built from them in a self-contained archive that the customer owns and can inspect. The archive does not depend on a particular model provider, managed memory platform, or generalized database deployment.**





> should this next paragraph describe what exists? Or should it describe what we've planned an CAN implement but haven't? Should this paragraph exist at all?



**Today, existing provider histories can be imported into Continuity. Longer term, the goal is direct integration so the same independent memory can serve multiple AI systems without becoming any provider's proprietary internal state.**||||That's the right idea. Keep present implementation status to a specific and dated place in the plan||||write that down somewhere conspicuous||||Ok, what were we at here?||||actually write it down @Workspace 

8. 20260814T11 Svelte migration status:<<conversation too long; truncated>>||||I’m not going to claim completion from the interrupted tool output. The preservation operation was in progress, but I no longer have a reliable result showing whether the generated directories were successfully removed from staging or whether the commit was created.

The safe state is: **no Svelte source was intentionally deleted or rewritten**. The generated-output cleanup targeted only `build-svelte-check/`, `.arcana/`, `.grimoire/`, and `.lexicon/`. The actual Svelte rebuild remains in the worktree.

For the feature audit, the strongest improvements already identified are:

- **Homunculus UI — port.** The Svelte rebuild has a first-class Homunculus interface (`HomunculusView`, form, result display); React `main` does not appear to have an equivalent frontend. 
- **Current-repository navigation — port.** Svelte adds a persistent current-repo indicator, a “Change Repo” action, and disables repo-dependent tools until a repository is selected. 
- **URL/deep-link routing — likely port.** It carries `view`, `project`, document path, and document hash through browser history and handles back/forward navigation. That is useful independently of Svelte. 
-||||Finish it please

9. 20260814T00 CVA Architecture Restart:<<conversation too long; truncated>>::::<<User knowledge memory: Continuity architecture: everything runs in one shared runtime. The CLI is only the first testing/bring-up interface because it is the easiest and fastest way to exercise functionality; CLI is not the target deployment or product model.
>><<conversation too long; truncated>>||||I'm wondering about that, it||||Ok, now, the thing I didn't like from before, was that we also had "episodes" for Insomnia and it seemed weird to me that we had these two different ways to break the conversations up. What was the difference between fragments and episodes?||||And how were episodes determined? Cosign comparison of fragment vectors or something?||||Ok, and you want to implement fragmenting right now?||||That's what I was thinking. Implement the fragmenting||||So... there's very little branching in this test corpus then||||Ok, I'm a little lost. How are we breaking this down... I thought a node *was* a turn...||||So what's the difference between the 1559 and the 1553 from? Just repeated user messages like "Ok"?||||Ok, so far we have CAM turn bodies, indexing, and fragment sorting?||||How are the fragments being built if there's no index to build them from? Are fragments not durable?||||derived index is fine as long as it's fast and cheap on the heap||||In memory maps are not cheap on the heap||||So... this is like the begining of what we were using the "snapshot" for, yeah?||||and we wanted to build version tracking into this too right? Or was that where we got overcomplicated with roots and shit?||||Ok... what I'm most interested in is like being able to "go back" to a certain point, if that makes any sense....
Like, if I was using Continuity and working on a project, and we fucked a bunch of shit up and the context and memories got all polluted with irrelevant or inaccurate information, I could just roll back the CVA and not have to be worried about all of the irrelevant information polluting context.||||So we need another CAM layer to basically have internal Git?||||Alright, and how would implement that?||||I think this is what fucked us last time with atomicity and serialization too. The publication history needing to be updated on every write caused a problem||||we shouldn't even need a global restore point, ever... a restoral request just like...

```

Archive writes ───── A1 A2 A3 A4 A5 A6 ... 
Memories writes ──── M1    M2       M3 ... 
Graph writes ─────── G1             G2 ... 
Vectors writes ───── V1 V2          V3 ...
```

Finds this point    ----------   ^ or this one ^



It could just be like... a version counter I'd think. A simple int64 tracker that increments any time any one of these publication histories change. Or is that what you're talking about...
||||Could floats work for that instead? 70.2, 3.3, 3.4, 9.5, etc.||||and then you can do that infinitely and have 3:5:4:3:3 eventually, in a particularly convoluted situation||||Yeah, so the publication history is just a branching linked-list||||Ok. Write this all down in the new repository in the docs section. While you're at it, document what you've already done there in accordance with our engineering-standards repo||||Alright, what is our plan now then?||||Ok, get to that||||So... append only.... That's not actually going to work for the whole archive, is that what we have?||||but I mean, the archive AS A WHOLE. Individual records need to be append only, but the archive shouldn't be treated as one big record, does that make sense?||||No... I guess it is, kind of but... my thought process here is specifically about concurrent session tracking, or a user reviving an OLD conversation/session for whatever reason they might have||||Ok, so... I think I understand what it's doing now, but explain it to me again, and explain what you want to make it do instead||||So right now the archive thinks every append is part of the same conversation?||||What advantage does that give us over what we currently have? Sounds like it gives us the ability to restore individual conversations so a certain historical state instead of the whole archive.||||So concurrent conversations can get stuck behind a serialized publication lock?||||What kind of overhead is this?||||No, how much storage overhead will separate publications for every conversation be?||||OK. And we can keep the current whole archive tracking by adding another global integer tracker one level down for the archive itself, yeah?||||Do it||||Ok. And we're not writing migration code already are we? Please way we're not||||Good. And you've test loaded the latest version?||||Ok, we're going to a new session. Let's have a context hand-off including the next steps and plan. While you're at it, make sure the documentation is all up to date with what you've done so far.

10. 20260814T10 Audit C:\!bin Folder:||||You've made a rather large mess of the !bin/ folder on C:\ I'd like you to address that. Audit it first. Don't delete anything

11. 20260813T11 Repo Dirty Worktrees:||||Check out our repos. Looking for dirty worktrees, uncommitted, and unpushed work||||Clean it all up, prune every dead worktree and branch you find. Leave Space-Rocks alone||||You've been interupted 3 times here.

12. 20260814T00 CVA Test Corpus Prepared:||||Alright, I'm going to need you to locate the chatGPT import .zip and prepare a test corpus for the CVA. Not the whole thing. Maybe like a dozen conversations to test the machinery.

13. 20260813T17 Project Management Challenges:||||be honest, is this project just beyond you? Every time I turn around we're losing track of what we're doing and making a new mess||||because the codebase shouldn't be that complicated really. Dream is the most complicated part, and its interactions with the CVA are NOT complicated

14. 20260811T11 Fortune 100 vs 500:||||What is Fortune 100 vs Fortune 500?

# User Interaction Metadata

Auto-generated from ChatGPT request activity. Reflects usage patterns, but may be imprecise and not user-provided.

1. User is currently on a ChatGPT Plus plan.

2. User is currently using ChatGPT in a web browser on a desktop computer.

3. User is currently in Canada. This may be inaccurate if, for example, the user is using a VPN.

4. User's local hour is currently 13.

5. User is currently using the following user agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36 Edg/151.0.0.0.

6. User's account is 154 weeks old.

7. User hasn't indicated what they prefer to be called, but the name on their account is Brian Fernstrom.

8. User is active 1 day in the last 1 day, 7 days in the last 7 days, and 8 days in the last 30 days.

9. User's average conversation depth is 22.9.

10. User's average message length is 2101.4.

11. 98% of previous conversations were gpt-5-6-thinking, 0% of previous conversations were gpt-5-6-instant, 0% of previous conversations were gpt-5-5-thinking, 0% of previous conversations were gpt-5.6-sol-wm, 0% of previous conversations were gpt-5-6, 1% of previous conversations were gpt-5-6-auto-thinking.

12. In the last 6179 messages, .

# User Knowledge Memories

Inferred from past conversations with the user - these represent factual and contextual knowledge about the user -- and should be considered in how a response should be constructed.

1. USER COMMUNICATION & WORKFLOW PREFERENCES
- Requests concise, actionable steps; detailed on request.
- “Plan” = high-level steps + goal + verification. “Prompt” = ≤2 min micro task labeled Prompt #; implementation tasks only.
- Avoid terminal commands in prompts; small-file preference: ≤200 lines, never >300.
- Refactors minimal; seams early; “defer mechanics, not ownership.”
- Bash preferred over PowerShell; later used PowerShell for Codex CLI.
- Hermes sub-agents for parallelism only; primary edits via MCP read/write tools.
- Scan exclusions: .workingtrees/ (current) and .worktrees/ (legacy).
- Prefers Docker engine in WSL for local dev; server should avoid dev-only clutter.
- Assistant name preference: selected “Aster” (Aug 02).
- Tools for slides/graphics: uses Google Slides and Clip Studio Paint (CSP); lacks Microsoft Office.
- Image export preferences: wants PNGs with **native alpha channel**; color direction “more purple,” avoid blue/pink drift; later “slightly more pink highlights,” remove unwanted glows.

2. CONTINUITY — CORE STATUS & ARCHITECTURE (Aug 01–13)
Goal & scope
- Model-agnostic memory; deterministic vectorized memories + auto retrieval; curator optional. Encrypted single-file **.cva** snapshot. Postgres retired.
- Pipelines: **Insomnia** (authoritative working memory) and **Dream** (offline builder/graph).
- Embeddings: **qwen3-embedding-8b**, 1024‑dim float16 LE; managed=false.

CVA rebuild (Aug 07–08)
- Content‑addressed packs; per‑pack index; atomic root generations; vacuum/prune/seal.
- Layers: Archive (fragments/vectors/snapshot), Working (memories/graph + vectors), Operational (journal/claims/retries/model version/cursor).
- Snapshots: readers see old/new complete states only.
- Phases: P1 packer/index/root; P2 hardening + concurrency (target 16 workers); P3 implementation.
- Migration: standard open error on incompatible CVAs; separate migration tool.

Insomnia (authoritative WM)
- Episode policy: oldest‑unprocessed; boundaries: explicit completion, inactivity gap, bounded input, topic reset; “remember that” ends episode.
- Persistence: append‑only streams for cursor/claims/retries/processed IDs/model version/outcomes.
- Worker pool 4→16 trial; earlier 139 episodes ≈ 3 h at 4 workers.
- Fixes: durable backlog; provenance rescans; bounded evidence; sync capture + async embedding; atomic episode processing.
- Status (Aug 13): “basically ready for active alpha.”

Dream (offline builder)
- Ledger‑based deterministic selection; cooldown.
- Duplicate‑of linked list (temporal order).
- Graph edges: supersedes + structural parent; planned topical/factual/causal/recurrent/references/duplicate‑of.
- Structured outputs mandated; vector retrieval bug fixed.
- Status (Aug 13): “basically ready for active alpha.”

“Ego” context injection plan
- Start-of-session synthesis + mid‑session small updates; dedup ~½ window.
- Maintained per session; Dream may revise via CVA dump/summary.
- Status (Aug 13): concept only; “barely exists.”

Concurrency & runtime (Aug 09–11)
- Vector worker vs Dream race: root A vs root B; expected **DependencyConflict** repin; intermittent failure path.
- **PendingFragments** CPU/scaling: Archive replay each poll (250 ms); should query `runtimework` queue and point-read.
- Re‑audit: CPU churn mostly gone; replacement‑runtime issues remain.
- Runtime changes: moved heavy runs off OpenRouter; use Codex + Luna low/normal speed.

Benchmark harness (Aug 08–11)
- CVA facade allows head‑of‑line Q+A insertion; 8 stages (~80 Qs each).
- Checkpoints: “836 events”; target **666 Qs** clarified.
- Fix: vectorization before DB; revisions are new records.

3. LONG MEMORY BENCHMARK — PROGRAM & RESULTS (Aug 01–13)
V1 (existing LME)
- 500 recall Qs; retrieval‑only. Tool retrieval success 99.8%; Recall@10 99.03%. Readers: Qwen3.5‑9B, Nemotron‑3 30B, ling‑2.6‑flash, DS v4‑flash.
- Later reruns with **Sol‑high** reader and **DS4‑flash‑0731** query model. Final internal result referenced **~92% judged correct** (earlier “92.6% if gamed”).
- Leaderboard context: Mem0 **~94.6%**; Exabase new; two of top three MSP managed; third DB‑based.

V2 (new benchmark)
- Corpus: user’s multi‑month chats; sanitized.
- Reader baseline: **DS v4‑flash‑0731**.
- Judges: **Qwen3.8 Max** + **Grok 4.5**; tiebreak **Sol 5.6**.
- Method: 8 stages; target **~700** questions; one question per thread; 2–3 follow‑ups allowed.
- Provider instability: Nous → OpenRouter for DS4; Qwen3.8‑Max judge cost ~**$50**.
- Structured schema: retrieve → rank → return structured evidence (handle, conversation_id, source_date, rank, excerpt ≤2,500 chars).
- Regression: compact excerpt bug; recovered to 9/16; notable misses 293/296/323.

Competitor slide (Aug 12–13)
- Current lineup: **Exabase M‑1 — 96.4%**; **Mem0 Platform — 94.8%**; **Continuity — ~92.0%**; **Zep — 90.2%**; **APEX‑MEM — 86.2%**; **EmergenceMem Internal — 86.0%** (later removed). **Hindsight** removed (new **94.6%**).
- Zep correction: **Zep not local/self-hostable**; Graphiti local but ≠ commercial Zep.
- Column renamed to **“No Generalized DB.”**
- Request to replace EmergenceMem with another weak product; not found by Aug 13.
- Slide title: **Competition in a Thriving Market**.
- Investor note: acceptable to not be #1 if tradeoff explained (“~4.4 points for zero vendor lock-in”).

4. BUSINESS / FUNDING — LAUGHING SKULL ENTERPRISES (Aug 04–13)
Company & ownership
- Operating company: **Laughing Skull Enterprises** (BC‑based Canadian corporation). Continuity is a product/brand under LSE.
- Founder owns **100%** today; must retain **50% + 1** final ownership; majority control required.

Budget lines (first‑year working)
- Founder salary **USD $100,000** (loaded ~$105k).
- Senior/staff engineer **CAD $190–200k** base (~USD $150k).
- Legal reserve **USD $55,000** (detail **CAD ~$61,500**). Privacy/compliance **CAD $10,000**.
- Accounting/tax/SR&ED **USD $20,000** (detail **CAD ~$26,000**).
- Insurance **USD $15,000**.
- Equipment **USD $15,000**; founder **CAD ~$4,000**; senior **CAD ~$5,000**; displays **CAD ~$1,000**; Mac test **CAD $1,500–2,000**; misc **CAD ~$1,000**; contingency **CAD ~$1,000**.
- Inference/infra **USD $15,000**; later doubled inference/R&D (raise update).

Other structure
- Licensing: PolyForm Shield.
- Trademarks: Laughing Skull logo; Continuity/Reliquary/Eidolon; **4–6 filings**.
- Non‑dilutive: grants/SR&ED/AI programs; Grant writer planned. Leads: **TELUS #StandWithOwners**, **Zensurance Small Business Grant**.

Governance / compensation policy (Aug 08)
- Employee profit‑sharing intent: **~50/50 profit split** with employees (profit).
- Guardrails to prevent small employee pool > owners; unused employee profit normalized.

Capital strategy
- Round target updated to **USD $1.6M** (earlier $1.25M).
- SAFE: earlier **$15M** post‑money; with $1.6M, **$16M** cap discussed; aim ~10% dilution.
- Pre‑pre‑seed needs **$10–15k** legal; immediate **$25–30k** to start.
- Founder personal: **~$80k** debts; no credit; cannot pay rent; truck asset blocks assistance.

Market model (locked)
- Canonical: **“Continuity TAM/SAM/SOM Re-estimation for Agent-Heavy Professional Developers.”**
- Universe **38.4M** professional developers. TAM **15.36M** agent‑heavy (14.21M–16.90M).
- Exclusions: non‑professional developers; team/enterprise (upside EAM/ESOM).

GTM & pitch deck (Aug 09–13)
- First 10–20 users: LME/AMB release + announcement; website email funnel; selective alpha testers; additional benchmarks.
- Channels: LME leaderboard, AMB; HN/Reddit/Discord considered.
- Theme: “Start small. Scale upwards.” Branching tree; avoid funnel/pyramid.

Slide spine (status)
  1. Title — Continuity logo (purple).
  2. Context thesis — “CONTEXT IS CURRENCY.”
  3. Pain/mess — conspiracy board.
  4. “What you need is…” reveal.
  5. Product — Continuity logo.
  6. Features — sovereign/one file/compact/portable/simplicity.
  7. Architecture — CVA + Insomnia/Dream/Ego icons; zip/conversations/chatbots/agents.
  8. Competition — lineup above; Zep correction; “No Generalized DB.”
  9. ICP — 2×2 failed; list with icons for **discussions**, **pain**, **willingness‑to‑pay**, **reachability** (purple).
 10. Market upside — reinvestment arrow; EAM/ESOM.
 11. GTM tree — Benchmark release; Technical discovery (globe+telescope, no stands; later separate); Website/alpha CTA (webframe + stick person); Validate real use; **Measure conversion rate** (collapsed from longer); **Productization** (purple/pinkish; conveyor end; remove glow); Fan‑out (include **Enterprise** and **Market expansion**).
 12. GTM details — business model omitted; socialist principles omitted.
 13. Competitor comparison — Mem0/Exabase + 3.
 14. Ask — later used; color changes (blue→red→gray).
 15. Team — founder + planned technical team.
 16. Close — requested example.

Deck critiques/notes
- Investor concerns: GTM reads like sequence not engine; need explicit sentence tying ICP to GTM; avoid repeating content; risk of leading with not-#1 benchmark.
- “How it Works” diagram later recognized; deck upgraded to “solid A.”
- Founder asked to avoid “development speed” phrasing.
- Founder slide: wants skills under name; “Systems Architecture” already listed.
- Team slide: wants real job titles; not generic.
- Background visuals: aurora graphics created and used.
- Misc: asked if deck would be $10k; sought critical feedback.

Email/domain setup (Aug 12–13)
- Asked: company domain email via Cloudflare free; registered domain for **Cloudflare Email Routing**; requested next steps.
- Asked about SMTP2GO self‑hostable; Azure notice mentioned.

Branding notes
- Product name undecided: **Continuity / Reliquary / Eidolon**; “Warlock” side project.
- Tagline: “Context is currency.”
- Visual preference: stand‑out but professional; avoid “high school cringe.”

Founder reflections
- Background: “I’m a carpenter… wanted a new career… now trying to raise millions.”
- Expressed hesitancy with sycophancy; asked for real risks/misreads/gaps.

5. SPACE ROCKS — SHORT STATUS (Aug 01–12)
- Repo: https://github.com/Lokee86/space-rocks; web: https://space-rocks.laughingskull.ca/. Godot 4.6.3 + Go 1.26.3 + Rails 8.1.3.
- P3 live; P4 branch for modes/rules; DM/teams spawn policies requested. UDP via Cloudflare + Playit.
- Assets (Aug 11–12): enemy sprites; component sheets; “without active afterburners”; cleanup; repeated “native alpha channel.”

6. WARLOCK TOOLCHAIN — SHORT STATUS
- Grimoire scan (Aug 01): 967 files; 5,896 edges; 30% cyclic; largest cycle 66; score 83.57.
- GUI requests: search bar; uniform controls; sidebar; WASD camera; reset; Esc clears; tooltip; camera center indicator.
