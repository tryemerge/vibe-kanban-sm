# Proposal: Kanban + State Machine Workflow Architecture

**Date:** December 31, 2024
**Status:** Draft
**Author:** Architecture Discussion

---

## Executive Summary

Replace the Factory Floor visual workflow builder with a simpler, more intuitive Kanban + State Machine model inspired by Atlassian/Jira. The Kanban board itself becomes the workflow visualization - no separate graph editor needed.

---

## Problem Statement

### Current Factory Floor Limitations

1. **Complexity Overhead**: 5+ database tables (workflows, stations, transitions, executions, station_executions) for a concept users struggle to map mentally
2. **Separate Paradigm**: Users must learn a new visual programming metaphor distinct from familiar Kanban boards
3. **Maintenance Burden**: React Flow graph editor, custom node types, edge routing, and complex orchestration logic
4. **Debugging Difficulty**: When workflows fail, tracing through station executions across a visual graph is harder than following a linear state progression

### What Users Actually Want

- Drag tasks between columns (they already do this)
- Have automations trigger when tasks move (this is the missing piece)
- Define rules for what moves are allowed (simple state machine)

---

## Proposed Solution

### Core Concept

**Kanban columns ARE workflow states.** Moving a task between columns IS a workflow transition. Automations attach to column entry/exit.

```
┌─────────┐    ┌─────────────┐    ┌─────────────┐    ┌──────┐
│  Todo   │───▶│ In Progress │───▶│ Code Review │───▶│ Done │
└─────────┘    └─────────────┘    └─────────────┘    └──────┘
                     │                   │
                     │    On Enter:      │    On Enter:
                     │    Run Agent      │    Create PR
                     │                   │    Run Review Agent
                     │                   │
                     └───────────────────┘
                         (needs changes)
```

### Key Differences from Factory Floor

| Aspect | Factory Floor | Kanban State Machine |
|--------|--------------|---------------------|
| Visualization | Separate graph editor | Kanban board itself |
| Mental Model | Visual programming | Familiar Kanban + rules |
| Tables | 5+ | 4 |
| Editing | Drag nodes/edges | Configure columns + rules |
| User Learning | High | Low (they know Kanban) |

---

## Database Schema

### New Tables

```sql
-- 1. Customizable columns per project
-- Replaces hardcoded status enum with flexible states
CREATE TABLE kanban_columns (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,              -- "Todo", "In Progress", "Code Review", "Done"
    slug TEXT NOT NULL,              -- "todo", "in_progress", "code_review", "done"
    position INTEGER NOT NULL,       -- For ordering columns left-to-right
    color TEXT,                      -- Optional: hex color for UI
    is_initial BOOLEAN DEFAULT FALSE,-- Tasks start here when created
    is_terminal BOOLEAN DEFAULT FALSE,-- Tasks end here (done/cancelled states)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, slug)
);

-- 2. Allowed transitions between columns (the state machine)
-- If no transitions defined, all moves are allowed (open workflow)
-- If transitions exist, only defined moves are permitted
CREATE TABLE state_transitions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    from_column_id TEXT NOT NULL REFERENCES kanban_columns(id) ON DELETE CASCADE,
    to_column_id TEXT NOT NULL REFERENCES kanban_columns(id) ON DELETE CASCADE,
    name TEXT,                       -- Optional: "Start Work", "Request Review"
    requires_confirmation BOOLEAN DEFAULT FALSE,  -- Show confirmation dialog
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_column_id, to_column_id)
);

-- 3. Automation rules triggered on column entry/exit
CREATE TABLE automation_rules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    column_id TEXT NOT NULL REFERENCES kanban_columns(id) ON DELETE CASCADE,
    trigger_type TEXT NOT NULL,      -- 'on_enter' | 'on_exit'
    action_type TEXT NOT NULL,       -- 'run_agent' | 'create_pr' | 'merge_pr' | 'webhook' | 'notify'
    action_config TEXT NOT NULL,     -- JSON configuration for the action
    enabled BOOLEAN DEFAULT TRUE,
    priority INTEGER DEFAULT 0,      -- Execution order for multiple rules on same trigger
    name TEXT,                       -- Human-readable rule name
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 4. Execution log for automation runs
CREATE TABLE automation_executions (
    id TEXT PRIMARY KEY,
    rule_id TEXT NOT NULL REFERENCES automation_rules(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    attempt_id TEXT REFERENCES task_attempts(id) ON DELETE SET NULL,
    status TEXT NOT NULL,            -- 'pending' | 'running' | 'completed' | 'failed' | 'skipped'
    trigger_context TEXT,            -- JSON: what triggered this (transition details)
    result TEXT,                     -- JSON: output or error message
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient queries
CREATE INDEX idx_kanban_columns_project ON kanban_columns(project_id, position);
CREATE INDEX idx_state_transitions_from ON state_transitions(from_column_id);
CREATE INDEX idx_automation_rules_column ON automation_rules(column_id, trigger_type);
CREATE INDEX idx_automation_executions_task ON automation_executions(task_id, created_at);
```

