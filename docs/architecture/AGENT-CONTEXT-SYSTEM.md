# Agent Context System: The Five-Layer Plan

> This document describes InDusk's complete agent context system — how agents receive, produce, and share knowledge across tasks and workflow stages. It is the authoritative reference for how context flows through the system.

## Why Context Matters

InDusk's core thesis: humans can't fully explain what they want upfront. They discover requirements through the build process — seeing agent output and correcting course. InDusk structures this by making agent work observable and human corrections persistent.

For this to work, agents need the right context at the right time. An agent with no project knowledge starts from scratch every time. An agent drowning in irrelevant context wastes tokens on noise. The context system is the mechanism that makes the collaboration loop compound — each completed task should make the next one smarter.

## The Five Layers

When a task enters a workflow column with an agent, InDusk assembles the agent's context from five layers:

```
┌─────────────────────────────────────────────┐
│  Layer 5: Knowledge Production              │
│  (Agents told what knowledge to create)     │
├─────────────────────────────────────────────┤
│  Layer 4: Workflow Memory                   │
│  (What happened in prior columns)           │
├─────────────────────────────────────────────┤
│  Layer 3: File/Module Context               │
│  (Per-file knowledge for files being touched)│
├─────────────────────────────────────────────┤
│  Layer 2: Task Context                      │
│  (Knowledge scoped to this specific task)   │
├─────────────────────────────────────────────┤
│  Layer 1: Global Knowledge                  │
│  (Project-wide ADRs, patterns, decisions)   │
└─────────────────────────────────────────────┘
```

### Layer 1: Global Knowledge

**What:** Project-wide artifacts — ADRs, patterns, decisions, dependency info — that any agent on any task should know about.

**Example:** "All API endpoints use a JSON envelope format: `{ data, error, meta }`." Every agent building an endpoint needs this, regardless of which specific endpoint they're working on.

**How it works:** Artifacts with `scope = global` are queried from the database, deduplicated by chain (only latest version), sorted by type priority (ADR > Pattern > IPlan > Decision > Dependency > ChangelogEntry), and packed into 50% of the token budget (4000 of 8000 tokens).

**Established by:** ADR-007 (Context Budget & Relevance System).

**Current status:** Working.

### Layer 2: Task Context

**What:** Artifacts scoped to a specific task — plans, decisions, and notes that only matter for this one piece of work.

**Example:** "This task was rejected once because the error messages weren't user-friendly enough. The reviewer said: use plain English, no error codes." Only the agent retrying this task needs that context.

**How it works:** Artifacts with `scope = task` and matching `source_task_id` are queried, deduplicated, sorted, and packed into 30% of the token budget (2400 tokens). Unused global budget rolls over.

**Established by:** ADR-007.

**Current status:** Working.

### Layer 3: File/Module Context

**What:** Per-file knowledge that gets injected when an agent is working on a specific file. Every file can accumulate its own context over time.

**Example:** An agent modified `src/billing/webhook.rs` and created a module memory: "This file handles Stripe webhooks. It uses signature verification via the `stripe-rust` crate. The retry logic was added in task #42 to handle idempotency." The next agent touching this file gets that context automatically.

**How it works:** Artifacts with `scope = path` are matched against file paths the agent will touch, packed into 20% of the token budget (1600 tokens). Unused budget from higher layers rolls over.

**Established by:** ADR-007.

**Current status:** Mechanism exists in `build_full_context()`, but **disconnected**. The paths parameter is passed as `&[]` in `container.rs` because the system doesn't yet know which files the agent will touch at startup time.

### Layer 4: Workflow Memory

**What:** Rich handoff between workflow columns. When a task moves from Plan → Develop → Review, each agent needs to understand what the previous columns produced and what's expected of them.

**Example:** The Plan agent produces an implementation plan with 5 steps and decides to use WebSockets over polling. The Develop agent needs to receive not just "some commits happened in Plan" but the actual plan content: "implement these 5 steps, we chose WebSockets because of requirement X." The Review agent then needs to know what was planned AND what was actually built, so it can evaluate whether the implementation matches the plan.

This is coordination — like a construction crew where each person needs to know what the others did and what stage the work is at before they can do their part.

**How it works:** `TaskEvent::build_workflow_history()` queries task events (column enters, commits) and builds a chronological log. This is passed as `workflow_history` in the `AgentContext`, separate from the artifact-based context.

**Established by:** Implemented in `container.rs` and `task_event.rs`.

**Current status:** Exists but **too thin**. Currently only includes column names, agent names, and commit messages. Does not include the actual content produced by previous columns (plans, decisions, deliverable output, artifacts created).

### Layer 5: Knowledge Production

**What:** Explicit instructions telling agents to produce knowledge as they work. Without this, artifact creation is accidental — agents only create knowledge when they happen to or when humans ask. With it, knowledge production becomes a deliberate part of every workflow stage.

