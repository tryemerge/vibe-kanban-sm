# ADR 014: Task Group Lifecycle & Orchestration Observability

**Date:** 2026-02-19
**Status:** Proposed
**Author:** User + Claude
**Depends on:** ADR-012 (Task Groups), ADR-013 (Group-Scoped Context), ADR-008 (Task Lifecycle Observability)

## Context

ADR-012 introduced task groups — project-scoped groupings of related tasks with sequential execution via auto-dependencies and immutability once started. ADR-013 added group-scoped context and DAG-based knowledge inheritance. Together they give InDusk the structural foundation for managing related work.

But structure isn't orchestration. Today, task groups are passive containers: a user creates a group, manually adds tasks, manually arranges order, and the system blindly executes in sequence. Three problems emerge:

### 1. No analysis phase

When you queue 5 tasks in a group, you're making assumptions: you've guessed the right order, you haven't missed any steps, and nothing in the group conflicts with parallel execution. These are judgment calls that an agent could make — and should make, because agents will see issues a human skimming task titles won't catch. "Add auth API" and "add auth middleware" might be better done concurrently on the same worktree. "Add auth UI" might need a step the user forgot: "add auth types/interfaces."

No tool in the industry has an **analysis step** where an orchestrator reviews the task group before execution begins.

### 2. No lifecycle for groups themselves

ADR-012's `started_at` timestamp is binary: null (mutable) or set (frozen). There's no concept of a group progressing through stages — being planned, being analyzed, ready for execution, actively executing, or completed. The group doesn't have its own workflow; it's just a bag of tasks that happens to be frozen once the first task runs.

Without a group lifecycle, there's no place for the analysis step to live, no way to represent "this group is waiting on another group to finish," and no way for backlog groups to automatically enter the pipeline when their dependencies clear.

### 3. Observability is absent at the orchestration level

ADR-008 added task-level observability: event timelines, context previews, artifact browsers. This lets you see what happened to an individual task. But task groups operate **above** the task level — they're the orchestration layer. And that layer is invisible.

When a group enters analysis, what did the planner decide? When it reordered tasks or identified gaps, why? When a running agent discovers out-of-scope work and creates a backlog group, that decision is buried in agent terminal output. When a backlog group's dependencies clear and it auto-promotes to analysis, the user has no idea it happened.

**Observability at the orchestration level is the core of this application.** It's what makes the difference between "a bunch of agents running tasks" and "a system I trust to manage work on my behalf." It serves two purposes:

1. **Developing and improving the system.** Seeing orchestration decisions (analysis outcomes, DAG construction, backlog promotion, dependency resolution) lets you tune the workflow — tighten prompts, adjust analysis criteria, identify recurring patterns of waste.

2. **User trust without micromanagement.** Seeing "what's going to happen next" and "what just happened and why" lets a user step away from the terminal. They don't need to watch every agent's every thought. They need to see: developer sent tasks to tester, tester evaluated and decided no test needed, group analysis identified a missing migration step and added it, backlog group promoted because auth group completed.

This is fundamentally different from terminal output. Terminal output is the agent's stream of consciousness — every file read, every compilation error, every retry. Orchestration observability is the **decision layer**: state changes, routing decisions, gap analysis results, dependency resolutions, and their justifications.

## Decision

### 1. Task group lifecycle states

Task groups transition through a defined lifecycle:

```
draft → analyzing → ready → executing → done
                                ↘
                              failed
```

| State | Meaning |
|-------|---------|
| `draft` | Group is being planned. Tasks can be added, removed, reordered. Mutable. |
| `analyzing` | A planner agent is reviewing the group's tasks, building the internal DAG, identifying gaps. Immutable. |
| `ready` | Analysis complete, inter-group dependencies satisfied. Waiting to start execution. |
| `executing` | Tasks are actively being worked on by agents. Immutable. |
| `done` | All tasks in terminal columns. Group's outgoing dependencies are satisfied. |
| `failed` | Analysis or execution encountered an unrecoverable error. Requires user intervention. |

**Schema change** — replace the boolean `started_at` approach with an explicit state column:

```sql
ALTER TABLE task_groups ADD COLUMN state TEXT NOT NULL DEFAULT 'draft';
-- Allowed values: 'draft', 'analyzing', 'ready', 'executing', 'done', 'failed'
-- Keep started_at for backward compat / timestamp of first execution
```

