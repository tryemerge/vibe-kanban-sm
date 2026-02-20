# IMPL-015: Group-Level Worktrees and Split-Screen UI

**Related ADR**: [ADR-015](../adr/2026-02-19-015-group-level-worktrees-and-split-screen-ui.md)
**Status**: In Progress
**Started**: 2026-02-19

## Overview

This implementation plan tracks the major architectural change from per-task worktrees to per-group worktrees, along with the split-screen UI for visualizing the two-level task hierarchy.

## Architecture Summary

```
┌─────────────────────────────────────────────────┐
│ TaskGroup Board (Top) - Visual                  │
│ Backlog → Analyzing → Ready → Executing → Done │
├─────────────────────────────────────────────────┤
│ Task Board (Bottom) - Interactive               │
│ Todo → In Progress → Review → Done → Cancelled │
│ [Agent execution logs / communication]          │
└─────────────────────────────────────────────────┘

TaskGroup.workspace_id → Workspace → Worktree (shared by all tasks)
Task.task_group_id → TaskGroup.workspace_id → Workspace.container_ref
```

## Phase 1: Database Schema Migration

### 1.1 Add workspace_id to task_groups
- [ ] Create migration: `20260219000006_add_workspace_to_task_groups.sql`
- [ ] Add `workspace_id UUID REFERENCES workspaces(id)` to task_groups table
- [ ] Add index on workspace_id
- [ ] Update TaskGroup model struct to include workspace_id field
- [ ] Update TaskGroup::create() to accept optional workspace_id
- [ ] Update TaskGroup queries to include workspace_id in SELECT

**Files to modify:**
- `crates/db/migrations/20260219000006_add_workspace_to_task_groups.sql` (new)
- `crates/db/src/models/task_group.rs`

### 1.2 Add task_group_id to workspaces
- [ ] Create migration: `20260219000007_add_task_group_to_workspaces.sql`
- [ ] Add `task_group_id UUID REFERENCES task_groups(id)` to workspaces table
- [ ] Add index on task_group_id
- [ ] Add unique constraint: `UNIQUE(task_group_id)` (one workspace per group)
- [ ] Update Workspace model struct to include task_group_id field
- [ ] Update Workspace::create() to accept task_group_id parameter

**Files to modify:**
- `crates/db/migrations/20260219000007_add_task_group_to_workspaces.sql` (new)
- `crates/db/src/models/workspace.rs`

### 1.3 Data migration (optional, for existing data)
- [ ] Create migration: `20260219000008_migrate_existing_workspaces.sql`
- [ ] For existing workspaces linked to tasks:
  - If task has task_group_id, link workspace to that group
  - If task has no group, leave workspace.task_group_id NULL (legacy support)
- [ ] Note: May need manual cleanup of orphaned workspaces

**Files to modify:**
- `crates/db/migrations/20260219000008_migrate_existing_workspaces.sql` (new)

### 1.4 Update SQLx cache
- [ ] Run `pnpm run prepare-db` to update SQLx offline cache
- [ ] Verify all queries compile

---

## Phase 2: Backend - Workspace Management at Group Level

### 2.1 Create workspace when TaskGroup enters "Executing"
- [ ] Add method: `TaskGroup::create_workspace(&pool, group_id) -> Result<Workspace>`
- [ ] Modify group state transition handler to detect "ready" → "executing"
- [ ] Call create_workspace when entering executing state
- [ ] Update task_groups.workspace_id with new workspace ID
- [ ] Log group event: "workspace_created"

**Files to modify:**
- `crates/db/src/models/task_group.rs` (add create_workspace method)
- `crates/services/src/services/group_analyzer.rs` (or new group_executor.rs)

### 2.2 Update task execution to find workspace via group
- [ ] Modify container.rs: When starting task execution, look up workspace via:
  - `task.task_group_id` → `task_groups.workspace_id` → `workspaces.container_ref`
- [ ] Add helper: `Task::get_workspace_via_group(&pool, task_id) -> Result<Option<Workspace>>`
- [ ] Fall back to legacy task.parent_workspace_id if task_group_id is NULL (backward compat)
- [ ] Update ExecutionContext to include group_id for context