**Example:** A Development column has a knowledge prompt: "As you work, capture patterns you establish as `pattern` artifacts and file-level knowledge as `module_memory` artifacts. Prefer creating knowledge over not — artifacts that aren't useful can be cleaned up, but knowledge that's never captured is lost."

Different columns produce different types of knowledge:
- **Plan** → ADRs, implementation plans
- **Develop** → Patterns, module memories
- **Review** → Patterns observed, quality decisions
- **Test** → Probably nothing (not every stage should produce knowledge)

**How it works:** A `knowledge_prompt` field on each column is appended to the agent's start command. The prompt instructs the agent on what types of artifacts to create and how (via `.vibe/decision.json` or the `create_artifact` MCP tool).

**Established by:** ADR-009 (Agent Self-Improvement Protocol & Conditional Artifact Gating).

**Current status:** Not yet implemented. Designed, with implementation plan in `docs/impl/009`.

## How Layers Interact

The layers aren't independent — they form a cycle:

```
Task enters column
       │
       ▼
┌──────────────┐
│ Assemble     │◄── L1: Global knowledge (filtered by relevance)
│ Agent        │◄── L2: Task-specific context
│ Context      │◄── L3: File/module context for touched files
│              │◄── L4: Workflow memory (what prior columns did)
│              │◄── L5: Knowledge production instructions
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Agent Works  │──► Creates artifacts (feeding L1, L2, L3)
│              │──► Makes commits (feeding L4)
│              │──► Writes decision.json (routing + artifacts)
└──────┬───────┘
       │
       ▼
  Task transitions to next column
       │
       ▼
  Context assembled again (now richer)
```

Each column's agent both **consumes** context (L1-L4) and **produces** context (guided by L5). The next column's agent gets everything the previous one started with, plus whatever new knowledge was created. This is the compounding effect.

## Roadmap

### Next Step: ADR-009 — Knowledge Production & Smart Filtering

**What it adds:**
- **Layer 5** — `knowledge_prompt` field on columns. Agents are explicitly told to produce knowledge, with guidance tailored to their workflow stage.
- **Layer 1 improvement** — Conditional gating via `metadata.conditions` on artifacts. Artifacts can specify "only inject me for tasks with label X" or "only when the task mentions keyword Y." As the knowledge base grows, irrelevant artifacts stop consuming token budget.

**What remains after ADR-009:**
- Layer 3 is still disconnected (paths not wired up)
- Layer 4 is still thin (just commit messages)

### After ADR-009: Layer 3 — Wire Up File/Module Context

**Problem:** `build_full_context()` accepts a `paths` parameter but `container.rs` passes `&[]`. Module memories exist in the database but never get injected based on file matching.

**Approach:**
1. When a task enters a column, determine which files it will likely touch. Sources:
   - Files mentioned in the task description
   - Files changed in the workspace's git branch (for retry/continuation scenarios)
   - Files listed in the task's implementation plan (if one exists as an artifact)
2. Pass these paths to `build_full_context()` so path-scoped module memories get included
3. When an agent completes, auto-extract which files were modified and encourage (via L5 knowledge prompts) creation of module memories for significant changes

**Impact:** Agents working on `src/billing/` automatically receive accumulated knowledge about billing modules. Agents working on `src/auth/` get auth knowledge. File-level context becomes a living documentation layer that agents both read and write.

### After ADR-009: Layer 4 — Rich Workflow Memory

**Problem:** `build_workflow_history()` only captures column names and commit messages. The Develop agent doesn't know what the Plan agent actually decided. The Review agent doesn't know what was planned vs. what was built.

**Approach:**
1. When an agent completes a column, capture a **column summary** — not just commits, but:
   - The deliverable that was produced (e.g., the implementation plan content)
   - The decision that was made (from `.vibe/decision.json`)
   - Artifacts created during this column's execution
   - Any structured output the column's deliverable requested
2. Store this as a richer event type (or expand the existing `TaskEvent` metadata)
3. `build_workflow_history()` assembles these summaries into a narrative: "In the Plan column, the Planning Agent produced this plan: [content]. It decided to use approach X. It created these artifacts: [list]."
4. The next column's agent receives this as actionable instructions, not a commit log

**Impact:** Multi-column workflows become genuine pipelines where each stage builds on the previous one's work. The Plan → Develop handoff goes from "here are some commit messages" to "here's the plan, execute steps 3-7, we chose WebSockets because of requirement X."

## Measuring Progress

The test scenario harness (ADR-010) provides a way to measure whether these layers are working:

1. **Baseline** (current): Run scenarios, measure what context each task receives
2. **After ADR-009**: Re-run, verify that knowledge prompts cause more artifact creation and gating reduces irrelevant injection
3. **After L3 wiring**: Verify that file-scoped context appears for relevant tasks
4. **After L4 enrichment**: Verify that workflow memory includes actual content, not just commit messages

Each improvement should show measurably richer context in the scenario checkpoints.
