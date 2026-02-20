# Implementation Plan: Task Group Lifecycle & Orchestration Observability (ADR-014)

## Overview

Add a lifecycle state machine to task groups (draft → analyzing → ready → executing → done), auto-created analysis tasks with planner agents, internal execution DAGs for parallel task execution within groups, backlog group auto-promotion, and an orchestration event system that surfaces every decision at the group level.

## Existing Assets (reuse)

| Asset | Location |
|-------|----------|
| TaskGroup model | `crates/db/src/models/task_group.rs` |
| TaskGroupDependency model | `crates/db/src/models/task_group_dependency.rs` |
| TaskEvent model | `crates/db/src/models/task_event.rs` |
| Task events API routes | `crates/server/src/routes/task_events.rs` |
| TaskEventTimeline component | `frontend/src/components/tasks/TaskDetails/TaskEventTimeline.tsx` |
| Container (agent context assembly) | `crates/services/src/services/container.rs` |
| MCP task tools | `crates/server/src/mcp/task_server.rs` |
| useTaskGroups hooks | `frontend/src/hooks/useTaskGroups.ts` |
| TaskGroupsContext | `frontend/src/contexts/TaskGroupsContext.tsx` |
| Task group settings UI | `frontend/src/pages/settings/TaskGroupsSettings.tsx` |

## Steps

### Phase 1: Group Lifecycle State Machine

#### 1.1 Migration: Add state column and is_backlog

**File:** `crates/db/migrations/YYYYMMDD_add_task_group_lifecycle.sql`

```sql
-- Add lifecycle state to task groups
-- Replaces the binary started_at approach with explicit states
ALTER TABLE task_groups ADD COLUMN state TEXT NOT NULL DEFAULT 'draft';
-- Allowed: 'draft', 'analyzing', 'ready', 'executing', 'done', 'failed'

-- Mark backlog groups for auto-promotion behavior
ALTER TABLE task_groups ADD COLUMN is_backlog BOOLEAN NOT NULL DEFAULT FALSE;

-- Store the analysis task's execution DAG for the group
-- JSON: { "parallel_groups": [["task-id-1", "task-id-2"], ["task-id-3"]] }
ALTER TABLE task_groups ADD COLUMN execution_dag TEXT;

-- Index for finding groups by state (useful for auto-promotion queries)
CREATE INDEX idx_task_groups_state ON task_groups(project_id, state);
```

#### 1.2 Update TaskGroup model

**File:** `crates/db/src/models/task_group.rs`

- Add `state: String` field to `TaskGroup` struct (with `#[ts(type = "string")]`)
- Add `is_backlog: bool` field
- Add `execution_dag: Option<String>` field
- Add to `CreateTaskGroup`: `is_backlog: Option<bool>`
- Add state transition methods:
  - `transition_state(pool, id, from: &str, to: &str) -> Result<TaskGroup>` — validates the transition is legal, updates state
  - `find_promotable_backlogs(pool, project_id) -> Vec<TaskGroup>` — finds backlog groups in `draft` state where all inter-group dependencies are satisfied
  - `update_execution_dag(pool, id, dag: &str) -> Result<TaskGroup>`
- Update immutability logic: reject mutations when `state` is not `draft` (replaces `started_at IS NULL` check)

**Valid state transitions:**
```rust
fn is_valid_transition(from: &str, to: &str) -> bool {
    matches!((from, to),
        ("draft", "analyzing") |
        ("analyzing", "ready") |
        ("analyzing", "failed") |
        ("ready", "executing") |
        ("executing", "done") |
        ("executing", "failed") |
        ("failed", "draft")
    )
}
```

#### 1.3 Update existing immutability checks

**Files:** `crates/server/src/routes/task_groups.rs`, `crates/server/src/mcp/task_server.rs`

Replace all `started_at IS NOT NULL` immutability checks with `state != 'draft'` checks. The `started_at` timestamp remains for auditing but is no longer the gatekeeper.

### Phase 2: Orchestration Events

#### 2.1 Migration: Group events table

**File:** Same migration as 1.1 (or a new one)

```sql
CREATE TABLE group_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    task_id UUID REFERENCES tasks(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    actor_type TEXT NOT NULL DEFAULT 'system',
    summary TEXT NOT NULL,
    payload TEXT, -- JSON
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_group_events_group ON group_events(task_group_id, created_at);
CREATE INDEX idx_group_events_type ON group_events(event_type);
```

#### 2.2 GroupEvent model

