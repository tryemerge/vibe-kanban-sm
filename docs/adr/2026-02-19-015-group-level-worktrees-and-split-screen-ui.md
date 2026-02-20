# ADR 015: Group-Level Worktrees and Split-Screen UI

**Date**: 2026-02-19
**Status**: Proposed
**Deciders**: Product Team
**Related**: ADR-012 (Task Groups), ADR-014 (Task Group Lifecycle)

## Context

InDusk currently operates with a per-task workspace model where each task gets its own git worktree and creates its own PR. This works for independent tasks but creates several problems:

1. **Fragmented Changes**: Related tasks create separate PRs, requiring manual coordination
2. **Lost Context**: Each task starts from a clean slate, can't build on previous work
3. **Merge Overhead**: Multiple small PRs instead of one cohesive PR per feature
4. **Unclear Organization**: No visual distinction between task organization and task execution
5. **Manual Grouping**: Developer must manually decide which tasks belong together

With Task Groups now providing logical organization of related tasks, we have an opportunity to fundamentally restructure how work is executed.

## Decision

We will implement a **two-level execution model** with **group-level worktrees** and a **split-screen UI**:

### Architectural Changes

#### 1. Worktrees at Group Level (Not Task Level)

**Current Model (Per-Task Worktrees):**
```
Task A → Workspace A → Worktree A → PR A
Task B → Workspace B → Worktree B → PR B
Task C → Workspace C → Worktree C → PR C
```

**New Model (Per-Group Worktrees):**
```
TaskGroup → Workspace → ONE Worktree → ONE PR
  ├─ Task A (executes in shared worktree)
  ├─ Task B (executes in shared worktree, builds on Task A)
  └─ Task C (executes in shared worktree, builds on Task B)
```

#### 2. Split-Screen UI: Two-Level Board

**Top Board (TaskGroup Level - Visual Only):**
- Shows: Backlog → Analyzing → Ready → Executing → Done
- Purpose: Watch autonomous orchestration
- Interaction: Visual only, no chat/execution logs

**Bottom Board (Task Level - Interactive):**
- Shows: Todo → In Progress → Review → Done → Cancelled
- Purpose: See agent execution, communicate if agent asks questions
- Interaction: Shows execution logs, allows developer feedback

#### 3. New Task Flow

```
Developer creates task (via conversation/UI)
  ↓
Task lands in TaskGroup Backlog (TOP board)
  ↓
[Every 5 min] TaskGrouper analyzes ungrouped tasks
  ↓
Tasks grouped → TaskGroup moves to "Analyzing"
  ↓
GroupAnalyzer builds execution DAG, creates inter-task dependencies
  ↓
TaskGroup approved → moves to "Ready"
  ↓
Dependencies satisfied → TaskGroup moves to "Executing"
  ↓
Worktree created for the group
  ↓
Tasks appear on Task board (BOTTOM) and execute based on DAG
  ↓
Each task builds on previous task's work in shared worktree
  ↓
All tasks complete → ONE PR created for entire group
```

## Consequences

### Positive

1. **Cohesive PRs**: One PR per feature/group instead of many small PRs
2. **Cumulative Progress**: Tasks build on each other's work in the same worktree
3. **Reduced Merge Overhead**: Fewer PRs to review and merge
4. **Clear Visualization**: Two levels (organization vs execution) clearly separated
5. **Autonomous Orchestration**: System automatically groups, analyzes, and executes
6. **Better Context**: Agent sees all previous work in the group when executing tasks

### Negative

1. **Higher PR Size**: PRs contain all work from a group (could be large)
2. **Sequential Dependencies**: Failures in early tasks block later tasks
3. **Shared State**: Tasks must handle potential conflicts from previous tasks
4. **Complex Rollback**: Reverting a PR undoes all tasks in the group
5. **Migration Complexity**: Requires significant schema and code changes

### Risks

1. **Large Worktree State**: Group worktrees accumulate changes, may become complex
2. **Task Isolation**: Tasks are no longer isolated, may cause unexpected interactions
3. **PR Review Burden**: Reviewers must understand entire feature, not just one task
4. **Workspace Cleanup**: Need proper cleanup when groups fail or are cancelled

## Database Schema Changes

```sql
-- TaskGroups now own workspaces
ALTER TABLE task_groups ADD COLUMN workspace_id UUID REFERENCES workspaces(id);

-- Workspaces belong to groups (not tasks)
ALTER TABLE workspaces ADD COLUMN task_group_id UUID REFERENCES task_groups(id);
ALTER TABLE workspaces DROP COLUMN task_id; -- Deprecated

-- Tasks find their workspace via group
-- (tasks.task_group_id -> task_groups.workspace_id -> workspaces.container_ref)
-- No direct task.workspace_id needed anymore
```

## Execution Model Changes

### Workspace Creation
- **Old**: When task enters workflow column
- **New**: When TaskGroup enters "Executing" state

### Task Execution
- **Old**: Task has direct workspace_id reference
- **New**: Task looks up: task.task_group_id → group.workspace_id → workspace.container_ref

### PR Creation
- **Old**: One PR per task (when task completes)
- **New**: One PR per group (when all tasks in group complete)

## Implementation Phases

See [IMPL-015](../impl/015-group-level-worktrees-and-split-screen-ui.md) for detailed implementation plan.

High-level phases:
1. Database schema migration (add group.workspace_id, migrate data)
2. Backend workspace management (create at group level)
3. Task execution updates (find workspace via group)
4. Frontend split-screen UI (two-level board)
5. PR creation at group level
6. Background services (TaskGrouper, GroupExecutor)

## Open Questions

1. **How to handle task failures?**
   - Do we roll back the group worktree?
   - Or leave it in failed state for debugging?

2. **What if a group becomes too large?**
   - Should we have a max tasks per group?
   - Allow splitting groups?

3. **How to handle manual task creation?**
   - Should developer be able to add tasks directly to Task board?
   - Or must all tasks go through group backlog?

4. **Workspace cleanup policy?**
   - Keep worktrees after group completes (for history)?
   - Or clean up immediately to save disk space?

5. **Inter-group dependencies?**
   - How do we model "Group B depends on Group A's PR being merged"?
   - See ADR-013 for initial thoughts on group dependencies

## References

- [ADR-012: Task Groups](./2026-02-19-012-task-groups.md)
- [ADR-013: Group-Scoped Context](./2026-02-19-013-group-scoped-context.md)
- [ADR-014: Task Group Lifecycle and Observability](./2026-02-19-014-task-group-lifecycle-and-observability.md)
- [IMPL-015: Implementation Plan](../impl/015-group-level-worktrees-and-split-screen-ui.md)