**Files to modify:**
- `crates/services/src/services/container.rs`
- `crates/db/src/models/task.rs` (add get_workspace_via_group helper)

### 2.3 Remove per-task workspace creation
- [ ] Identify where workspaces are created for tasks (container.rs, task_attempts.rs)
- [ ] Comment out or remove per-task workspace creation code
- [ ] Add check: If task has task_group_id, skip individual workspace creation
- [ ] Keep legacy path for ungrouped tasks (backward compatibility during transition)

**Files to modify:**
- `crates/services/src/services/container.rs`
- `crates/server/src/routes/task_attempts.rs`

---

## Phase 3: Backend - PR Creation at Group Level

### 3.1 Detect when all tasks in group complete
- [ ] Add query: `TaskGroup::all_tasks_completed(&pool, group_id) -> Result<bool>`
- [ ] Check if all tasks in group have status = Done
- [ ] Trigger on task completion event

**Files to modify:**
- `crates/db/src/models/task_group.rs`

### 3.2 Create PR for completed group
- [ ] Add method: `TaskGroup::create_pr(&pool, group_id) -> Result<PullRequest>`
- [ ] Use group's workspace to get git branch and changes
- [ ] Create PR with title: "Group: {group_name}"
- [ ] Include all task titles in PR description
- [ ] Link PR to group (may need new group_events entry)
- [ ] Transition group state: "executing" → "done"

**Files to modify:**
- `crates/services/src/services/git.rs` (or new pr_service.rs)
- `crates/db/src/models/task_group.rs`

### 3.3 Cleanup workspace after PR creation
- [ ] Add method: `TaskGroup::cleanup_workspace(&pool, group_id) -> Result<()>`
- [ ] Decide policy: Keep worktree for history or delete immediately?
- [ ] If deleting: Remove worktree directory, mark workspace as archived
- [ ] If keeping: Flag workspace as read-only, update status

**Files to modify:**
- `crates/services/src/services/worktree_manager.rs` (or group_executor.rs)

---

## Phase 4: Backend - Autonomous Background Services

### 4.1 TaskGrouper Service (polls every 5 min)
- [ ] Keep existing `TaskGrouperService` structure
- [ ] Fix to create analysis tasks (not just log warnings)
- [ ] When threshold met (e.g., 5+ ungrouped tasks):
  - Create "Analyze Backlog: {project_name}" task
  - Task enters workflow → spawns Claude Code
  - Claude uses MCP to create groups and assign tasks
- [ ] Make visible in UI (tasks show up on TaskGroup board)

**Files to modify:**
- `crates/services/src/services/task_grouper.rs`

### 4.2 GroupExecutor Service (monitors groups)
- [ ] Create new service: `GroupExecutorService`
- [ ] Poll every 30 seconds for groups in "ready" state
- [ ] Check if dependencies are satisfied (inter-group dependencies)
- [ ] If ready: Transition group "ready" → "executing"
- [ ] Trigger workspace creation
- [ ] Release tasks to Task board (make them visible for execution)

**Files to create:**
- `crates/services/src/services/group_executor.rs`
- Update `crates/services/src/services/mod.rs` to export

### 4.3 Spawn background services on server start
- [ ] In server main.rs or local-deployment, spawn services:
  - `TaskGrouperService::spawn(db, analytics)`
  - `GroupExecutorService::spawn(db, analytics)`
- [ ] Add graceful shutdown handling

**Files to modify:**
- `crates/server/src/main.rs` or `crates/local-deployment/src/container.rs`

---

## Phase 5: Frontend - Split-Screen UI

### 5.1 Create TaskGroupBoard component
- [ ] New component: `frontend/src/components/tasks/TaskGroupBoard.tsx`
- [ ] Display TaskGroups in columns: Backlog → Analyzing → Ready → Executing → Done
- [ ] Use existing Kanban board component as template
- [ ] Show group name, task count, status
- [ ] Click group → filter bottom board to show only that group's tasks
- [ ] Visual-only (no execution logs, no chat interface)

**Files to create:**
- `frontend/src/components/tasks/TaskGroupBoard.tsx`

