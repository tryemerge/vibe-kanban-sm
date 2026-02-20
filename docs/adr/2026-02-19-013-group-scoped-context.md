# ADR 013: Group-Scoped Context & Knowledge Inheritance

**Date:** 2026-02-19
**Status:** Proposed
**Author:** User + Claude
**Supersedes:** ADR-009 (Agent Self-Improvement Protocol & Conditional Artifact Gating)
**Depends on:** ADR-012 (Task Groups), ADR-007 (Context Budget & Relevance)
**Inspired by:** OpenClaw/Pi self-extension model, OpenClaw Foundry's pattern crystallization, pskoett/self-improving-agent promotion workflow

## Context

ADR-009 identified two gaps in InDusk's context compounding system:
1. Agents don't know they should produce knowledge (solved: column-level `knowledge_prompt`)
2. Artifacts have no relevance conditions beyond scope (proposed: metadata-based conditional gating with keyword/label matching)

ADR-009's conditional gating was designed to solve a specific problem: as the knowledge base grows, global artifacts compete for token budget with no way to say "this pattern is only relevant for billing tasks." The proposed solution was heuristic — keyword matching, label conditions, substring checks in `metadata.conditions`.

**ADR-012 changes the landscape.** Task groups introduce a structural boundary that handles the common case better than heuristics:

- A billing team's patterns naturally scope to the "billing" task group
- An auth team's decisions scope to the "auth" task group
- Neither pollutes the other's token budget
- The inter-group dependency DAG defines which groups inherit from upstream groups

This makes conditional gating a secondary mechanism (useful for cross-cutting concerns) rather than the primary solution. The primary solution is **structural**: group scope + DAG inheritance.

### What we learned from OpenClaw (Feb 2026 update)

OpenClaw/Pi has evolved since ADR-009 was written:

**Confirmed patterns:**
- **Self-extension is a first-class expectation.** Pi ships with only 4 built-in tools; everything else is agent-authored SKILL.md files. The bias toward creating knowledge remains core. InDusk's `knowledge_prompt` aligns with this.
- **Progressive disclosure saves tokens.** OpenClaw loads only skill names (~24 tokens each) into context initially; full skill content injects only when activated. InDusk should do the same with group-scoped artifacts — metadata visible, content injected only when the group/task is relevant.
- **Self-improving-agent pattern.** The community skill (`pskoett/self-improving-agent`) uses a promotion workflow: LEARNINGS.md → graduated to permanent memory based on frequency and success rate. OpenClaw Foundry crystallizes patterns after "5+ uses with 70%+ success rate." InDusk can learn from this: artifacts should have a lifecycle (draft → validated → promoted).

**Gaps OpenClaw hasn't solved:**
- **No project-level scoping.** Issue #13676 is an open feature request for first-class projects. Knowledge is organized by filesystem convention (workspace > local > bundled), not explicit scopes. Sessions model conversation state, not work organization.
- **No task dependency or orchestration.** Pi's DAG is for conversation branching, not work ordering. There's no concept of "task B depends on task A" or "these tasks form a sequential group."
- **No knowledge inheritance along execution paths.** Skills are either available or not, based on filesystem location and metadata gates. There's no way to say "this skill was produced by work stream A and should flow to work stream B because B depends on A."

InDusk's task groups + DAG + scoped artifacts address all three gaps. This is genuinely novel — no agent tool in the industry connects knowledge flow to task execution flow.

## Decision

### Part 1: `group` artifact scope

Add `group` as a new scope level for context artifacts. The scope hierarchy becomes:

| Scope | Injected for | Use case |
|-------|-------------|----------|
| `global` | All tasks in the project | Cross-cutting standards, project conventions |
| `group` | Tasks in the same task group | Domain-specific patterns, group decisions |
| `task` | Only the specific task | Task-specific notes, intermediate work |
| `path` | Tasks touching matching files | Module-specific knowledge |

When an agent creates an artifact with `scope: "group"`, the artifact is associated with the task's group via `source_task_id → task.task_group_id`. During `build_full_context()`, group-scoped artifacts are included only for tasks belonging to that group (or inheriting from it via the DAG — see Part 2).