**File:** `crates/db/src/models/group_event.rs` (new)

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct GroupEvent {
    pub id: Uuid,
    pub task_group_id: Uuid,
    pub task_id: Option<Uuid>,
    pub event_type: String,
    pub actor_type: String,
    pub summary: String,
    pub payload: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

Methods:
- `create(pool, group_id, event_type, actor, summary, payload, task_id)` — insert event
- `find_by_group(pool, group_id, limit, offset)` — paginated, newest first
- `find_by_project(pool, project_id, limit, offset)` — all events across groups, for the orchestration feed
- `find_by_type(pool, group_id, event_type)` — filter by event type

**Event types (as string constants):**
- `group_state_change`
- `group_analysis_start`
- `group_analysis_complete`
- `group_analysis_failed`
- `group_execution_start`
- `group_task_started`
- `group_task_completed`
- `group_execution_complete`
- `backlog_created`
- `backlog_promoted`
- `dependency_satisfied`
- `task_moved_between_groups`
- `dag_task_added`

Register in `crates/db/src/models/mod.rs`.

#### 2.3 Group events API routes

**File:** `crates/server/src/routes/group_events.rs` (new)

```
GET /api/task-groups/{group_id}/events         — list events for a group (paginated)
GET /api/projects/{project_id}/group-events    — orchestration feed (all groups, paginated)
```

Register in `crates/server/src/routes/mod.rs`.

#### 2.4 Emit events from existing state changes

**Files:** `crates/server/src/routes/task_groups.rs`, `crates/services/src/services/container.rs`

Anywhere a group state change occurs, emit a `GroupEvent`:
- Group created → `group_state_change` (to: "draft")
- Task added to group → `dag_task_added`
- Task removed from group → `task_moved_between_groups`
- Group state transitions → `group_state_change` with from/to in payload
- Group dependency satisfied → `dependency_satisfied`

### Phase 3: Analysis Task & Planner

#### 3.1 Analysis task auto-creation

**File:** `crates/services/src/services/container.rs` (or a new `crates/services/src/services/group_orchestrator.rs`)

When a group transitions from `draft` to `analyzing`:

1. Create a task in the group with title `[Analysis] {group_name}`
2. Set the task's description to include:
   - List of all tasks in the group with titles and descriptions
   - Group's inter-group dependencies (what upstream groups have produced)
   - Any existing group-scoped artifacts
3. Assign the task to the group's analysis column/agent (configurable per board, or use a default planner agent)
4. Emit `group_analysis_start` event
5. Start the task via the normal workflow engine (column enter → agent start)

**Analysis task's structured deliverable:**

The planner agent produces a `.vibe/decision.json` with:
```json
{
  "analysis_outcome": "ready",  // or "failed"
  "execution_dag": {
    "parallel_groups": [
      ["task-uuid-1", "task-uuid-2"],
      ["task-uuid-3"]
    ]
  },
  "tasks_added": [
    { "title": "Add auth types", "description": "..." }
  ],
  "tasks_moved_to_backlog": [
    { "task_id": "uuid", "backlog_group_name": "Infrastructure", "reason": "..." }
  ],
  "rationale": "..."
}
```

#### 3.2 Analysis completion handler

**File:** Same as 3.1

When the analysis task completes (reaches terminal column):

1. Parse the structured deliverable
2. If `analysis_outcome == "ready"`:
   - Create any `tasks_added` tasks in the group
   - Move any `tasks_moved_to_backlog` tasks to their backlog groups (create groups if needed, mark `is_backlog = true`)
   - Store `execution_dag` on the group (`task_groups.execution_dag`)
   - Transition group to `ready`
   - Emit `group_analysis_complete` event with the full analysis result
3. If `analysis_outcome == "failed"`:
   - Transition group to `failed`
   - Emit `group_analysis_failed` event
4. Check if any tasks in `tasks_moved_to_backlog` created new groups → emit `backlog_created` events

#### 3.3 Group context in agent system prompt

**File:** `crates/services/src/services/container.rs`

When assembling context for a task that belongs to a group, include an "Orchestration Protocol" section:

```
## Orchestration Context

You are working on task {position} of {total} in group "{group_name}".
Group state: {state}

Previous task in this group completed with:
{summary of previous task's artifacts/deliverables}

### Orchestration Protocol
- If you discover work outside this group's scope, use the `create_backlog_group` MCP tool
- All decisions affecting workflow must be logged via the `log_orchestration_event` MCP tool
- When you complete, your structured deliverable will be evaluated for the next step
```

### Phase 4: DAG-Driven Execution

#### 4.1 Execution engine

**File:** `crates/services/src/services/group_orchestrator.rs` (new)

A service that manages group execution based on the DAG:

- `start_group_execution(pool, group_id)` — called when group transitions to `executing`
  - Parse `execution_dag` from the group
  - Start all tasks in the first parallel set
  - Emit `group_execution_start` event listing first parallel set
  - Emit `group_task_started` event for each task

- `on_task_completed(pool, task_id)` — called when any task in an executing group reaches a terminal column
  - Check if the current parallel set is complete (all tasks in the set are terminal)
  - If yes, advance to the next parallel set: start all tasks in it
  - If this was the last set: transition group to `done`, emit `group_execution_complete`
  - Emit `group_task_completed` with info about next eligible tasks

- `check_backlog_promotions(pool, project_id)` — called after any group completes
  - Find all `is_backlog = true` groups in `draft` state
  - For each, check if all inter-group dependencies are satisfied
  - If satisfied, transition to `analyzing` (triggering analysis task creation)
  - Emit `backlog_promoted` event

#### 4.2 Wire into existing task completion flow

**File:** `crates/services/src/services/container.rs`

After a task reaches a terminal column (existing logic), add:

1. If task belongs to a group and group state is `executing`:
   - Call `group_orchestrator::on_task_completed()`
2. If a group just completed (all tasks terminal):
   - Call `TaskGroupDependency::satisfy_by_prerequisite()` for the group
   - Call `group_orchestrator::check_backlog_promotions()` for the project

#### 4.3 Ready → Executing auto-transition

When a group reaches `ready` state and has no unsatisfied inter-group dependencies, automatically transition to `executing` and call `start_group_execution()`.

This may happen:
- Immediately after analysis completes (if no inter-group deps)
- When an upstream group completes and satisfies this group's last dependency

### Phase 5: MCP Tools for Agents

#### 5.1 New MCP tools

**File:** `crates/server/src/mcp/task_server.rs`

- `create_backlog_group` — creates a task group with `is_backlog = true`, optionally adds tasks and inter-group dependencies
  - Params: `project_id`, `name`, `tasks: Vec<{title, description}>`, `depends_on_group_ids: Vec<Uuid>`
  - Emits `backlog_created` event

- `log_orchestration_event` — writes a group event
  - Params: `task_group_id`, `event_type`, `summary`, `payload` (optional JSON)
  - Validates that the calling task belongs to the group

Rebuild: `cargo build --release --bin mcp_task_server`

### Phase 6: Frontend — Type Generation & API

#### 6.1 Update type generation

**File:** `crates/server/src/bin/generate_types.rs`

Register `GroupEvent` for TypeScript generation. TaskGroup type already registered — will pick up new fields (`state`, `is_backlog`, `execution_dag`) automatically.

Run `pnpm run generate-types`.

#### 6.2 Frontend API client

**File:** `frontend/src/lib/api.ts`

Add to `taskGroupsApi`:
- `transitionState(projectId, groupId, toState)` — trigger state transition
- `getEvents(groupId, limit?, offset?)` — group events
- `getProjectEvents(projectId, limit?, offset?)` — orchestration feed

Add new:
```typescript
export const groupEventsApi = {
  listByGroup: async (groupId: string, limit?: number, offset?: number): Promise<GroupEvent[]> => { ... },
  listByProject: async (projectId: string, limit?: number, offset?: number): Promise<GroupEvent[]> => { ... },
};
```

#### 6.3 Frontend hooks

**File:** `frontend/src/hooks/useGroupEvents.ts` (new)

- `useGroupEvents(groupId)` — React Query for group events, polling every 5s during active states
- `useOrchestrationFeed(projectId)` — React Query for project-wide events
- `groupEventsKeys` — query key factory

### Phase 7: Frontend — Orchestration Feed UI

#### 7.1 Orchestration Feed page

**File:** `frontend/src/pages/OrchestrationFeed.tsx` (new)

A project-level page showing the live feed of group events:
- Chronological event list with group badges, event type icons, and human-readable summaries
- Filter by group, event type, or actor
- Auto-refresh (polling or future WebSocket)
- Group state cards at the top: visual summary of each group's current state

This page answers "what's happening now" and "what happens next."

#### 7.2 Group detail panel

**File:** `frontend/src/components/tasks/TaskDetails/GroupDetailPanel.tsx` (new)

When clicking a group (from swim lane header or orchestration feed):
- Group state with lifecycle indicator
- Execution DAG visualization (simple: ordered list of parallel sets, each showing task cards)
- Event timeline (reuse TaskEventTimeline pattern but for GroupEvents)
- Inter-group dependencies (upstream/downstream groups)
- Analysis results (if analysis has run): tasks added, tasks moved, rationale

#### 7.3 Update TaskGroupsSettings

**File:** `frontend/src/pages/settings/TaskGroupsSettings.tsx`

- Show group `state` instead of just "Started"/"Editable"
- Add state transition buttons: "Start Analysis" (draft → analyzing), "Reset" (failed → draft)
- Show `is_backlog` badge for backlog groups
- Show execution DAG preview for groups that have been analyzed

#### 7.4 Navigation

**File:** `frontend/src/App.tsx`, `frontend/src/pages/ProjectTasks.tsx`

- Add "Orchestration" tab/toggle alongside the kanban board (or as a sidebar panel)
- Route: `/projects/:id/orchestration` or integrate into the existing project view

### Phase 8: Planner Agent Configuration

#### 8.1 Default planner agent definition

The analysis task needs an agent to run. Options:
1. Use the existing agent assigned to the column the analysis task enters
2. Create a built-in "Planner" agent definition with a system prompt tuned for analysis

Recommend option 2 — a dedicated planner agent:

**File:** Migration seed data or agent definition in UI

```
Name: "Group Planner"
System prompt: "You are a planning agent. Review the tasks in this group, identify gaps,
determine optimal execution order (which tasks can run in parallel), and produce a
structured analysis. Use create_backlog_group for out-of-scope work."
Executor: CLAUDE_CODE (or configurable)
```

#### 8.2 Board integration

The planner agent needs a column to run in. Two approaches:
1. **Dedicated "Analysis" column** on the board — analysis tasks route through it
2. **Virtual execution** — analysis tasks run without a visible column, using the planner agent directly

Recommend approach 1 for observability — the analysis task is visible in the board, in a dedicated column, with its own timeline and events. This makes the system's behavior transparent.

Add a `planner_agent_id` field to `task_groups` or `projects` so the planner can be configured.

## Files Changed Summary

| File | Change |
|------|--------|
| `crates/db/migrations/YYYYMMDD_*.sql` | Add state, is_backlog, execution_dag to task_groups; create group_events table |
| `crates/db/src/models/task_group.rs` | Add state/is_backlog/execution_dag fields, state transition methods, promotable backlog query |
| `crates/db/src/models/group_event.rs` | New model for orchestration events |
| `crates/db/src/models/mod.rs` | Register group_event module |
| `crates/services/src/services/group_orchestrator.rs` | New service: DAG execution, backlog promotion, analysis task creation |
| `crates/services/src/services/container.rs` | Wire group lifecycle triggers into task completion flow, add orchestration context to agent prompts |
| `crates/server/src/routes/task_groups.rs` | Add state transition endpoint, update immutability to use state |
| `crates/server/src/routes/group_events.rs` | New routes for group and project-level events |
| `crates/server/src/routes/mod.rs` | Register new routes |
| `crates/server/src/mcp/task_server.rs` | Add create_backlog_group, log_orchestration_event tools |
| `crates/server/src/bin/generate_types.rs` | Register GroupEvent type |
| `frontend/src/lib/api.ts` | Add groupEventsApi, extend taskGroupsApi |
| `frontend/src/hooks/useGroupEvents.ts` | New hooks for group events and orchestration feed |
| `frontend/src/pages/OrchestrationFeed.tsx` | New page: project-level orchestration feed |
| `frontend/src/components/tasks/TaskDetails/GroupDetailPanel.tsx` | New panel: group detail with DAG, events, analysis |
| `frontend/src/pages/settings/TaskGroupsSettings.tsx` | Update: show state, state transition buttons, backlog badges |
| `frontend/src/App.tsx` | Add orchestration feed route |
| `shared/types.ts` | Generated GroupEvent type, updated TaskGroup type |

## Verification

### Phase 1 (Lifecycle)
1. `pnpm run prepare-db` — migration succeeds
2. Create a task group → state is `draft`
3. Add tasks → works while in `draft`
4. Transition to `analyzing` → group becomes immutable
5. Attempt to add/remove tasks → rejected
6. `failed → draft` reset works, group becomes mutable again

### Phase 2 (Events)
7. Every state change produces a `group_state_change` event
8. Events have human-readable summaries
9. Events API returns paginated results for group and project

### Phase 3 (Analysis)
10. `draft → analyzing` creates an analysis task in the group
11. Analysis task receives full group context (task list, artifacts)
12. Analysis task's deliverable is parsed on completion
13. New tasks from `tasks_added` appear in the group
14. Tasks from `tasks_moved_to_backlog` move to backlog groups
15. `execution_dag` is stored on the group

### Phase 4 (DAG Execution)
16. `ready → executing` starts first parallel set
17. When first set completes, second set starts automatically
18. When all sets complete, group transitions to `done`
19. Downstream groups unblocked when upstream completes
20. Backlog groups auto-promote when dependencies satisfied

### Phase 5 (MCP)
21. Agent can call `create_backlog_group` — creates group with `is_backlog = true`
22. Agent can call `log_orchestration_event` — event appears in feed
23. Release binary rebuilt and working

### Phase 6-7 (Frontend)
24. Orchestration feed shows live events across all groups
25. Group detail panel shows DAG, events, analysis results
26. Task group settings show lifecycle state and transition buttons
27. Events have clear, readable summaries

### End-to-End
28. Create a group with 4 tasks → start analysis → planner identifies gap and adds task → DAG built → execution starts → tasks run per DAG → group completes → backlog group auto-promotes → cycle continues
29. User watches entire flow in the orchestration feed without opening any terminal