### 5.2 Create SplitScreenLayout component
- [ ] New component: `frontend/src/components/layouts/SplitScreenLayout.tsx`
- [ ] Renders TaskGroupBoard (top) + TaskKanbanBoard (bottom)
- [ ] Resizable split (drag divider to resize)
- [ ] Collapsible panels (close top or bottom)
- [ ] State management: Selected group filters bottom board

**Files to create:**
- `frontend/src/components/layouts/SplitScreenLayout.tsx`

### 5.3 Update ProjectTasks page to use split-screen
- [ ] Modify `frontend/src/pages/ProjectTasks.tsx`
- [ ] Replace single Kanban board with SplitScreenLayout
- [ ] Pass project context to both boards
- [ ] Handle group selection → task filtering

**Files to modify:**
- `frontend/src/pages/ProjectTasks.tsx`

### 5.4 Add TaskGroup API hooks
- [ ] Create hook: `useTaskGroups(projectId)` - fetch all groups
- [ ] Create hook: `useTaskGroup(groupId)` - fetch single group
- [ ] Create hook: `useTasksByGroup(groupId)` - fetch tasks in group
- [ ] Add API calls to `frontend/src/lib/api.ts` for task groups

**Files to modify:**
- `frontend/src/hooks/useTaskGroups.ts` (new)
- `frontend/src/lib/api.ts`

---

## Phase 6: Frontend - Task Creation Flow Changes

### 6.1 New tasks land in TaskGroup backlog
- [ ] Update CreateTaskDialog to NOT set column_id (tasks start ungrouped)
- [ ] Tasks appear on TaskGroup board in "Backlog" column
- [ ] Wait for TaskGrouper to analyze and group them
- [ ] Only after grouping do tasks move to Task board

**Files to modify:**
- `frontend/src/components/dialogs/tasks/TaskFormDialog.tsx`

### 6.2 Show ungrouped tasks in TaskGroup backlog
- [ ] TaskGroup backlog column shows:
  - Ungrouped tasks (task_group_id IS NULL)
  - Groups in "draft" state
- [ ] Visual distinction between tasks and groups
- [ ] Count indicator: "5 ungrouped tasks"

**Files to modify:**
- `frontend/src/components/tasks/TaskGroupBoard.tsx`

---

## Phase 7: MCP Integration for Conversational Task Creation

### 7.1 Verify existing MCP tools work with new flow
- [ ] Test `create_task` MCP tool - tasks should land in group backlog
- [ ] Test `create_task_group` MCP tool
- [ ] Test `add_task_to_group` MCP tool
- [ ] Ensure tasks created via MCP appear on TaskGroup board

**Files to verify:**
- `crates/server/src/mcp/task_server.rs`

