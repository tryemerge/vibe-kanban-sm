# InDusk Project Status

**Last Updated:** 2026-02-08
**Current Branch:** `master`
**Active Work:** Documentation restructuring, task state machine migration

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

### MCP Server
- Full project/board/column/transition management tools
- Task CRUD + workspace session management
- `start_workspace_session` for agent orchestration
- Labels auto-created via `create_task` tool
- `create_artifact` + `list_artifacts` tools for context management
- Transition edit via PUT route

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
- Swim lane grouping toggle
- Task triggers UI
- Transition edit button in Board Settings

---

## What's In Progress

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

### This Documentation System
- ARCHITECTURE.md, STATUS.md, EXPERIMENTS.md
- **Status:** Being created now

---

## Known Issues

- Several `.sqlx` cache files deleted and need regeneration (visible in git status)
- Some migrations staged but not applied to dev database yet
- Fly.io deployment scripts added but deployment not tested end-to-end

---

## Open Questions

1. **Functional App Design (ADR-006):** Should Task 1 interactively ask clarifying questions, or should it make best-effort guesses?
2. **Context compounding validation:** How do we measure whether injected artifacts actually improve agent output? What's the experiment?
3. **Task state machine:** When exactly should `task_state` transition between `Queued` and `Transitioning`? Is the current two-state model sufficient?
4. **Board template sharing:** Should templates be exportable/importable between InDusk instances?
5. **Agent file locking:** MCP tools for agents to acquire/release locks are listed as "future work" in the CHANGELOG -- is this needed soon?
6. **Documentation approach:** Are these living docs (ARCHITECTURE.md, STATUS.md) the right layer, or should this knowledge live in the context_artifacts DB instead?

---

## Recent Decisions

- **2026-02-08:** "The Story" pitch doc — end-to-end InDusk walkthrough for pitching/reference
- **2026-02-08:** ADR-007: Context Budget & Relevance System — token-budgeted injection, priority ordering, chain dedup
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

---

## Next Steps (Priority Order)

1. **Validate task state machine migration** -- Apply to dev database, test transitions
2. **Test context compounding end-to-end** -- Create a project, run agents, verify artifacts appear and improve subsequent agents
3. **ADR-006 Phase 2: Research Bot agent** -- Build the agent that uses MCP tools to bootstrap projects
4. **File locking MCP tools** -- Give agents the ability to acquire/release locks
5. **Fly.io deployment testing** -- End-to-end deployment verification
6. **Opinionated project setup** -- Define the standard project scaffolding workflow (turborepo, docs, ADR structure)

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
| 2026-02-08 | ADR-007 context budget, MCP artifact tools, transition edit UI, "The Story" pitch doc |
| 2026-02-07 | Documentation restructuring (InDusk docs, architecture guide, project journal) |
