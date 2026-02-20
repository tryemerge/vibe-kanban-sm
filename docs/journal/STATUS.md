# InDusk Project Status

**Last Updated:** 2026-02-19
**Current Branch:** `master`
**Active Work:** Group-Level Worktrees & Split-Screen UI (ADR-015 planning phase)

---

## What's Working

### Core Workflow Engine
- Kanban columns as workflow states with `is_initial`, `is_terminal`, `starts_workflow`
- State transitions with conditional routing based on `.vibe/decision.json`
- Else paths (retry) and escalation paths (after N failures)
- Agent auto-start when task enters a workflow column
- Structured deliverables with `deliverable_variable` and `deliverable_options`
- Decision validation against allowed options

### Context Compounding
- Context artifacts table with 7 types (module_memory, adr, decision, pattern, dependency, iplan, changelog_entry)
- Three scopes: global (always), task (specific task), path (matching files)
- Unified injection: `build_full_context()` with token budget, priority ordering, chain dedup (ADR-007)
- `try_create_artifact_from_decision()` auto-extracts artifacts from `.vibe/decision.json`
- Version tracking with `chain_id` and `supersedes_id`
- MCP tools: `create_artifact` + `list_artifacts` for agents to produce artifacts during work

### Board Templates
- Template system with `is_template` / `template_group_id` on agents, boards, columns, transitions
- Built-in: "Code Review Pipeline" (Developer -> Reviewer with approve/reject routing)
- Built-in: "Simple Kanban" (To Do -> In Progress -> Done)
- Research board template for Functional App Design (ADR-006)
- Templates Settings UI for browsing and applying templates

### Task Management Extensions
- Task labels (project-scoped, colored, with swim lane grouping)
- Task triggers (soft dependencies -- "start B after A completes")
- File locking (prevent parallel agent conflicts)
- Task events audit trail
- **Task groups (ADR-012)** -- project-scoped grouping with sequential execution

### Task Groups (ADR-012) -- NEW
- `task_groups` table with project scope, name, color, position, started_at
- `task_group_dependencies` table for inter-group DAG
- `task_group_id` FK on tasks (nullable, ON DELETE SET NULL)
- `is_auto_group` flag on task_dependencies to distinguish auto vs manual deps
- Auto-dependency chain: adding task to group creates dependency on previous task
- Group immutability: `started_at` freezes group once first task executes (409 Conflict on mutations)
- Dependency chain re-linking: removing task from middle of chain re-links predecessor→successor
- Swim lane grouping by task group (alongside existing label grouping)
- TaskGroupsContext providing group lookup to all components
- Settings UI: create/edit/delete groups, manage inter-group dependencies, color picker
- MCP tools: `create_task_group`, `add_task_to_group`, `add_group_dependency`, `task_group_id` on `create_task`
- Full API: CRUD, reorder, task assignment, inter-group dependencies

### MCP Server
- Full project/board/column/transition management tools
- Task CRUD + workspace session management
- `start_workspace_session` for agent orchestration
- Labels auto-created via `create_task` tool
- `create_artifact` + `list_artifacts` tools for context management
- Transition edit via PUT route
- Task group tools (create_task_group, add_task_to_group, add_group_dependency)

### Context Budget System (ADR-007)
- Token-budgeted context injection (8000 token default, 50% global / 30% task / 20% path)
- Priority ordering by artifact type (ADR > Pattern > IPlan > ModuleMemory > Decision > ...)
- Chain deduplication (only latest version per chain_id)
- `token_estimate` column computed on create/update
- Unified injection path: container.rs uses `build_full_context()` with all three scopes

### Frontend
- Board settings page (column editor, drag-and-drop reordering)
- Agent settings page (system prompts, executors, context files)
- Templates settings page (browse, preview, apply)
- Labels management + assignment UI
- Swim lane grouping toggle (label-based and task-group-based)
- Task triggers UI
- Transition edit button in Board Settings
- Task Groups settings page
- Task lifecycle observability (event timeline, context panel, knowledge base)