### 7.2 Add MCP tool for triggering backlog analysis
- [ ] New MCP tool: `trigger_backlog_analysis(project_id)`
- [ ] Allows developer to manually trigger TaskGrouper via conversation
- [ ] Creates analysis task immediately (don't wait for 5 min poll)

**Files to modify:**
- `crates/server/src/mcp/task_server.rs`

---

## Phase 8: Testing & Validation

### 8.1 Database migration testing
- [ ] Test fresh database: Apply all migrations, verify schema
- [ ] Test existing database: Migrate data, verify integrity
- [ ] Test rollback: Ensure migrations can be reverted safely

### 8.2 Backend integration testing
- [ ] Create TaskGroup → verify workspace created when enters "executing"
- [ ] Execute tasks → verify they share the same worktree
- [ ] Complete all tasks → verify single PR created
- [ ] Test task dependencies within group

### 8.3 Frontend testing
- [ ] Test split-screen layout on different screen sizes
- [ ] Test group selection → task filtering
- [ ] Test collapse/expand panels
- [ ] Test drag-and-drop between boards (if applicable)

### 8.4 End-to-end flow testing
- [ ] Create tasks via conversation (Claude Code + MCP)
- [ ] Watch TaskGrouper group them automatically
- [ ] Watch GroupAnalyzer analyze groups
- [ ] Watch tasks execute in shared worktree
- [ ] Verify single PR created per group

---

## Rollout Strategy

### Backward Compatibility During Transition

**Support both models temporarily:**
- [ ] If task.task_group_id IS NOT NULL → use group's workspace
- [ ] If task.task_group_id IS NULL → use legacy per-task workspace (old flow)
- [ ] Allow gradual migration of existing tasks to groups

### Feature Flag (Optional)
- [ ] Add feature flag: `ENABLE_GROUP_WORKTREES`
- [ ] Default to false for safety
- [ ] Enable per-project or globally when ready

### Documentation Updates
- [ ] Update ARCHITECTURE.md with new execution model
- [ ] Update STATUS.md with implementation progress
- [ ] Add user guide: "How Task Groups Work"
- [ ] Update CHANGELOG.md with breaking changes

---

## Completion Checklist

### Phase 1: Database Schema ✅❌
- [ ] Migrations created and tested
- [ ] Models updated
- [ ] SQLx cache regenerated

### Phase 2: Backend Workspace Management ✅❌
- [ ] Workspace creation at group level
- [ ] Task execution finds workspace via group
- [ ] Per-task workspace creation removed/disabled

### Phase 3: Backend PR Creation ✅❌
- [ ] Group completion detection
- [ ] PR creation at group level
- [ ] Workspace cleanup policy implemented

### Phase 4: Background Services ✅❌
- [ ] TaskGrouper service fixed and running
- [ ] GroupExecutor service created and running
- [ ] Services spawn on server start

### Phase 5: Frontend Split-Screen UI ✅❌
- [ ] TaskGroupBoard component created
- [ ] SplitScreenLayout component created
- [ ] ProjectTasks page updated

### Phase 6: Task Creation Flow ✅❌
- [ ] New tasks land in group backlog
- [ ] Ungrouped tasks visible on TaskGroup board

### Phase 7: MCP Integration ✅❌
- [ ] Existing MCP tools verified
- [ ] Trigger backlog analysis tool added

### Phase 8: Testing ✅❌
- [ ] Database migrations tested
- [ ] Backend integration tested
- [ ] Frontend tested
- [ ] End-to-end flow validated

---

## Open Issues & Decisions Needed

### Issue 1: Workspace Cleanup Policy
**Question**: After TaskGroup completes and PR is created, what happens to the worktree?
- **Option A**: Delete immediately (save disk space)
- **Option B**: Keep for history (allow inspection)
- **Decision**: TBD

### Issue 2: Task Failure Handling
**Question**: If a task in the middle of a group fails, what happens?
- **Option A**: Block remaining tasks, mark group as failed
- **Option B**: Continue with remaining tasks, partial group completion
- **Decision**: TBD

### Issue 3: Maximum Group Size
**Question**: Should we limit how many tasks can be in a group?
- **Option A**: No limit (trust the grouper agent)
- **Option B**: Soft limit (10-15 tasks, warn if exceeded)
- **Option C**: Hard limit (reject groups over threshold)
- **Decision**: TBD

### Issue 4: Inter-Group Dependencies
**Question**: How do we model "Group B depends on Group A's PR being merged"?
- See ADR-013 for initial design
- Need to implement dependency checking in GroupExecutor
- **Decision**: Use existing task_group_dependencies table, check PR status

### Issue 5: Manual Task Addition to Groups
**Question**: Can developers manually add tasks to existing groups?
- **Option A**: Yes, via UI drag-and-drop
- **Option B**: No, only automatic grouping allowed
- **Option C**: Yes, but requires re-analysis of the group
- **Decision**: TBD

---

## Notes & Progress Log

### 2026-02-19
- Created ADR-015 and IMPL-015
- Documented architecture and phased implementation plan
- Ready to begin Phase 1 (database schema migrations)

---

## Related Documents

- [ADR-015: Group-Level Worktrees and Split-Screen UI](../adr/2026-02-19-015-group-level-worktrees-and-split-screen-ui.md)
- [ADR-012: Task Groups](../adr/2026-02-19-012-task-groups.md)
- [ADR-013: Group-Scoped Context](../adr/2026-02-19-013-group-scoped-context.md)
- [ADR-014: Task Group Lifecycle and Observability](../adr/2026-02-19-014-task-group-lifecycle-and-observability.md)
