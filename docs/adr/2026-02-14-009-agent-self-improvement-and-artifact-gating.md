# ADR 009: Agent Self-Improvement Protocol & Conditional Artifact Gating

**Date:** 2026-02-14
**Status:** Superseded by ADR-013 (Group-Scoped Context & Knowledge Inheritance)
**Author:** User + Claude
**Inspired by:** OpenClaw/Pi agent self-extension model

## Context

InDusk has a working context compounding system (ADR-007): agents *can* create artifacts via `.vibe/decision.json` or the `create_artifact` MCP tool, and those artifacts get injected into future agent prompts via `build_full_context()`.

But there are two gaps:

### 1. Agents don't know they should produce knowledge

Nothing in the agent's assembled context tells it "you should create artifacts when you learn something reusable." Artifact creation happens when agents stumble into it or when a human explicitly asks. This makes knowledge compounding an accidental side effect rather than a deliberate behavior.

OpenClaw/Pi takes the opposite stance: self-extension is a first-class expectation. If an agent learns something, it's *supposed to* write it down. Pi's author describes throwing away skills that aren't useful — the bias is toward creating knowledge, not toward caution.

For InDusk, this means adding an explicit **knowledge production prompt** to the agent context assembly. Not every workflow stage should produce knowledge (a test runner shouldn't be writing ADRs), so this should be configurable per column.

### 2. Artifacts have no relevance conditions beyond scope

The current injection logic is: scope match (global/task/path) → type priority → token budget. This works when the knowledge base is small, but as it grows, global artifacts will start competing for budget space with no way to say "this artifact is only relevant when the task involves database work" or "skip this pattern if the project doesn't use React."

OpenClaw handles this with `metadata` gating on skills — `requires.bins`, `requires.env`, `os` filters. Skills that don't match the current environment aren't loaded at all, saving token budget for relevant ones.

For InDusk, we can use the existing `metadata` JSON field on `ContextArtifact` (which exists in the schema but is currently unused during injection) to store filtering conditions. The `build_full_context()` function would check these conditions before including an artifact in the budget.

## Decision

### Part 1: Knowledge Production Prompt

Add a `knowledge_prompt` text field to `kanban_columns`. When set, this prompt is appended to the agent's start command, instructing the agent to produce knowledge artifacts during or after its work.

**Column-level configuration** because:
- A Research column should produce ADRs and implementation plans
- A Development column should produce patterns and module memories
- A Testing column probably shouldn't produce knowledge at all
- A Review column might produce patterns it observes

**Example `knowledge_prompt` for a Development column:**

```
## Knowledge Production

As you work, look for opportunities to capture reusable knowledge:

- **Patterns**: If you establish a code pattern that future tasks should follow, create a `pattern` artifact
- **Module memories**: If you build or modify a significant module, create a `module_memory` artifact describing what it does and key decisions
- **Decisions**: If you make a non-obvious technical choice, create a `decision` artifact explaining why

To create an artifact, include it in your `.vibe/decision.json`:
```json
{
  "decision": "your_routing_decision",
  "artifact_type": "pattern",
  "title": "Descriptive Title",
  "content": "Detailed explanation...",
  "scope": "global"
}
```

Or use the `create_artifact` MCP tool at any point during your work.

Prefer creating knowledge over not — artifacts that turn out to be unhelpful can be cleaned up, but knowledge that's never captured is lost forever.
```

**Default templates** provide starter knowledge prompts per column type (research → ADRs/iplans, development → patterns/module_memory, review → patterns), but users can customize or disable them.

### Part 2: Conditional Artifact Gating

Use the existing `metadata` JSON field on `ContextArtifact` to store injection conditions. During `build_full_context()`, check conditions before including an artifact in the budget.

**Condition format** (stored in `metadata.conditions`):

```json
{
  "conditions": {
    "labels": ["billing", "stripe"],
    "paths": ["src/billing/", "src/payments/"],
    "artifact_types_present": ["adr"],
    "keywords_in_task": ["payment", "invoice", "subscription"]
  }
}
```

**Condition semantics:**

| Condition | Meaning | Check Against |
|-----------|---------|---------------|
| `labels` | Task must have at least one of these labels | Task's assigned labels |
| `paths` | Task must touch files matching these prefixes | Task context files / workspace paths |
| `keywords_in_task` | Task title or description must contain at least one keyword | Task title + description text |
| `artifact_types_present` | Only include if the project already has artifacts of these types | Project's existing artifact types |

**Evaluation rules:**
- If `metadata.conditions` is absent or empty → always include (backward compatible)
- If multiple condition types are present → ALL must match (AND logic)
- Within a condition type → ANY value matches (OR logic)
- Example: `{"labels": ["billing"], "keywords_in_task": ["stripe"]}` means "task must have the billing label AND mention stripe"

**Why this approach:**
- Uses the existing `metadata` column — no schema migration needed for storage
- Conditions are checked in Rust during `build_full_context()` — no extra DB queries beyond what we already do (we already load all artifacts for the project)
- Labels and task info are cheap to pass into the context builder
- Agents can set conditions when creating artifacts via MCP or decision.json, making the gating self-organizing

### Part 3: MCP Tool Enhancement

Update the `create_artifact` MCP tool to accept an optional `conditions` parameter:

```json
{
  "artifact_type": "pattern",
  "title": "Stripe Webhook Pattern",
  "content": "...",
  "scope": "global",
  "conditions": {
    "labels": ["billing"],
    "keywords_in_task": ["stripe", "webhook", "payment"]
  }
}
```

This lets agents self-organize: when an agent creates a pattern for billing work, it can tag it so only future billing tasks receive it. The knowledge base grows without drowning unrelated tasks in irrelevant context.

## Consequences

### Positive
- Agents are explicitly told to produce knowledge, making compounding deliberate rather than accidental
- Knowledge production is configurable per workflow stage — research produces ADRs, development produces patterns, testing produces nothing
- Conditional gating reduces token waste — billing patterns don't consume budget on auth tasks
- Self-organizing: agents can set their own conditions on artifacts they create
- Backward compatible: no conditions = always include (existing behavior preserved)
- No new tables or complex migrations — uses existing `metadata` field and adds one column

### Negative
- Knowledge prompt adds tokens to every agent start (mitigated: only when configured on the column)
- Agents may produce low-quality artifacts (mitigated: knowledge base cleanup via ADR-008's Knowledge Base page)
- Condition matching adds computation to `build_full_context()` (mitigated: conditions are cheap string checks, no DB queries)
- Keyword matching is naive (substring match, not semantic) — may miss relevant artifacts or include irrelevant ones

### Future Work
- **Knowledge prompt templates**: Pre-built prompts for common column types (research, dev, review) — selectable in column settings UI
- **Semantic condition matching**: Use embeddings for task↔artifact relevance instead of keyword substring
- **Artifact quality signals**: Track whether injected artifacts correlated with agent success (ADR-007 future work)
- **Cross-project artifact sharing**: Export/import artifacts between InDusk projects (inspired by OpenClaw's ClawHub registry)
- **`.vibe/artifacts/` directory**: Allow artifacts as files in the repo (version-controlled, portable) alongside DB storage

## Alternatives Considered

1. **Global knowledge prompt in agent system prompt**: Simpler but one-size-fits-all. A research agent needs different knowledge production guidance than a dev agent. Column-level gives the right granularity.

2. **Tag-based artifact filtering**: Add a separate `tags` table for artifacts instead of using `metadata.conditions`. More normalized but adds schema complexity for what is essentially a filter hint. Tags can be added later if conditions prove insufficient.

3. **Mandatory artifact production**: Require every column with an agent to produce at least one artifact. Too aggressive — testing columns and simple tasks shouldn't be forced to produce knowledge. The prompt-based approach encourages without mandating.

4. **Embedding-based relevance**: Use vector similarity between task description and artifact content. More accurate than keyword matching but adds infrastructure (vector DB or in-process embeddings). Deferred to future work — keyword conditions are good enough for v1 and don't require new dependencies.