**Transition rules:**
- `draft → analyzing`: Triggered when user marks group ready, or automatically when the group has tasks and no unsatisfied inter-group dependencies
- `analyzing → ready`: Set by the analysis agent upon successful completion
- `analyzing → failed`: Set if analysis encounters an error
- `ready → executing`: System starts the first eligible task(s) per the internal DAG
- `executing → done`: All tasks reach terminal columns
- `executing → failed`: A task fails beyond retry/escalation limits
- `failed → draft`: User resets the group for re-planning

### 2. Auto-created analysis task

When a group transitions to `analyzing`, the system automatically creates a **planner task** within the group:

- Title: `[Analysis] {group_name}`
- Assigned to a planner/orchestrator agent (a new agent type or a configurable column with a planner agent)
- Task receives the group's full context: all task titles, descriptions, labels, and any existing group-scoped artifacts

**The analysis task's job:**

1. **Review all tasks in the group.** Do they make sense together? Are they in the right order? Are any missing?
2. **Identify gaps.** If "Add auth API" and "Add auth UI" are present but "Add auth types" is missing, create it.
3. **Move out-of-scope work.** If a task doesn't belong in this group (e.g., "Update CI pipeline" mixed into an auth group), move it out to a backlog group.
4. **Build the internal execution DAG.** Determine which tasks can run in parallel vs. which must be sequential. Write this as a structured deliverable (JSON DAG in `.vibe/decision.json`).
5. **Produce a group-scoped artifact** summarizing the analysis: what was changed, what the execution plan is, and why.

**The structured deliverable format:**

```json
{
  "analysis_outcome": "ready",
  "execution_dag": {
    "parallel_groups": [
      ["task-id-1", "task-id-2"],
      ["task-id-3"],
      ["task-id-4", "task-id-5"]
    ]
  },
  "tasks_added": ["task-id-new"],
  "tasks_moved_to_backlog": ["task-id-moved"],
  "rationale": "Added auth types task as prerequisite. Moved CI task to infrastructure backlog. Tasks 1 and 2 can run in parallel (no file overlap)."
}
```

Where `parallel_groups` is an array of arrays — each inner array is a set of tasks that can execute concurrently, and the outer array defines the sequential order of these parallel sets.

### 3. Internal execution DAG within groups

ADR-012 defined sequential execution via auto-dependencies (task N depends on task N-1). The analysis step replaces this with a richer **internal DAG** that supports both parallel and sequential execution within a single group:

```
Group: "Auth Feature"
  ├─ [parallel] Add auth types, Add auth config
  ├─ [sequential] Add auth API (depends on types + config)
  ├─ [parallel] Add auth middleware, Add auth UI (both depend on API)
  └─ [sequential] Add auth integration tests (depends on middleware + UI)
```

**Implementation:** The DAG is stored as the analysis task's structured deliverable. The execution engine reads the DAG to determine which tasks to start next:

- Start all tasks in the first parallel set
- When all tasks in a set complete, start the next set
- If a task fails, pause downstream sets and surface the failure

The existing `is_auto_group` dependency chain from ADR-012 is **replaced** by the DAG-driven execution. Auto-dependencies are still created (for the dependency UI to work), but the DAG is the source of truth for execution order.

### 4. Backlog groups and auto-promotion

During execution, agents may discover work that doesn't belong in the current group. Rather than silently adding tasks or ignoring the need, they create **backlog task groups**:

- A backlog group starts in `draft` state
- It can have inter-group dependencies (e.g., "infrastructure backlog" depends on "auth feature" group)
- Tasks moved during analysis also land in backlog groups

**Auto-promotion rule:** When all of a backlog group's inter-group dependencies are satisfied (`satisfied_at IS NOT NULL` for all entries in `task_group_dependencies`), and the group has at least one task, the system automatically transitions it from `draft` to `analyzing`.

This creates a **continuous pipeline**: active groups execute, produce backlog groups as side effects, and those backlog groups automatically enter the pipeline when they're unblocked.

**Safeguard:** Auto-promotion only applies to groups tagged as `is_backlog = true`. Manually created groups stay in `draft` until the user explicitly triggers analysis. This prevents surprise execution of groups the user is still planning.

