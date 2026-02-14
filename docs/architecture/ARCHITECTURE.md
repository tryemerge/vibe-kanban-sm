# InDusk Architecture Guide

> Read this document to understand the full system before starting any significant work.
> For current project state and priorities, see `docs/journal/STATUS.md`.

## What is InDusk?

**InDusk** (codename: Infinite Dusky) is a heavily enhanced fork of [Vibe Kanban](https://github.com/BloopAI/vibe-kanban), forked December 31, 2025.

**Core thesis:** Agents are not subprocess invocations but **specialized contexts** that shape LLM behavior. When a task enters a workflow column, InDusk assembles a rich context (system prompt + task description + deliverable + accumulated project knowledge) and hands it to an LLM-agnostic executor.

**Key innovation:** Context compounding -- every completed task can produce knowledge artifacts that are stored in the database and automatically injected into future agent prompts. Projects get smarter over time.

## Core Concepts

### Isolated Workspaces
Every task runs in its own **git worktree**. Multiple agents can work in parallel on different tasks without file conflicts. Each workspace has its own branch, working directory, and lifecycle.

### Workflow Engine
Kanban columns are **states in a state machine**. Transitions between columns can be conditional (based on agent decisions), have fallback paths, and escalation after repeated failures. Agents write decisions to `.vibe/decision.json` and InDusk routes automatically.

### Context Compounding
Agents can create **context artifacts** (ADRs, patterns, module memories, decisions) that get stored in the database. Future agents automatically receive relevant artifacts as part of their prompt, building accumulated project knowledge.

### Board Templates
Pre-built workflow configurations (columns + transitions + agents) that can be applied to projects. Built-in templates include "Code Review Pipeline" and "Simple Kanban".

---

## Codebase Map

### Rust Backend (`crates/`)

#### `crates/server/` -- HTTP API + MCP Server
The Axum-based server with two binaries:
- **`main`** -- HTTP API server
- **`mcp_task_server`** -- MCP server binary (used by Claude Code, etc.)

Key locations:
- `src/routes/` -- REST API endpoints (tasks, projects, boards, agents, columns, transitions, labels, triggers)
- `src/mcp/task_server.rs` -- **MCP tool definitions**. This is the primary interface for agent orchestration. Tools: `create_project`, `update_project`, `list_projects`, `list_tasks`, `create_task`, `update_task`, `delete_task`, `get_task`, `start_workspace_session`, `list_boards`, `create_board`, `get_board`, `create_column`, `create_transition`, `list_agents`, `list_repos`, `get_project`
- `src/bin/generate_types.rs` -- TypeScript type generation from Rust structs (ts-rs)
- `src/middleware/` -- Auth, model loaders

> **Critical:** The MCP server uses the **release** binary at `target/release/mcp_task_server`. After MCP code changes, run `cargo build --release --bin mcp_task_server`.

#### `crates/db/` -- Database Layer (SQLx + PostgreSQL)
All database models and migrations.

Key models in `src/models/`:
| Model | File | Purpose |
|-------|------|---------|
| `Task` | `task.rs` | Core task with status, column_id, task_state, agent_status |
| `Project` | `project.rs` | Project with board_id, settings |
| `Workspace` | `workspace.rs` | Git worktree per task attempt |
| `KanbanColumn` | `kanban_column.rs` | Workflow state with agent_id, deliverable, starts_workflow |
| `StateTransition` | `state_transition.rs` | Routing rules with conditions, else/escalation paths |
| `ContextArtifact` | `context_artifact.rs` | Knowledge storage with type, scope, versioning |
| `Agent` | `agent.rs` | Agent config: executor, system_prompt, start_command |
| `TaskTrigger` | `task_trigger.rs` | Soft dependencies between tasks |
| `TaskEvent` | `task_event.rs` | Audit trail of all task state changes |
| `FileLock` | `file_lock.rs` | Prevent parallel file conflicts |
| `TaskLabel` | `task_label.rs` | Project-scoped labels for swim lanes |

Migrations in `migrations/` -- chronological SQL files. Always test with `pnpm run prepare-db`.

#### `crates/services/` -- Business Logic
The core orchestration layer.

- **`src/services/container.rs`** -- **THE critical file.** Contains:
  - `AgentContext` struct -- assembled context for agent execution
  - `read_decision_file()` -- reads `.vibe/decision.json` from workspace
  - `try_create_artifact_from_decision()` -- extracts and stores knowledge artifacts
  - `validate_decision_variable()` -- validates agent output against column options
  - `build_project_context()` -- assembles ADRs + patterns into context string
  - `build_decision_instructions()` -- generates structured output instructions for agents
- **`src/services/project.rs`** -- Project CRUD, board template application
- **`src/services/git.rs`** -- Git operations (branch, commit, merge, worktree)

#### `crates/executors/` -- Agent Executors
LLM-agnostic execution layer. Each executor knows how to:
1. Start an agent session (Claude Code, Codex, Gemini, Cursor, etc.)
2. Pass the assembled context
3. Stream output logs
4. Detect completion

#### Other Crates
- `crates/utils/` -- Shared utilities (logging, text helpers)
- `crates/deployment/` -- Fly.io deployment infrastructure
- `crates/local-deployment/` -- Local dev deployment
- `crates/remote/` -- Remote deployment support

### Frontend (`frontend/`)
React + TypeScript, Vite, Tailwind CSS, shadcn/ui.

Key areas:
- `src/pages/settings/` -- BoardSettings, AgentSettings, TemplatesSettings, LabelsSection
- `src/components/tasks/` -- TaskCard, SwimLane, LabelPicker, LabelBadge
- `src/components/dialogs/` -- TaskFormDialog, and other dialog components
- `src/hooks/` -- useSwimLaneConfig, and other custom hooks
- `src/api/` -- API client functions

### Shared Types (`shared/`)
- `types.ts` -- Auto-generated from Rust structs via ts-rs. **Never edit manually.**
- Regenerate: `DATABASE_URL="sqlite:///path/to/db.sqlite" pnpm run generate-types`

### Documentation (`docs/`)
- `docs/adr/` -- 6 Architecture Decision Records
- `docs/impl/` -- Implementation plans
- `docs/infinite-dusky/CHANGELOG.md` -- Detailed feature history since fork
- `docs/indusk/` -- Mintlify user-facing docs
- `docs/architecture/` -- This file and DATA_MODEL.md (agent-facing)
- `docs/journal/` -- Living project state (STATUS.md, EXPERIMENTS.md)

---

## Data Flow: Task Lifecycle

```
1. CREATE TASK
   User creates task → REST API or MCP `create_task`
   Task placed in initial column (e.g., Backlog)

2. MOVE TO WORKFLOW COLUMN
   Manual drag-drop or task trigger fires
   Task enters column with `starts_workflow = true`

3. WORKSPACE CREATION
   Git worktree created for the task
   Isolated branch, working directory

4. CONTEXT ASSEMBLY
   ┌─ Agent system prompt (from agent config)
   ├─ Task description + workflow history
   ├─ Column deliverable ("What you should produce")
   ├─ Decision instructions ("Write to .vibe/decision.json")
   └─ Project context artifacts (ADRs, patterns, module memories)
       └─ Assembled by build_project_context() + build_full_context()

5. AGENT EXECUTION
   Executor launches (Claude Code, Codex, Gemini, etc.)
   Agent works in isolated worktree
   Agent writes .vibe/decision.json with its decision

6. TRANSITION ROUTING
   try_auto_transition() reads decision file
   validate_decision_variable() checks against column options
   Matching transition → to_column_id
   No match → else_column_id (retry)
   Max failures → escalation_column_id

7. KNOWLEDGE EXTRACTION
   try_create_artifact_from_decision() checks for artifact fields
   If present: creates ContextArtifact in database
   Knowledge compounded for future agents

8. NEXT STATE
   Task moves to next column
   If next column also has starts_workflow + agent → cycle repeats
```

## Data Flow: Context Assembly

The `build_full_context()` method in `context_artifact.rs` assembles three layers:

1. **Global artifacts** (scope = `global`) -- Always included for all agents in the project. Typically ADRs, patterns, dependency info.
2. **Task artifacts** (scope = `task`) -- Only included for the specific task. Task-specific decisions and plans.
3. **Path artifacts** (scope = `path`) -- Included when the agent is working on matching file paths. Module-specific knowledge.

Result is a markdown-formatted string injected into the agent's prompt.

---

## Key Abstractions

### Board vs Project
- **Board** = workflow template (columns + transitions + agents). Can be shared.
- **Project** = instance. Has a board applied, tasks created, repos attached.
- Board templates use `is_template = true` and `template_group_id` for grouping.

### Agent Model
Agents are stored in the database with:
- `name`, `executor` (e.g., "CLAUDE_CODE"), `variant`
- `system_prompt` -- persona and expertise
- `start_command` -- initial instruction for the agent
- `context_files` -- glob patterns for relevant files

Each column can have one agent assigned. When a task enters that column (and `starts_workflow = true`), execution begins automatically.

### Context Artifact Types

| Type | Purpose | Typical Scope |
|------|---------|---------------|
| `module_memory` | Knowledge about a specific file/module | path |
| `adr` | Architecture Decision Record | global |
| `decision` | Specific choice made during development | task or global |
| `pattern` | Reusable pattern or best practice | global |
| `dependency` | Dependency information | global |
| `iplan` | Implementation plan (subtask breakdown) | global |
| `changelog_entry` | Completed work log | global |

Artifacts support **versioning** via `chain_id` (groups versions) and `supersedes_id` (links to previous version).

### State Transition Hierarchy
Transitions can be scoped to different levels (highest priority wins):
1. **Task-level** -- Override for a specific task
2. **Project-level** -- Override for all tasks in a project
3. **Board-level** -- Default behavior from the template

### The `.vibe/` Convention
Agents write structured output to `.vibe/` in their worktree:
- `decision.json` -- Routing decision (e.g., `{"decision": "approve"}`)
- `summary.md` -- Task completion summary
- `context.md` -- Discovered context for future agents

Decision files can also include artifact fields (`artifact_type`, `title`, `content`, `scope`) to create context artifacts automatically.

---

## Key Files Quick Reference

| What you need | File |
|---|---|
| MCP tool definitions | `crates/server/src/mcp/task_server.rs` |
| Core orchestration logic | `crates/services/src/services/container.rs` |
| Context artifact model | `crates/db/src/models/context_artifact.rs` |
| Column model (workflow states) | `crates/db/src/models/kanban_column.rs` |
| Transition model (routing) | `crates/db/src/models/state_transition.rs` |
| Task model | `crates/db/src/models/task.rs` |
| Agent model | `crates/db/src/models/agent.rs` |
| Task triggers | `crates/db/src/models/task_trigger.rs` |
| File locks | `crates/db/src/models/file_lock.rs` |
| REST API routes | `crates/server/src/routes/` |
| Type generation | `crates/server/src/bin/generate_types.rs` |
| Frontend components | `frontend/src/components/` |
| Frontend settings pages | `frontend/src/pages/settings/` |
| TypeScript types (generated) | `shared/types.ts` |
| Database migrations | `crates/db/migrations/` |
| ADRs | `docs/adr/` |
| Feature changelog | `docs/infinite-dusky/CHANGELOG.md` |

---

## InDusk Features vs Base Vibe Kanban

| Feature | Base Vibe Kanban | InDusk |
|---------|------------------|--------|
| Git worktree isolation | Yes | Yes |
| Multi-agent support | Yes | Yes |
| Visual code review | Yes | Yes |
| GitHub integration | Yes | Yes |
| **Workflow state machine** | No | Yes |
| **Conditional routing** | No | Yes |
| **Context artifacts** | No | Yes |
| **Knowledge compounding** | No | Yes |
| **Structured deliverables** | No | Yes |
| **Task triggers** | No | Yes |
| **File locking** | No | Yes |
| **Board templates** | No | Yes |
| **Task labels & swim lanes** | No | Yes |
| **Task events audit trail** | No | Yes |
