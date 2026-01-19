# Changelog - Infinite Dusky Fork

This changelog documents features built after forking from the original vibe-kanban codebase.

**Fork Date:** December 31, 2025
**Codename:** Infinite Dusky

---

## [Unreleased]

### Added

#### Task Labels System (ADR 2026-01-18-004)
*Commits: a7870d39 (backend), 49d6092f (frontend)*

- **Task Labels** (`task_labels` table)
  - Project-scoped labels with name and color
  - Reorderable via position field
  - Many-to-many assignment via `task_label_assignments`

- **Labels Management UI** (`frontend/src/components/settings/LabelsSection.tsx`)
  - Create, edit, delete labels in Project Settings
  - Color picker with preset colors
  - Inline editing with dialog forms

- **Label Display on Tasks**
  - `LabelBadge` / `LabelBadges` components for colored badges
  - Shows up to 3 labels on TaskCard with "+N" overflow indicator
  - Auto-contrast text color based on background luminance

- **Label Assignment UI** (`frontend/src/components/tasks/LabelPicker.tsx`)
  - Popover-based label picker in TaskPanel
  - Toggle labels on/off with checkbox-style interface
  - Removable badge display for assigned labels

- **Bulk Loading Pattern**
  - `GET /api/projects/:id/labels/assignments` endpoint
  - Returns all (task_id, label) pairs for efficient loading
  - `TaskLabelsContext` provides labels to kanban board components

- **API Routes**
  - `GET/POST /api/projects/:id/labels` - List/create labels
  - `PUT/DELETE /api/projects/:id/labels/:id` - Update/delete labels
  - `POST /api/projects/:id/labels/reorder` - Reorder labels
  - `GET/POST/DELETE /api/tasks/:id/labels/:id` - Assign/remove labels

- **MCP Integration**
  - `create_task` now accepts optional `labels` array
  - Labels auto-created if they don't exist in the project
  - Consistent color generation based on label name hash

- **Task Edit Dialog**
  - LabelPicker integrated into TaskFormDialog
  - Labels assignable when editing tasks via UI

#### Agent File Locking (ADR 2026-01-18-003)
*Commit: 2409d310*

- **File Lock Model** (`file_locks` table)
  - Lock files/directories before modifying to prevent conflicts
  - Unique constraint per project + file path
  - Optional TTL with `expires_at` for automatic release
  - Glob pattern support (e.g., `src/auth/*`, `**/*.config.js`)

- **Lock Operations**
  - `acquire()` - Claim exclusive access to file(s)
  - `release()` - Explicitly release locks early
  - `check_conflicts()` - Test if paths overlap with existing locks
  - `release_by_workspace()` - Cleanup on task completion

- **Automatic Release**
  - Task marked done/cancelled
  - Workspace deleted
  - TTL expires

- **Future Work**
  - MCP tools for agents to acquire/release locks
  - Deadlock detection
  - Lock status in workspace UI

#### Task Auto-Start Triggers (ADR 2026-01-18-002)
*Commit: 92c085ae*

- **Soft Task Dependencies**
  - `task_triggers` table links tasks in a dependency chain
  - Trigger conditions: `Completed`, `Merged`, `CompletedWithStatus`
  - `is_persistent` flag for one-shot vs recurring triggers
  - No hard blocking - users can still manually start any task

- **Automatic Task Start**
  - When trigger task completes → waiting task moves to workflow column
  - If workflow column has an agent → execution starts automatically
  - One-shot triggers removed after firing

- **API Routes**
  - `GET/POST /api/tasks/:task_id/triggers` - Manage triggers
  - `DELETE /api/tasks/:task_id/triggers/:trigger_id` - Remove trigger

- **Integration Points**
  - Hooks into task completion flow
  - Hooks into PR merge detection
  - Records trigger events in task_events

#### Structured Deliverables (ADR 2026-01-18-001)
*Commit: 7c825706*