```sql
ALTER TABLE task_groups ADD COLUMN is_backlog BOOLEAN NOT NULL DEFAULT FALSE;
```

### 5. Orchestration event system

Extend the existing `TaskEvent` system with **group-level events** — a new `GroupEvent` entity (or extend TaskEvent with a nullable `task_group_id`):

| Event Type | Actor | Payload |
|-----------|-------|---------|
| `group_state_change` | System/Agent | `{from: "draft", to: "analyzing", reason: "dependencies_satisfied"}` |
| `group_analysis_start` | System | `{planner_task_id, task_count}` |
| `group_analysis_complete` | Agent | `{dag, tasks_added, tasks_moved, rationale}` |
| `group_analysis_failed` | Agent | `{error, recommendation}` |
| `group_execution_start` | System | `{first_parallel_set: [...]}` |
| `group_task_started` | System | `{task_id, parallel_set_index, reason: "parallel_set_0_ready"}` |
| `group_task_completed` | System | `{task_id, next_eligible: [...]}` |
| `group_execution_complete` | System | `{duration, tasks_completed, tasks_failed}` |
| `backlog_created` | Agent | `{backlog_group_id, source_group_id, reason}` |
| `backlog_promoted` | System | `{group_id, satisfied_dependencies: [...]}` |
| `dependency_satisfied` | System | `{group_id, depends_on_group_id, satisfied_by_completion_of: group_name}` |
| `task_moved_between_groups` | Agent/System | `{task_id, from_group_id, to_group_id, reason}` |
| `dag_task_added` | Agent | `{task_id, group_id, reason: "gap identified in analysis"}` |

**Every event includes:**
- `id`, `timestamp` — standard audit fields
- `task_group_id` — which group this event belongs to
- `task_id` (nullable) — specific task if relevant
- `actor_type` — `system`, `agent`, or `user`
- `event_type` — from the enum above
- `payload` — JSON with structured data specific to the event type
- `summary` — human-readable one-liner (e.g., "Planner identified missing auth types task and added it to the group")

**The `summary` field is critical.** It's the difference between an audit log (useful for debugging) and an observability surface (useful for understanding). Every event must produce a human-readable summary that answers "what happened and why" in one sentence.

### 6. Orchestration feed

A new UI surface: the **Orchestration Feed**. This is not per-task (that's ADR-008's timeline). This is the project-level view of what's happening across all groups:

- **Live feed** of group events, ordered by time, filterable by group
- **Group cards** showing current state, active tasks, next up in the DAG
- **Decision highlights** — when an agent makes a routing decision (e.g., "decided this task doesn't need testing"), it's surfaced as a highlighted event with rationale
- **Dependency graph** — visual representation of inter-group dependencies with satisfied/pending/blocked states

This feed answers the question: **"What's going to happen next?"** A user can glance at it and see:
- Auth group is executing: task 3 of 5 running, tasks 4 and 5 will run in parallel after
- Infrastructure backlog has 2 tasks, waiting on auth group to complete
- Billing group analysis just finished: planner added a migration task and reordered for parallel execution

### 7. Agent decision logging contract

All agents — not just the planner — must log decisions via the event system. This is a **contract**, not a suggestion:

When an agent makes a decision that affects workflow (not just code):
- Moving tasks between groups → `task_moved_between_groups` event
- Creating backlog work → `backlog_created` event
- Deciding a task doesn't need a particular step (e.g., "no tests needed") → `ManualAction` event with decision rationale in payload
- Completing with a structured deliverable → existing `AgentComplete` event, but now with richer payload

