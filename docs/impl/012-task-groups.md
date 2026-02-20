# Implementation Plan: Task Groups (ADR-012)

## Overview

Add task groups as a first-class entity: a project-scoped grouping of related tasks that execute sequentially. Wire task groups into swim lanes (replacing label-based grouping), auto-create dependencies within groups, and expose via MCP.

## Existing Assets (reuse)

| Asset | Location |
|-------|----------|
| TaskDependency model | `crates/db/src/models/task_dependency.rs` |
| Swim lane config hook | `frontend/src/hooks/useSwimLaneConfig.ts` |
| Swim lane grouping logic | `frontend/src/components/tasks/TaskKanbanBoard.tsx` (`groupItemsByLabel`) |
| TaskLabelsContext pattern | `frontend/src/contexts/TaskLabelsContext.tsx` |
| Label settings page pattern | `frontend/src/pages/settings/LabelSettings.tsx` |
| Task model | `crates/db/src/models/task.rs` |
| MCP task tools | `crates/server/src/mcp/task_server.rs` |
| Type generation | `crates/server/src/bin/generate_types.rs` |

## Steps

### 1. Backend: Migration

**File:** `crates/db/migrations/YYYYMMDD_add_task_groups.sql`

```sql
-- Task groups: project-scoped grouping of related tasks
CREATE TABLE task_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    -- NULL = mutable (draft), set = frozen (executing)
    started_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, name)
);

-- Inter-group dependency DAG
CREATE TABLE task_group_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The blocked group
    task_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    -- The prerequisite group that must complete first
    depends_on_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- When prerequisite group completed (all tasks terminal)
    satisfied_at TIMESTAMPTZ,
    UNIQUE(task_group_id, depends_on_group_id)
);

CREATE INDEX idx_task_group_deps_group ON task_group_dependencies(task_group_id);
CREATE INDEX idx_task_group_deps_prereq ON task_group_dependencies(depends_on_group_id);

-- A task belongs to zero or one group
ALTER TABLE tasks ADD COLUMN task_group_id UUID REFERENCES task_groups(id) ON DELETE SET NULL;
CREATE INDEX idx_tasks_task_group ON tasks(task_group_id);

-- Distinguish auto-created group dependencies from manual ones
ALTER TABLE task_dependencies ADD COLUMN is_auto_group BOOLEAN NOT NULL DEFAULT FALSE;
```

### 2. Backend: TaskGroup model