The `ContextArtifact` model already has `source_task_id`. No schema change needed for storage — the scope value `"group"` is sufficient. The context builder resolves the group by looking up `task.task_group_id` for the source task.

### Part 2: Context inheritance via the DAG

When Group B depends on Group A in the inter-group dependency DAG, Group B's tasks inherit Group A's group-scoped artifacts. This creates a knowledge flow that follows the execution path:

```
Group A (auth model) creates:
  - pattern: "User model conventions" (scope: group)
  - decision: "Why bcrypt over argon2" (scope: group)

Group B (auth API) depends on Group A:
  → Group B's tasks receive Group A's group artifacts + Group B's own group artifacts

Group C (auth UI) depends on Group B:
  → Group C's tasks receive Group A's + Group B's + Group C's group artifacts
```

**Inheritance is transitive along the DAG.** If C depends on B which depends on A, C inherits from both A and B. This mirrors how code builds: the UI layer needs to know about the model and API decisions that came before it.

**Implementation in `build_full_context()`:**
1. Look up the task's `task_group_id`
2. If the task is in a group, query `task_group_dependencies` to find all upstream groups (recursive walk up the DAG)
3. Include group-scoped artifacts from the task's own group + all upstream groups
4. Apply token budget allocation: global → group (own + inherited) → task → path

**Budget allocation update (ADR-007):**
The existing budget splits global/task/path. Add group to the split:
- Global: 30% (was 40%)
- Group (own + inherited): 30% (new)
- Task: 20% (was 30%)
- Path: 20% (was 30%)

These are defaults; the existing per-project budget configuration applies.

### Part 3: Knowledge production prompt (preserved from ADR-009)

Column-level `knowledge_prompt` remains the right design. ADR-009's decision here is unchanged — agents need explicit instructions to produce knowledge, and the right granularity is per workflow stage.

**Enhancement for groups:** Add an optional `knowledge_prompt` field to `task_groups`. When set, this prompt is appended alongside the column's `knowledge_prompt`, providing group-specific guidance:

```
Column knowledge_prompt: "Create patterns and module memories as you work..."
Group knowledge_prompt: "This group focuses on the billing domain. Tag artifacts with scope: group."
```

The group prompt gives agents domain context for their knowledge production. Without it, a dev column's generic prompt produces generic artifacts. With it, the billing group's dev tasks produce billing-specific artifacts scoped to the group.

**Default behavior:** If no group prompt is set, agents use only the column prompt (backward compatible with ADR-009). If neither is set, no knowledge production instruction is given (also backward compatible).

### Part 4: Conditional gating (simplified from ADR-009)

ADR-009 proposed a full condition system in `metadata.conditions` with label matching, path prefixes, keyword checks, and artifact type filters. With group scope handling the primary use case (domain-specific knowledge staying in its domain), conditional gating is simplified:

**Retained conditions:**
- `paths` — still valuable for path-scoped artifacts that need more specificity
- `keywords_in_task` — useful for truly cross-cutting concerns (e.g., "only inject this Docker pattern when the task mentions deployment")

**Removed conditions:**
- `labels` — labels are for tagging/filtering, not context gating. Group scope handles domain separation.
- `artifact_types_present` — too niche, never implemented, unclear value

**Evaluation rules** (unchanged from ADR-009):
- If `metadata.conditions` is absent or empty → always include
- If multiple condition types are present → ALL must match (AND)
- Within a condition type → ANY value matches (OR)

### Part 5: Artifact lifecycle (new, inspired by OpenClaw Foundry)

OpenClaw Foundry crystallizes patterns after "5+ uses with 70%+ success rate." The `pskoett/self-improving-agent` skill promotes learnings from ephemeral logs to permanent memory.

InDusk should support **scope promotion** — an artifact can be promoted up the scope hierarchy as it proves useful:

```
task → group → global
```

A pattern discovered during a specific task starts as task-scoped. If it proves useful across the group, promote to group scope. If it's universally valuable, promote to global.

**Implementation:** This is a simple `scope` update on the artifact record. The Knowledge Base settings page (ADR-008) already shows artifacts with their scopes. Add a "Promote" action that changes the scope one level up.