### Modified Tables

```sql
-- Update tasks table to reference kanban_columns instead of status enum
-- Migration will convert existing status values to column references
ALTER TABLE tasks ADD COLUMN column_id TEXT REFERENCES kanban_columns(id);

-- Keep status for backwards compatibility during migration, then drop
-- ALTER TABLE tasks DROP COLUMN status;  -- After migration complete
```

### Action Config Schema Examples

```json
// run_agent action
{
  "agent_id": "uuid",
  "prompt_template": "Implement the following task:\n\n{{task.title}}\n\n{{task.description}}",
  "executor": "CLAUDE_CODE",
  "timeout_minutes": 60
}

// create_pr action
{
  "title_template": "{{task.title}}",
  "body_template": "## Summary\n{{task.description}}\n\n## Changes\n{{attempt.summary}}",
  "draft": false,
  "auto_merge_on_approval": false
}

// webhook action
{
  "url": "https://example.com/webhook",
  "method": "POST",
  "headers": {"Authorization": "Bearer {{secrets.WEBHOOK_TOKEN}}"},
  "body_template": {"task_id": "{{task.id}}", "event": "{{trigger.type}}"}
}

// notify action
{
  "channel": "slack",
  "webhook_url": "{{secrets.SLACK_WEBHOOK}}",
  "message_template": "Task '{{task.title}}' moved to {{column.name}}"
}
```

---

## Repository Structure

### Branching Strategy

```
main          ← Pure mirror of upstream vibe-kanban
  │              (never commit custom code here)
  │
  └── develop ← Kanban + State Machine implementation
        │
        ├── feature/kanban-columns
        ├── feature/state-transitions
        ├── feature/automation-rules
        └── feature/automation-executor
```

### Sync Workflow

```bash
# Keep main synced with upstream
git checkout main
git fetch upstream
git reset --hard upstream/master
git push origin main --force

# Rebase develop onto updated main
git checkout develop
git rebase main
git push origin develop --force-with-lease
```

---

## Implementation Plan

### Phase 1: Foundation (Database + Models)

**Goal:** Create the new tables and Rust models without breaking existing functionality.

**Tasks:**
1. Create migration for `kanban_columns` table
2. Create migration for `state_transitions` table
3. Create migration for `automation_rules` table
4. Create migration for `automation_executions` table
5. Create Rust models with SQLx queries
   - `crates/db/src/models/kanban_column.rs`
   - `crates/db/src/models/state_transition.rs`
   - `crates/db/src/models/automation_rule.rs`
   - `crates/db/src/models/automation_execution.rs`
6. Generate TypeScript types
7. Create default columns migration (seed data for existing projects)

**Deliverable:** Database ready, models ready, existing functionality unchanged.

---

### Phase 2: API Layer

**Goal:** REST endpoints for managing columns, transitions, and rules.

**Tasks:**
1. Kanban columns CRUD
   - `GET /api/projects/:id/columns`
   - `POST /api/projects/:id/columns`
   - `PUT /api/projects/:id/columns/:columnId`
   - `DELETE /api/projects/:id/columns/:columnId`
   - `POST /api/projects/:id/columns/reorder`

2. State transitions CRUD
   - `GET /api/projects/:id/transitions`
   - `POST /api/projects/:id/transitions`
   - `DELETE /api/projects/:id/transitions/:transitionId`

3. Automation rules CRUD
   - `GET /api/projects/:id/automations`
   - `POST /api/projects/:id/automations`
   - `PUT /api/projects/:id/automations/:ruleId`
   - `DELETE /api/projects/:id/automations/:ruleId`
   - `POST /api/projects/:id/automations/:ruleId/test` (dry run)

4. Task column updates
   - `PATCH /api/tasks/:id/column` (move task, triggers automations)

**Deliverable:** Full API for new system, can be tested via curl/Postman.

---

### Phase 3: Automation Executor

**Goal:** Service that runs automation rules when tasks move columns.

**Tasks:**
1. Create `AutomationExecutor` service
   - Listen for task column changes
   - Find applicable rules (on_exit from old, on_enter to new)
   - Execute rules in priority order
   - Log executions to `automation_executions`

2. Implement action handlers
   - `RunAgentHandler` - reuses existing agent executor infrastructure
   - `CreatePrHandler` - creates GitHub PR from attempt
   - `MergePrHandler` - merges approved PR
   - `WebhookHandler` - HTTP calls
   - `NotifyHandler` - Slack/Discord notifications

3. Error handling and retry logic
   - Configurable retry count per rule
   - Exponential backoff
   - Failure doesn't block task movement (async execution)

4. SSE events for automation progress
   - `automation:started`
   - `automation:completed`
   - `automation:failed`