**File:** `crates/db/src/models/task_group.rs` (new)

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskGroup {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub position: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateTaskGroup {
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateTaskGroup {
    pub name: Option<String>,
    pub color: Option<String>,
}
```

Methods:
- `find_by_project(pool, project_id)` — ordered by position
- `find_by_id(pool, id)`
- `create(pool, data)` — auto-assign position as max+1
- `update(pool, id, data)` — rejects if `started_at` is set (immutable)
- `delete(pool, id)` — rejects if `started_at` is set
- `reorder(pool, project_id, group_ids: Vec<Uuid>)` — update positions
- `mark_started(pool, id)` — sets `started_at = NOW()`
- `is_started(pool, id) -> bool` — convenience check

**File:** `crates/db/src/models/task_group_dependency.rs` (new)

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskGroupDependency {
    pub id: Uuid,
    pub task_group_id: Uuid,
    pub depends_on_group_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub satisfied_at: Option<DateTime<Utc>>,
}
```

Methods:
- `find_by_group(pool, task_group_id)` — what this group is waiting for
- `find_by_prerequisite(pool, depends_on_group_id)` — who is blocked by this group
- `create(pool, task_group_id, depends_on_group_id)`
- `satisfy_by_prerequisite(pool, depends_on_group_id)` — called when all tasks in a group reach terminal
- `has_unsatisfied(pool, task_group_id) -> bool` — check if group can start
- `delete(pool, id)`

**Register:** Add `pub mod task_group;` and `pub mod task_group_dependency;` to `crates/db/src/models/mod.rs`

### 3. Backend: Update Task model

**File:** `crates/db/src/models/task.rs`

- Add `task_group_id: Option<Uuid>` to `Task` struct
- Add `#[ts(type = "string | null")]` annotation
- Add `task_group_id` to all SELECT queries: `find_by_id`, `find_by_project_with_attempt_status`, `create`, `update`
- Add `task_group_id: Option<Uuid>` to `CreateTask` and `UpdateTask`
- Add method: `update_task_group(pool, task_id, task_group_id: Option<Uuid>)`

### 4. Backend: API routes

**File:** `crates/server/src/routes/task_groups.rs` (new)

```
GET    /api/projects/{project_id}/task-groups              — list_task_groups
POST   /api/projects/{project_id}/task-groups              — create_task_group
PUT    /api/projects/{project_id}/task-groups/{group_id}   — update_task_group
DELETE /api/projects/{project_id}/task-groups/{group_id}   — delete_task_group
POST   /api/projects/{project_id}/task-groups/reorder      — reorder_task_groups
POST   /api/tasks/{task_id}/task-group/{group_id}          — add_task_to_group
DELETE /api/tasks/{task_id}/task-group                     — remove_task_from_group
```

**Add task to group logic:**
1. Check `task_group.started_at IS NULL` — reject with 409 if group is frozen
2. Set `task.task_group_id = group_id`
3. Query tasks in the group ordered by position/created_at, find the last one
4. If a previous task exists, call `TaskDependency::create()` with `is_auto_group: true`
5. Return updated task

**Remove task from group logic:**
1. Check `task_group.started_at IS NULL` — reject with 409 if group is frozen
2. Delete all `task_dependencies` where `task_id = target` AND `is_auto_group = true`
3. Delete all `task_dependencies` where `depends_on_task_id = target` AND `is_auto_group = true`
4. Re-link: if task was in the middle of a chain (B depends on A, C depends on B), create A→C dependency
5. Clear `task.task_group_id`

**Inter-group dependency routes:**
```
POST   /api/projects/{project_id}/task-group-dependencies           — add_group_dependency
DELETE /api/projects/{project_id}/task-group-dependencies/{dep_id}  — remove_group_dependency
GET    /api/projects/{project_id}/task-group-dependencies           — list_group_dependencies
```

**Register:** Wire routes in `crates/server/src/routes/mod.rs`

**Immutability trigger:** In the task status update path (`container.rs` or `tasks.rs`), when a task transitions out of `todo` for the first time, check if it belongs to a group. If so, and the group's `started_at` is null, call `TaskGroup::mark_started()`.

**Group completion trigger:** When all tasks in a group reach terminal columns, call `TaskGroupDependency::satisfy_by_prerequisite()` to unblock downstream groups. This mirrors how `TaskDependency::satisfy_by_prerequisite()` works for individual tasks.

### 5. Frontend: Type generation

**File:** `crates/server/src/bin/generate_types.rs`

Register `TaskGroup`, `CreateTaskGroup`, `UpdateTaskGroup`, `TaskGroupDependency` for TypeScript generation.

Run `pnpm run generate-types` to produce types in `shared/types.ts`.

### 6. Frontend: API client

**File:** `frontend/src/lib/api.ts`

```typescript
export const taskGroupsApi = {
  list: async (projectId: string): Promise<TaskGroup[]> => { ... },
  create: async (projectId: string, data: CreateTaskGroup): Promise<TaskGroup> => { ... },
  update: async (projectId: string, groupId: string, data: UpdateTaskGroup): Promise<TaskGroup> => { ... },
  delete: async (projectId: string, groupId: string): Promise<void> => { ... },
  reorder: async (projectId: string, groupIds: string[]): Promise<void> => { ... },
  addTask: async (taskId: string, groupId: string): Promise<void> => { ... },
  removeTask: async (taskId: string): Promise<void> => { ... },
  // Inter-group dependencies
  listDependencies: async (projectId: string): Promise<TaskGroupDependency[]> => { ... },
  addDependency: async (projectId: string, groupId: string, dependsOnGroupId: string): Promise<TaskGroupDependency> => { ... },
  removeDependency: async (projectId: string, depId: string): Promise<void> => { ... },
};
```

### 7. Frontend: Hooks

**File:** `frontend/src/hooks/useTaskGroups.ts` (new)

- `useTaskGroups(projectId)` — React Query, returns `{ data: TaskGroup[], isLoading }`
- `useTaskGroupMutations(projectId)` — returns `{ createGroup, updateGroup, deleteGroup, reorderGroups, addTaskToGroup, removeTaskFromGroup, addDependency, removeDependency }` with query invalidation
- `useTaskGroupDependencies(projectId)` — React Query for listing inter-group dependencies

**File:** `frontend/src/hooks/index.ts` — export new hooks

### 8. Frontend: TaskGroupsContext

**File:** `frontend/src/contexts/TaskGroupsContext.tsx` (new)

Follow the `TaskLabelsContext` pattern:
- `TaskGroupsProvider` wraps the project page
- Provides `groups: TaskGroup[]`, `getGroupForTask(taskId): TaskGroup | undefined`
- Safe variant `useTaskGroupsContextSafe()` for components that might render outside provider

**File:** `frontend/src/pages/ProjectTasks.tsx` — wrap with `TaskGroupsProvider`

### 9. Frontend: Swim lanes by task group

**File:** `frontend/src/hooks/useSwimLaneConfig.ts`

- Add `{ type: 'task_group' }` to `SwimLaneGroupBy` union type
- Change default groupBy from `'none'` to `{ type: 'task_group' }`

**File:** `frontend/src/components/tasks/TaskKanbanBoard.tsx`

- Add `groupItemsByTaskGroup(items, groups)` function alongside existing `groupItemsByLabel()`
- When `groupBy.type === 'task_group'`:
  - Group tasks by `task.task_group_id`
  - "Ungrouped" lane for tasks where `task_group_id` is null
  - Order lanes by `task_group.position`
  - Use `task_group.color` for lane accent color

### 10. Frontend: Task group management UI

**File:** `frontend/src/pages/settings/TaskGroupsSettings.tsx` (new)

- List groups with name, color swatch, task count, status (draft/started)
- Create group: name + color picker
- Edit group inline: name, color — disabled if group is started (show lock icon + tooltip)
- Delete group with confirmation — disabled if group is started
- Drag to reorder (updates position)
- Inter-group dependencies: "depends on" dropdown per group, showing the DAG relationships
- Visual indicator: started groups show a "frozen" badge, draft groups show "editable"

**File:** `frontend/src/pages/settings/SettingsLayout.tsx`

- Add "Task Groups" nav item (use Layers or FolderKanban icon)
- Add route for the new page

**Task assignment UI (in task details):**
- Add "Group" dropdown to task panel or card context menu
- Shows available groups, current assignment
- Selecting a group calls `addTaskToGroup`, selecting "None" calls `removeTaskFromGroup`

### 11. MCP integration

**File:** `crates/server/src/mcp/task_server.rs`

- Add `task_group_id` optional param to existing `create_task` tool
- Add `create_task_group` tool: params `project_id`, `name`, `color`
- Add `add_task_to_group` tool: params `task_id`, `group_id`
- Add `add_group_dependency` tool: params `project_id`, `group_id`, `depends_on_group_id`

Rebuild release binary: `cargo build --release --bin mcp_task_server`

## Files Changed Summary

| File | Change |
|------|--------|
| `crates/db/migrations/YYYYMMDD_*.sql` | New migration: task_groups, task_group_dependencies, tasks.task_group_id, task_dependencies.is_auto_group |
| `crates/db/src/models/task_group.rs` | New model with CRUD + immutability enforcement |
| `crates/db/src/models/task_group_dependency.rs` | New model for inter-group DAG |
| `crates/db/src/models/task.rs` | Add task_group_id to Task, all queries, CreateTask, UpdateTask |
| `crates/db/src/models/task_dependency.rs` | Add is_auto_group to struct and queries |
| `crates/db/src/models/mod.rs` | Register task_group and task_group_dependency modules |
| `crates/server/src/routes/task_groups.rs` | New API routes with auto-dependency + immutability + inter-group deps |
| `crates/server/src/routes/mod.rs` | Register routes |
| `crates/server/src/mcp/task_server.rs` | Add task_group_id to create_task, new MCP tools |
| `crates/server/src/bin/generate_types.rs` | Register TaskGroup + TaskGroupDependency types |
| `crates/services/src/services/container.rs` | Trigger group started_at + group completion satisfaction |
| `frontend/src/lib/api.ts` | Add taskGroupsApi with inter-group dependency methods |
| `frontend/src/hooks/useTaskGroups.ts` | New hooks including useTaskGroupDependencies |
| `frontend/src/hooks/useSwimLaneConfig.ts` | Add task_group groupBy type |
| `frontend/src/hooks/index.ts` | Export new hooks |
| `frontend/src/components/tasks/TaskKanbanBoard.tsx` | Add groupItemsByTaskGroup |
| `frontend/src/contexts/TaskGroupsContext.tsx` | New context provider |
| `frontend/src/pages/ProjectTasks.tsx` | Wrap with TaskGroupsProvider |
| `frontend/src/pages/settings/TaskGroupsSettings.tsx` | New settings page with DAG management |
| `frontend/src/pages/settings/SettingsLayout.tsx` | Add nav item + route |
| `shared/types.ts` | Generated TaskGroup + TaskGroupDependency types |

## Verification

1. `pnpm run prepare-db` — migration succeeds
2. `pnpm run generate-types` — TypeScript types generated
3. `cargo check --workspace` — backend compiles
4. `pnpm run check` — frontend type checks
5. Create a task group in Settings → Task Groups
6. Add tasks to the group → verify auto-dependencies created (check task dependencies in UI)
7. Board shows tasks grouped in swim lanes by task group
8. Tasks within a group execute sequentially (second task doesn't start until first completes)
9. Collapsing a swim lane works and persists across page loads
10. Remove a task from a group → auto-dependencies cleaned up, chain re-linked
11. Start first task in a group → group becomes frozen (started_at set)
12. Attempt to add/remove tasks from started group → 409 Conflict
13. Create second group that depends on first group → second group blocks until first completes
14. All tasks in first group complete → downstream group unblocked (satisfied_at set)
15. MCP: `create_task` with `task_group_id` works
16. MCP: `create_task_group` creates a group visible in settings
17. MCP: `add_group_dependency` creates inter-group dependency
