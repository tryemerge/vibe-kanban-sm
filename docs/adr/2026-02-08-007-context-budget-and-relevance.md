# ADR 007: Context Budget & Relevance System

**Date:** 2026-02-08
**Status:** Accepted
**Author:** User + Claude

## Context

InDusk stores knowledge extracted from agent work as **context artifacts** in the database (ADRs, patterns, module memories, decisions, etc.). These artifacts are injected into agent prompts when tasks start, with the goal of compounding intelligence across tasks.

The current implementation has several problems that will prevent this from scaling:

1. **No token budget.** `build_full_context()` dumps all matching artifacts with no size limit. As artifacts accumulate, the injected context will overflow the agent's useful context window.

2. **Two disconnected injection paths.** `container.rs` uses its own `build_project_context()` (5 ADRs + 5 patterns only), while the richer `build_full_context()` (global + task + path scopes) exists in the model layer but is never called during agent startup.

3. **No chain deduplication.** When an artifact has versions 1, 2, 3 via `chain_id`, all three could be returned. Only the latest version should be injected.

4. **No relevance ordering.** Artifacts are sorted by `updated_at DESC`. Recency is a weak proxy for relevance — an ADR about database choice is always relevant; a transient decision from yesterday may not be.

5. **No size awareness.** There's no way to know how much context window an artifact will consume without reading its full content.

## Decision

Implement a **token-budgeted context builder** that:

1. **Estimates token cost** per artifact (stored as `token_estimate` column, computed on create/update)
2. **Prioritizes by type** with a fixed ordering: ADR > Pattern > IPlan > ModuleMemory > Decision > Dependency > ChangelogEntry
3. **Deduplicates by chain** — only the latest version per `chain_id` is included
4. **Enforces a configurable token budget** (default: 8000 tokens) across all three scopes
5. **Allocates budget across scopes**: Global gets 50%, Task gets 30%, Path gets 20%
6. **Unifies the injection path** — `container.rs` calls `build_full_context()` instead of its own `build_project_context()`

### Token Estimation

We use a simple heuristic: `token_estimate = content.len() / 4` (roughly 4 chars per token for English text). This is computed on artifact create/update and stored in the DB, avoiding repeated computation during context assembly.

### Priority Ordering

Within each scope, artifacts are ordered by:
1. Type priority (ADR=1, Pattern=2, IPlan=3, ModuleMemory=4, Decision=5, Dependency=6, ChangelogEntry=7)
2. Recency (`updated_at DESC`)

The builder fills the budget greedily: iterate in priority order, include each artifact if it fits in the remaining budget, skip if it doesn't.

### Scope Budget Allocation

For an 8000-token budget:
- **Global**: 4000 tokens (ADRs, patterns — the most important context)
- **Task**: 2400 tokens (task-specific plans, decisions)
- **Path**: 1600 tokens (module memories for files being touched)

If a scope doesn't use its full budget, the remainder rolls over to the next scope.

## Consequences

### Positive
- Context injection is bounded — agents always get a useful amount of context, never overwhelming
- Higher-value artifacts (ADRs, patterns) are prioritized over lower-value ones (changelog entries)
- Chain deduplication prevents redundant information
- Single injection path simplifies the codebase and ensures all scope types are used
- Token estimates enable future UI features (showing budget usage, warning on large artifacts)

### Negative
- Token estimation is approximate (char/4 heuristic)
- Fixed type priorities may not match all use cases (a critical decision could be more important than a generic pattern)
- Budget allocation percentages are hardcoded initially

### Future Work
- **Relevance scoring** via feedback signals (agent outcomes validate/invalidate artifacts)
- **Dynamic budget allocation** based on task type or column configuration
- **Staleness detection** — deprioritize artifacts that haven't been updated in a long time
- **Configurable type priorities** per project or board

## Alternatives Considered

1. **No budget, just truncate**: Simpler but loses important context that happens to be at the end
2. **Summarization**: Run artifacts through an LLM to compress — too expensive at injection time
3. **Embedding-based relevance**: Store vector embeddings, do similarity search against task description — adds complexity (vector DB), deferred to future work