- **Structured Deliverable Options on Columns**
  - `deliverable_variable` field (e.g., "decision", "review_outcome")
  - `deliverable_options` JSON array (e.g., `["approve", "reject", "needs_work"]`)
  - Replaces freeform condition values with validated options
  - Enables Transition Builder UI to show available routing options

- **Benefits**
  - Clear contract between workflow design and agent behavior
  - Agents know exactly what values are valid for output
  - Reduces configuration errors from typos
  - Foundation for automated prompt injection with decision instructions

#### Kanban + State Machine Workflow System (Phase 1)
*Commit: 5ea7cc8b*

- **Kanban Columns** (`kanban_columns` table)
  - Customizable columns per project with name, slug, position, color
  - `is_initial` / `is_terminal` flags for workflow boundaries
  - Column-level status mapping (`todo`, `in_progress`, `done`)

- **State Transitions** (`state_transitions` table)
  - Define allowed movements between columns
  - Optional confirmation requirements
  - Foundation for automated routing

- **Automation Rules** (`automation_rules` table)
  - Trigger types: `on_enter`, `on_exit`, `on_complete`
  - Action types: `move_to_column`, `notify`, `run_script`
  - Priority ordering for rule execution

- **Automation Executions** (`automation_executions` table)
  - Track rule execution history
  - Success/failure status with error messages

#### API Routes for Board Configuration (Phase 2)
*Commit: 3e3d13ba*

- `GET/POST /api/agents` - Agent CRUD operations
- `GET/PUT/DELETE /api/agents/:id` - Individual agent management
- `GET/POST /api/boards/:id/columns` - Column management
- `PUT/DELETE /api/boards/:id/columns/:id` - Column updates
- `GET/POST /api/boards/:id/transitions` - Transition management
- `GET/POST /api/boards/:id/automation-rules` - Automation rules

#### Board Settings UI
*Commit: e0ff4e5a*

- **Board Settings Page** (`frontend/src/pages/settings/BoardSettings.tsx`)
  - Visual column editor with drag-and-drop reordering
  - Color picker for column customization
  - Initial/terminal column designation
  - Agent assignment per column

- **Agent Settings Page** (`frontend/src/pages/settings/AgentSettings.tsx`)
  - Create/edit/delete agents
  - Configure system prompts, roles, executors
  - Context files with glob patterns
  - Color-coded agent identification

#### Task Events System
*Commit: 7b229ed9*

- **Task Events** (`task_events` table)
  - Event types: `column_transition`, `status_change`, `agent_start`, `agent_complete`, `commit`, `comment`
  - Actor tracking (user vs automation)
  - Trigger types: `manual`, `drag_drop`, `automation`, `api`
  - Metadata JSON for flexible event data

- **Default Board Seeding**
  - Automatic board creation for new projects
  - Default columns: Backlog, In Progress, Review, Done
  - Proper status mapping out of the box

- **Column-Status Mapping**
  - `status` field on columns (`todo`, `in_progress`, `done`)
  - Task status auto-updates when moved between columns

#### Agent-as-Context Features
*Commit: eedb495e*

- **Column Deliverables**
  - `deliverable` field on `kanban_columns`
  - Describes expected output when task leaves column
  - Injected into agent context for guidance

- **Auto-Start Workflow**
  - `starts_workflow` flag on columns
  - Automatic agent execution when task enters column
  - Agent's `start_command` as initial instruction

- **Conditional State Transitions**
  - `condition_key` / `condition_value` for routing decisions
  - Agents write decisions to `.vibe/decision.json`
  - Example: `{"decision": "approve"}` or `{"decision": "reject", "feedback": "..."}`
  - Automatic routing based on decision values

- **Hierarchical Transitions**
  - `parent_id` for grouping related transitions
  - `else_column_id` for fallback routing
  - `escalation_column_id` for loop prevention
  - `max_failures` threshold before escalation