**How agents know to do this:** The system prompt assembled by `container.rs` includes an "Orchestration Protocol" section when the task is part of a group. This section tells the agent:
1. You are part of group `{group_name}`, task {position} of {total}
2. Previous task completed with: {summary of previous task's artifacts}
3. If you discover work outside this group's scope, use the `create_backlog_group` MCP tool
4. All decisions affecting workflow must be logged via the `log_orchestration_event` MCP tool

Two new MCP tools:
- `create_backlog_group` — creates a new group in `draft` state with `is_backlog = true`, optionally with tasks and inter-group dependencies
- `log_orchestration_event` — writes a group event with event_type, summary, and payload

## Consequences

### Positive
- Task groups become active orchestration units with their own lifecycle, not passive containers
- Analysis step catches gaps, fixes ordering, and enables parallel execution — work that previously required manual planning
- Backlog auto-promotion creates a continuous pipeline where discovered work enters the system without user intervention
- Orchestration-level observability gives users confidence to step away from the terminal — they can see decisions, not just terminal output
- The orchestration feed answers "what's happening now" and "what happens next" at a glance
- Decision logging contract makes agent behavior auditable — every routing decision, every scope change, every gap identification is recorded with rationale
- Foundation for future autonomous project management: with analysis + execution + observability, the system can eventually propose entire task groups from a project description

### Negative
- Group lifecycle adds state machine complexity (6 states, multiple transitions, auto-promotion rules)
- Analysis task adds latency before execution begins — every group pays a planning cost even for simple sequences
- Internal DAG execution is more complex than simple sequential auto-dependencies
- Event system expansion adds storage and query overhead (mitigated: events are append-only and indexed by group)
- Agent decision logging contract requires all agent prompts to include orchestration instructions — increases prompt size
- Auto-promotion of backlog groups could create runaway execution if analysis keeps discovering work (mitigated: `is_backlog` flag + user can pause auto-promotion per project)

### Risks
- **Analysis quality depends on agent capability.** A weak planner may produce bad DAGs or miss gaps. Mitigation: analysis artifacts are visible in the orchestration feed, and users can reject analysis and re-plan.
- **Backlog cascade.** Group A's analysis creates backlog B, B's analysis creates backlog C, etc. Mitigation: limit auto-promotion depth (configurable, default 2 levels), surface cascade warnings in the feed.
- **Event volume.** Active projects with many groups could generate hundreds of events per hour. Mitigation: pagination, filtering, and event aggregation in the feed UI.

### Future Work
- **Shared worktree per group**: The execution DAG enables this — parallel tasks in the same set could share a worktree with branch-per-task within the worktree (git worktree stacking)
- **Group templates**: "Backend Feature" = [types, model, API, tests], "Full Stack Feature" = [backend group + frontend group + integration group], pre-configured with DAGs
- **Analysis learning**: Track which gaps the planner commonly identifies and surface them earlier (e.g., "you usually need a types task — add one?")
- **Cost tracking**: Aggregate token usage per group for budgeting and optimization
- **Cross-group artifact flow visualization**: Show which artifacts flowed from group A to group B and whether they were useful
- **Natural language group creation**: "Build auth for my app" → system creates the group, populates tasks, runs analysis

## Alternatives Considered

1. **No analysis step — let users plan manually.** Faster to build but defeats the purpose of agent orchestration. Users are queuing related tasks because they want the system to manage the work, not because they want to meticulously plan every step. The analysis step is where the system earns trust.

2. **Analysis at the task level, not group level.** Each task could analyze its own scope on start. But this misses cross-task concerns: parallel opportunities, missing steps, out-of-scope work. Only a group-level view can see the full picture.

3. **Simple sequential execution (keep ADR-012's approach).** Works for linear workflows but wastes time on independent tasks. "Add auth types" and "Add auth config" have no dependency — running them sequentially when they could be parallel is pure waste. The DAG eliminates this.

4. **User-defined DAG instead of agent-generated.** Users draw the dependency graph manually. More control but more work and more error-prone. The planner agent can handle this better — and the user can always override by editing the group before analysis.

5. **Observability via terminal log aggregation.** Scrape agent terminal output for decisions. Unreliable (output format varies), high volume (99% is irrelevant), and requires parsing natural language. Structured events with explicit summaries are deterministic and queryable.

6. **Separate observability service (external tool like Grafana/Datadog).** Adds infrastructure complexity, requires data export, and loses tight integration with the kanban UI. The orchestration feed belongs in the product — it's not a monitoring concern, it's a core workflow concern.

## Related
- ADR-012: Task Groups (structural foundation — groups, DAG, immutability)
- ADR-013: Group-Scoped Context (knowledge inheritance along the DAG)
- ADR-008: Task Lifecycle Observability (task-level observability — this ADR extends to group level)
- ADR-001: Structured Deliverables (analysis task uses `.vibe/decision.json` for DAG output)