**Deliverable:** Moving a task triggers automations automatically.

---

### Phase 4: Frontend - Kanban Board

**Goal:** Replace/enhance the existing Kanban with customizable columns.

**Tasks:**
1. Update `KanbanBoard` component
   - Fetch columns from API (not hardcoded)
   - Render dynamic number of columns
   - Column headers show name + color

2. Drag-and-drop with transition validation
   - On drag start, highlight valid drop targets
   - Invalid transitions show disabled/red
   - On drop, call `PATCH /api/tasks/:id/column`

3. Task cards show automation status
   - Spinner when automation running
   - Success/failure indicators
   - Click to view automation execution log

4. Column management UI
   - Add/remove/reorder columns
   - Edit column name/color
   - Set initial/terminal flags

**Deliverable:** Fully functional Kanban with dynamic columns.

---

### Phase 5: Frontend - Automation Rules UI

**Goal:** UI for configuring automation rules per column.

**Tasks:**
1. Column settings panel
   - Click column header → settings drawer
   - "On Enter" rules section
   - "On Exit" rules section

2. Rule editor
   - Select action type (dropdown)
   - Configure action (form fields based on type)
   - Enable/disable toggle
   - Priority ordering (drag to reorder)

3. Rule templates
   - "Run coding agent" preset
   - "Create PR when done" preset
   - "Notify on failure" preset

4. Execution history view
   - Per-task automation log
   - Filter by rule, status, date
   - View full execution details (input/output)

**Deliverable:** Users can configure automations without code.

---

### Phase 6: Migration & Cleanup

**Goal:** Migrate existing data, remove Factory Floor code.

**Tasks:**
1. Data migration script
   - Convert `tasks.status` to `tasks.column_id`
   - Create default columns for existing projects
   - Map existing workflows to automation rules (best effort)

2. Remove Factory Floor code
   - Delete `workflows`, `workflow_stations`, `workflow_transitions` tables
   - Delete `workflow_executions`, `station_executions` tables
   - Remove `WorkflowOrchestrator` service
   - Remove React Flow components
   - Remove `/factory-floor` route

3. Update documentation
   - New user guide for Kanban automations
   - Migration guide for existing users

4. Final testing
   - Full regression test
   - Performance testing with many rules

**Deliverable:** Clean codebase with only the new system.

---

## Success Metrics

1. **Reduced Complexity**
   - 4 tables instead of 5+
   - ~50% less backend code
   - No React Flow dependency

2. **Improved UX**
   - Users configure automations in < 2 minutes
   - No learning curve (they know Kanban)
   - Clear visibility into what's running

3. **Maintainability**
   - Easier to debug (linear state flow)
   - Simpler to extend (add new action types)
   - Upstream sync possible (main branch stays clean)

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Existing workflows can't be migrated | Document as breaking change, provide migration guide |
| Users want complex branching logic | Recommend external orchestration (n8n, Temporal) for complex cases |
| Performance with many rules | Execute async, batch where possible, add caching |
| Upstream diverges significantly | Keep main synced, rebase develop regularly |

---

## Open Questions

1. **Should transitions be optional or required?**
   - Option A: If no transitions defined, all moves allowed (open workflow)
   - Option B: Always require explicit transitions (strict workflow)
   - **Recommendation:** Option A - less friction for simple use cases

2. **How to handle in-flight automations when task moves again?**
   - Option A: Cancel previous, start new
   - Option B: Queue new, wait for previous
   - Option C: Block move until automation completes
   - **Recommendation:** Option A - most intuitive behavior

3. **Should automation failures block task movement?**
   - Option A: No - task moves, automation fails in background
   - Option B: Yes - task stays until automation succeeds
   - Option C: Configurable per rule
   - **Recommendation:** Option C - flexibility matters

---

## Next Steps

1. Review and approve this proposal
2. Create new repository with branching structure
3. Begin Phase 1 implementation
4. Weekly check-ins on progress

---

## Appendix: Example Workflow Configuration

### Simple Development Workflow

**Columns:**
| Position | Name | Slug | Initial | Terminal |
|----------|------|------|---------|----------|
| 0 | Backlog | backlog | ✓ | |
| 1 | In Progress | in_progress | | |
| 2 | Code Review | code_review | | |
| 3 | Done | done | | ✓ |

**Transitions:**
| From | To | Name |
|------|-----|------|
| Backlog | In Progress | Start Work |
| In Progress | Code Review | Request Review |
| Code Review | In Progress | Needs Changes |
| Code Review | Done | Approve & Merge |

**Automation Rules:**
| Column | Trigger | Action | Config |
|--------|---------|--------|--------|
| In Progress | on_enter | run_agent | Claude Code, implement task |
| Code Review | on_enter | create_pr | Draft PR with changes |
| Done | on_enter | merge_pr | Merge approved PR |