- **Cancel Attempt Feature**
  - `cancelled_at` timestamp on workspaces
  - API endpoint to cancel running attempts
  - Cleanup of worktree and return to initial column

#### Context Artifacts System
*Commit: e421d295*

- **Context Artifacts** (`context_artifacts` table)
  - Types: `ModuleMemory`, `ADR`, `Decision`, `Pattern`, `Dependency`
  - Scope-based filtering: `path`, `task`, `global`
  - Path patterns for intelligent context matching
  - Active/archived status

- **Workspace Final Context**
  - `final_context` field on workspaces
  - Captures context summary before worktree cleanup
  - Preserves agent learnings across attempts

- **`.vibe/` Convention**
  - Standard directory for agent-written files
  - `summary.md` - Task completion summary
  - `context.md` - Discovered context
  - `decision.json` - Routing decisions

#### Workflow Templates
*Commit: e421d295*

- **Template System**
  - `is_template` / `template_group_id` on agents, boards, columns, transitions
  - Pre-built configurations for common workflows
  - One-click application to projects

- **Built-in Templates**
  - **Code Review Pipeline**: Developer → Reviewer flow with approve/reject routing
  - **Simple Kanban**: Basic To Do → In Progress → Done board

- **Templates Settings Page** (`frontend/src/pages/settings/TemplatesSettings.tsx`)
  - Browse available templates
  - Preview template structure
  - Apply to selected project

---

## Database Migrations (Post-Fork)

| Migration | Description |
|-----------|-------------|
| `20251231000001` | Add kanban_columns table |
| `20251231000002` | Add state_transitions table |
| `20251231000003` | Add automation_rules table |
| `20251231000004` | Add automation_executions table |
| `20251231000005` | Seed default columns |
| `20251231000006` | Fix automation_executions FK |
| `20251231000007` | Add agents table |
| `20260104000001` | Add boards table |
| `20260105000001` | Add agent_id to columns |
| `20260106000001` | Add task_events table |
| `20260106050000` | Make column project_id nullable |
| `20260107000001` | Seed default board |
| `20260107100000` | Add status to kanban_columns |
| `20260107200000` | Fix column slugs |
| `20260107210000` | Add agent color |
| `20260107220000` | Add cancelled_at to workspaces |
| `20260108000001` | Add agent start_command |
| `20260108100000` | Add conditional transitions |
| `20260108120000` | Hierarchical state transitions |
| `20260108130000` | Add else and escalation columns |
| `20260109000001` | Add starts_workflow to columns |
| `20260110000001` | Add agent deliverable |
| `20260111000001` | Add deliverable to columns |
| `20260112000001` | Add context_artifacts table |
| `20260112000002` | Add workspace final_context |
| `20260112000003` | Add artifact scope |
| `20260112000004` | Add template fields |
| `20260112000005` | Seed workflow templates |
| `20260118000001` | Add structured deliverables to columns |
| `20260118000002` | Add task_triggers table |
| `20260118000003` | Add file_locks table |

---

## Architecture Decisions

### Agent-as-Context Model
Agents are not sub-process invocations but **specialized contexts** that shape LLM behavior. When a task enters a column with an agent:
1. Agent's system prompt establishes persona
2. Task description + workflow history provides context
3. Agent's start command gives instructions
4. Column's deliverable sets expectations

This approach is LLM-agnostic (works with Claude, Gemini, Codex, etc.).

### State Machine Routing
Transitions between columns can be:
- **Static**: Column A → Column B (unconditional)
- **Conditional**: Route based on `.vibe/decision.json` values
- **Escalating**: After N failures, route to human review

### Context Preservation
Context artifacts persist across:
- Task attempts (same task, different execution)
- Related tasks (via path patterns)
- Global project knowledge (via global scope)

---

## Pre-Fork Foundation

The original codebase provided:
- Project and task management
- Workspace/worktree handling
- Executor sessions and process management
- Git operations (branch, commit, PR)
- User authentication
- Frontend React app with shadcn/ui