---

## What's In Progress

### Group-Level Worktrees & Split-Screen UI (ADR-015) -- ACTIVE
- **Major architectural change:** Move from per-task worktrees to per-group worktrees
- **Split-screen UI:** TaskGroup board (top, visual) + Task board (bottom, interactive)
- **One PR per group:** All tasks in a group share a worktree, create one cohesive PR
- **Autonomous orchestration:** Background services automatically group tasks, analyze, and execute
- **Implementation:** 8-phase plan created (IMPL-015)
  - Phase 1: Database schema (workspace_id on task_groups, task_group_id on workspaces)
  - Phase 2-3: Backend workspace management + PR creation at group level
  - Phase 4: Background services (TaskGrouper polling, GroupExecutor monitoring)
  - Phase 5-6: Frontend split-screen layout + new task creation flow
  - Phase 7-8: MCP integration + testing
- **Status:** ADR and implementation plan written, ready to begin Phase 1

### Task Group Lifecycle & Observability (ADR-014) -- Design Complete
- ADR-014 written: group lifecycle state machine (draft → analyzing → ready → executing → done)
- Implementation plan written: 8 phases covering backend, orchestration engine, frontend
- Key concepts: auto-created analysis tasks, internal execution DAGs for parallelism, backlog auto-promotion, orchestration event system
- **Status:** Design documents complete, implementation not started

### Group-Scoped Context (ADR-013) -- Design Complete
- ADR-013 written: `group` artifact scope, DAG-based context inheritance, knowledge production prompts per group, scope promotion lifecycle
- **Status:** Design document complete, implementation not started

### Task State Machine (Migration Exists, Not Applied to Dev DB)
- New `task_state` enum: `Queued`, `Transitioning`
- New `agent_status` enum: `Running`, `AwaitingResponse`
- Migration: `20260204000004_add_task_state_and_agent_status.sql`
- Rust models updated in `task.rs` with `TaskState` and `AgentStatus`
- **Status:** Migration written, models updated, but not yet applied to dev database

### InDusk Documentation
- Mintlify docs restructured with two tabs: "Vibe Kanban" (base) and "InDusk" (fork features)
- Dark gradient headers on InDusk pages for visual distinction
- 5 InDusk pages: Welcome, The Story (pitch/walkthrough), How It Works, Workflow Engine, Context System
- "The Story" page: end-to-end pitch doc showing idea-to-shipped-feature flow with concrete SaaS billing example
- **Status:** Content written, visual styling applied, needs preview/polish

---

## Known Issues

- Several `.sqlx` cache files deleted and need regeneration (visible in git status)
- Some migrations staged but not applied to dev database yet
- Fly.io deployment scripts added but deployment not tested end-to-end
- ADR-012 migration (`20260219000001_add_task_groups.sql`) not yet applied to dev database

---

## Open Questions

1. **Functional App Design (ADR-006):** Should Task 1 interactively ask clarifying questions, or should it make best-effort guesses?
2. **Context compounding validation:** How do we measure whether injected artifacts actually improve agent output? What's the experiment?
3. **Task state machine:** When exactly should `task_state` transition between `Queued` and `Transitioning`? Is the current two-state model sufficient?
4. **Board template sharing:** Should templates be exportable/importable between InDusk instances?
5. **Planner agent configuration (ADR-014):** Should analysis use a dedicated board column or virtual execution? Dedicated column recommended for observability but adds board complexity.
6. **Backlog cascade depth (ADR-014):** Default auto-promotion depth limit of 2 levels -- is this sufficient or too conservative?

---

## Recent Decisions