**Future: automatic promotion.** Track artifact injection frequency and correlation with agent success. When a task-scoped artifact is manually copied to 3+ tasks, suggest promotion to group scope. When a group-scoped artifact is inherited by 3+ downstream groups, suggest promotion to global. This is future work — v1 is manual promotion.

### Part 6: MCP tool update

Update the `create_artifact` MCP tool:

```json
{
  "artifact_type": "pattern",
  "title": "Stripe Webhook Validation",
  "content": "...",
  "scope": "group",
  "conditions": {
    "keywords_in_task": ["webhook", "stripe"]
  }
}
```

The `scope: "group"` value scopes the artifact to the creating task's group. If the task has no group, fall back to `global` scope with a warning.

## Consequences

### Positive
- Group scope eliminates the most common context pollution problem (domain A's patterns bloating domain B's context) through structure rather than heuristics
- DAG-based inheritance creates a knowledge flow that mirrors the execution flow — downstream groups automatically receive upstream knowledge
- Progressive disclosure: group-scoped artifacts only load for relevant groups, saving token budget
- Scope promotion gives artifacts a natural lifecycle from local discovery to project-wide convention
- Conditional gating is simpler (fewer condition types) because groups handle the primary use case
- Knowledge production prompts per group + per column give agents precise guidance for what knowledge to produce and how to scope it
- Builds on OpenClaw's proven patterns (self-extension, progressive disclosure, promotion) while solving gaps they haven't addressed (project scoping, execution-path knowledge flow)

### Negative
- DAG traversal for inherited artifacts adds complexity to `build_full_context()` (mitigated: the DAG is typically small, and results can be cached per group)
- Budget allocation with four scope levels is harder to tune than three
- Group prompt + column prompt stacking could produce verbose agent instructions (mitigated: both are optional)
- Scope promotion is manual in v1 — automatic promotion requires usage tracking infrastructure

### Future Work
- **Automatic scope promotion**: Track artifact usage and suggest promotions based on frequency/success
- **Artifact versioning with group context**: When an artifact is updated, tag the version with the group/task that updated it
- **Cross-project knowledge sharing**: Export group-scoped artifacts as portable skill bundles (inspired by ClawHub)
- **Vector search for artifact discovery**: Embeddings-based relevance (like OpenClaw's memory search) alongside structured scope matching
- **Context snapshots for frozen groups**: When a group's `started_at` is set, snapshot the available context so running groups aren't affected by new artifacts from other groups

## Alternatives Considered

1. **Keep ADR-009's full conditional gating without group scope.** Keywords and labels can approximate domain boundaries, but they're heuristic — they miss relevant artifacts and include irrelevant ones. Group scope is structural and deterministic. Conditions remain as a supplementary mechanism for edge cases.

2. **Automatic scope inference (no explicit group scope).** Infer that an artifact created by a task in a group should be visible to other group tasks. This is implicit and fragile — agents couldn't explicitly choose to make an artifact global vs group-scoped. Explicit scope selection is clearer.

3. **Separate knowledge base per group.** Instead of a scope on the shared artifact table, create distinct storage per group. This prevents inheritance and makes promotion harder. A unified artifact table with scope filtering is more flexible.

4. **Embedding-based relevance instead of scope hierarchy.** Use vector similarity between task and artifact content to determine inclusion. More accurate for edge cases but adds infrastructure complexity (vector DB), is non-deterministic, and doesn't align with the execution DAG. Deferred to future work alongside scope hierarchy (not as a replacement).

## Related
- ADR-009: Agent Self-Improvement Protocol (superseded — knowledge production prompts preserved, conditional gating simplified, group scope added)
- ADR-012: Task Groups (introduces groups, DAG, immutability — the structural foundation for group-scoped context)
- ADR-007: Context Budget & Relevance (budget allocation updated to include group scope)
- ADR-008: Task Lifecycle Observability (Knowledge Base page for artifact management and scope promotion UI)
- OpenClaw Issue #13676: First-class Projects (confirms the need for project-level knowledge scoping — InDusk already has this)