- **2026-02-19:** ADR-015: Group-Level Worktrees & Split-Screen UI -- Major architectural change: worktrees created at TaskGroup level (not per-task), split-screen UI (TaskGroup board + Task board), one PR per group, autonomous background orchestration. Implementation plan created with 8 phases.
- **2026-02-19:** ADR-014: Task Group Lifecycle & Orchestration Observability -- group state machine, auto-analysis, DAG execution, backlog promotion, orchestration event system
- **2026-02-19:** ADR-013: Group-Scoped Context -- group artifact scope, DAG-based inheritance, scope promotion
- **2026-02-19:** ADR-012: Task Groups -- implemented end-to-end (migration, models, routes, MCP, frontend, swim lanes)
- **2026-02-14:** ADR-008: Task Lifecycle Observability -- event timeline, context panel, knowledge base settings
- **2026-02-08:** "The Story" pitch doc -- end-to-end InDusk walkthrough for pitching/reference
- **2026-02-08:** ADR-007: Context Budget & Relevance System -- token-budgeted injection, priority ordering, chain dedup
- **2026-02-08:** MCP tools for artifact CRUD (`create_artifact`, `list_artifacts`)
- **2026-02-08:** Transition edit UI (UpdateStateTransition backend + frontend edit dialog)
- **2026-02-07:** Created three-layer documentation system (AGENTS.md pointer -> ARCHITECTURE.md stable knowledge -> STATUS.md/EXPERIMENTS.md living state)
- **2026-02-07:** Restructured Mintlify docs with separate Vibe Kanban / InDusk tabs
- **2026-02-04:** Added task_state and agent_status to Task model for finer-grained state tracking
- **2026-01-26:** ADR-006: Functional Application Design -- single prompt bootstraps entire dev environment
- **2026-01-23:** ADR-005: Fly.io chosen for cloud deployment
- **2026-01-18:** ADRs 001-004: Structured deliverables, task triggers, file locking, swim lanes

---

## Architecture Debt / Things to Revisit

- `automation_rules` and `automation_executions` tables exist but aren't actively used (from early workflow design)
- Column `status` field maps to `TaskStatus` but the mapping is somewhat rigid -- may need more flexibility
- Template seeding happens via SQL migrations rather than a dedicated seeding mechanism
- Context artifact `file_path` field exists but its role vs `path` is unclear
- ADR-012's `started_at` immutability will be replaced by ADR-014's explicit `state` column -- minor migration needed

---

## Next Steps (Priority Order)

1. **Apply ADR-012 migration to dev database** -- Test task groups end-to-end with real data
2. **Implement ADR-014 Phase 1-2** -- Group lifecycle state machine + orchestration events (foundation for everything else)
3. **Implement ADR-014 Phase 3-4** -- Analysis task + DAG-driven execution (the core orchestration loop)
4. **Implement ADR-013** -- Group-scoped context + DAG inheritance (knowledge flows with execution)
5. **Implement ADR-014 Phase 5-7** -- MCP tools for agents + orchestration feed UI
6. **Validate task state machine migration** -- Apply to dev database, test transitions
7. **Test context compounding end-to-end** -- Create a project, run agents, verify artifacts appear and improve subsequent agents

---

## Development Timeline

| Date | Milestone |
|------|-----------|
| 2025-12-31 | Forked from Vibe Kanban |
| 2026-01-08 | Workflow engine: columns, transitions, agents |
| 2026-01-09 | Agent-as-Context: auto-start, deliverables, conditional routing |
| 2026-01-10 | Escalation paths, hierarchical transitions |
| 2026-01-12 | Context artifacts + workflow templates |
| 2026-01-18 | ADRs 001-004: Structured deliverables, triggers, file locking, labels |
| 2026-01-23 | ADR-005: Fly.io deployment |
| 2026-01-26 | ADR-006: Functional Application Design |
| 2026-02-04 | MCP server extensions, task state machine, board template seeding |
| 2026-02-07 | Documentation restructuring (InDusk docs, architecture guide, project journal) |
| 2026-02-08 | ADR-007 context budget, MCP artifact tools, transition edit UI, "The Story" pitch doc |
| 2026-02-14 | ADR-008 task lifecycle observability (event timeline, context panel, knowledge base) |
| 2026-02-19 | ADR-012 task groups (full implementation), ADR-013 group-scoped context (design), ADR-014 group lifecycle & observability (design) |
