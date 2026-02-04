# Implementation Plan: Functional Application Design

**ADR:** [006-functional-application-design](../adr/2026-01-26-006-functional-application-design.md)
**Created:** 2026-01-26
**Status:** Planning

## Overview

Enable users to create a project with a single prompt, and have a "Research Bot" (Claude Code + MCP tools) automatically bootstrap the entire development environment.

---

## Phase 1: MCP Server Extensions

**Goal:** Add missing MCP tools to enable programmatic board/project management.

### 1.1 Project Management Tools

```rust
// File: crates/server/src/mcp/task_server.rs

#[tool(description = "Create a new project with a repository")]
async fn create_project(
    &self,
    Parameters(req): Parameters<CreateProjectRequest>,
) -> Result<CallToolResult, ErrorData>

#[tool(description = "Update project settings (board, scripts, agent_working_dir, copy_files)")]
async fn update_project(
    &self,
    Parameters(req): Parameters<UpdateProjectRequest>,
) -> Result<CallToolResult, ErrorData>

#[tool(description = "Get project details including board and settings")]
async fn get_project(
    &self,
    Parameters(req): Parameters<GetProjectRequest>,
) -> Result<CallToolResult, ErrorData>
```

**Request/Response Types:**
```rust
struct CreateProjectRequest {
    name: String,
    repo_url: Option<String>,        // Clone from URL
    repo_path: Option<String>,       // Or use existing path
    board_id: Option<Uuid>,          // Apply board template
}

struct UpdateProjectRequest {
    project_id: Uuid,
    name: Option<String>,
    board_id: Option<Uuid>,
    setup_script: Option<String>,
    cleanup_script: Option<String>,
    dev_script: Option<String>,
    agent_working_dir: Option<String>,
    copy_files: Option<String>,
}
```

### 1.2 Board Management Tools

```rust
#[tool(description = "List all available board templates")]
async fn list_boards(&self) -> Result<CallToolResult, ErrorData>

#[tool(description = "Create a new board template")]
async fn create_board(
    &self,
    Parameters(req): Parameters<CreateBoardRequest>,
) -> Result<CallToolResult, ErrorData>

#[tool(description = "Get board with columns and transitions")]
async fn get_board(
    &self,
    Parameters(req): Parameters<GetBoardRequest>,
) -> Result<CallToolResult, ErrorData>
```

### 1.3 Column Management Tools

```rust
#[tool(description = "Add a column to a board")]
async fn create_column(
    &self,
    Parameters(req): Parameters<CreateColumnRequest>,
) -> Result<CallToolResult, ErrorData>

#[tool(description = "Update column settings (agent, status, position)")]
async fn update_column(
    &self,
    Parameters(req): Parameters<UpdateColumnRequest>,
) -> Result<CallToolResult, ErrorData>
```

**Request Types:**
```rust
struct CreateColumnRequest {
    board_id: Uuid,
    name: String,
    slug: String,
    color: Option<String>,
    status: String,              // "todo", "inprogress", "inreview", "done", "cancelled"
    is_initial: Option<bool>,
    is_terminal: Option<bool>,
    starts_workflow: Option<bool>,
    agent_id: Option<Uuid>,
    position: Option<i32>,
}
```

### 1.4 Transition Management Tools

```rust
#[tool(description = "Create a state transition between columns")]
async fn create_transition(
    &self,
    Parameters(req): Parameters<CreateTransitionRequest>,
) -> Result<CallToolResult, ErrorData>
```

**Request Type:**
```rust
struct CreateTransitionRequest {
    board_id: Uuid,
    from_column_id: Uuid,
    to_column_id: Uuid,
    name: Option<String>,
    condition_key: Option<String>,
    condition_value: Option<String>,
    else_column_id: Option<Uuid>,
    escalation_column_id: Option<Uuid>,
    max_failures: Option<i32>,
    requires_confirmation: Option<bool>,
}
```

### 1.5 Agent Tools

```rust
#[tool(description = "List all available agents with their capabilities")]
async fn list_agents(&self) -> Result<CallToolResult, ErrorData>

#[tool(description = "Get agent configuration details")]
async fn get_agent(
    &self,
    Parameters(req): Parameters<GetAgentRequest>,
) -> Result<CallToolResult, ErrorData>
```

### 1.6 Task Trigger Tools

```rust
#[tool(description = "Add a trigger to a task")]
async fn create_task_trigger(
    &self,
    Parameters(req): Parameters<CreateTaskTriggerRequest>,
) -> Result<CallToolResult, ErrorData>
```

### Implementation Tasks

- [ ] Add `CreateProjectRequest`, `UpdateProjectRequest` structs
- [ ] Implement `create_project` tool
- [ ] Implement `update_project` tool
- [ ] Implement `get_project` tool
- [ ] Add `CreateBoardRequest` struct
- [ ] Implement `list_boards` tool
- [ ] Implement `create_board` tool
- [ ] Implement `get_board` tool
- [ ] Add `CreateColumnRequest`, `UpdateColumnRequest` structs
- [ ] Implement `create_column` tool
- [ ] Implement `update_column` tool
- [ ] Add `CreateTransitionRequest` struct
- [ ] Implement `create_transition` tool
- [ ] Implement `list_agents` tool
- [ ] Implement `get_agent` tool
- [ ] Implement `create_task_trigger` tool
- [ ] Update TypeScript types (`pnpm run generate-types`)
- [ ] Test all new tools via MCP client

---

## Phase 2: Research Bot Agent

**Goal:** Create a specialized Claude Code configuration for project bootstrapping.

### 2.1 Research Bot Agent Definition

Create a new agent in the database seed:

```sql
-- File: crates/db/migrations/YYYYMMDD_seed_research_bot.sql

INSERT INTO agents (id, name, executor, variant, system_prompt, created_at, updated_at)
VALUES (
    'research-bot-uuid-here',
    'Research Bot',
    'CLAUDE_CODE',
    'OPUS',
    'You are a project research and setup assistant...',
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
);
```

### 2.2 Research Bot System Prompt

```markdown
# Research Bot

You are a project bootstrapping assistant. Your job is to analyze the user's project
requirements and set up the complete development environment.

## Your Capabilities

You have access to the vibe-kanban MCP server with these tools:
- Project management: create_project, update_project, get_project
- Board management: list_boards, create_board, get_board
- Column management: create_column, update_column
- Transition management: create_transition
- Agent management: list_agents, get_agent
- Task management: create_task, update_task, list_tasks
- Trigger management: create_task_trigger

## Your Process

1. **Analyze Requirements**
   - Parse the task description for project goals
   - Identify key features and components
   - Determine technical stack implications

2. **Design Workflow**
   - Choose or create an appropriate board template
   - Define columns for the development workflow
   - Set up state transitions and escalation paths
   - Assign appropriate agents to columns

3. **Configure Project**
   - Set up project scripts (setup, dev, cleanup)
   - Configure copy files for environment
   - Set agent working directory

4. **Scaffold Repository**
   - Create docs/ directory structure
   - Initialize docs/adr/ for architecture decisions
   - Initialize docs/impl/ for implementation plans
   - Create initial ADRs documenting decisions

5. **Generate Tasks**
   - Break down project into phases
   - Create tasks with clear descriptions
   - Set up triggers for automated workflows
   - Prioritize the backlog

## Output Format

After completing setup, provide a summary:
- Board configuration
- Columns and their agents
- Number of tasks created
- Key ADRs written
- Recommended next steps

## Important Notes

- Ask clarifying questions if requirements are ambiguous
- Prefer existing board templates when appropriate
- Create tasks that are actionable and well-scoped
- Document all architectural decisions in ADRs
```

### 2.3 Research Board Template

Create a minimal board template for new projects:

```sql
-- Research Board Template
INSERT INTO kanban_boards (id, name, description, created_at, updated_at)
VALUES ('research-board-uuid', 'Research Template', 'Minimal board for project bootstrapping', ...);

-- Columns
INSERT INTO kanban_columns (id, board_id, name, slug, status, is_initial, is_terminal, starts_workflow, position, agent_id)
VALUES
    ('col-backlog', 'research-board-uuid', 'Backlog', 'backlog', 'todo', true, false, false, 0, NULL),
    ('col-research', 'research-board-uuid', 'Research', 'research', 'inprogress', false, false, true, 1, 'research-bot-uuid'),
    ('col-complete', 'research-board-uuid', 'Complete', 'complete', 'done', false, true, false, 2, NULL),
    ('col-cancelled', 'research-board-uuid', 'Cancelled', 'cancelled', 'cancelled', false, true, false, 3, NULL);

-- Transitions
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name)
VALUES
    ('trans-1', 'research-board-uuid', 'col-backlog', 'col-research', 'Start Research'),
    ('trans-2', 'research-board-uuid', 'col-research', 'col-complete', 'Complete'),
    ('trans-3', 'research-board-uuid', 'col-research', 'col-cancelled', 'Cancel'),
    ('trans-4', 'research-board-uuid', 'col-backlog', 'col-cancelled', 'Cancel');
```

### Implementation Tasks

- [ ] Create Research Bot agent migration
- [ ] Write Research Bot system prompt
- [ ] Create Research Board template migration
- [ ] Add column/transition seeds
- [ ] Test Research Bot agent execution
- [ ] Refine system prompt based on testing

---

## Phase 3: Project Initialization Flow

**Goal:** Streamline new project creation to auto-bootstrap with Research Bot.

### 3.1 "New Functional Project" Option

Add a new project creation mode in the UI:

```typescript
// frontend/src/components/dialogs/NewProjectDialog.tsx

interface NewProjectDialogProps {
  mode: 'standard' | 'functional';
}

// Functional mode:
// 1. User enters project name
// 2. User enters project description/requirements (becomes Task 1)
// 3. System creates project with Research board
// 4. System creates Task 1 with user's requirements
// 5. System auto-starts Task 1 (moves to Research column)
```

### 3.2 Backend: Functional Project Endpoint

```rust
// POST /api/projects/functional
struct CreateFunctionalProjectRequest {
    name: String,
    requirements: String,  // Becomes Task 1 description
    repo_url: Option<String>,
    repo_path: Option<String>,
}

// Response includes:
// - project_id
// - task_1_id
// - workspace_session_id (auto-started)
```

### 3.3 Auto-Start Logic

```rust
// In project creation handler:
async fn create_functional_project(req: CreateFunctionalProjectRequest) {
    // 1. Create project with Research board template
    let project = create_project_with_board(req.name, RESEARCH_BOARD_ID);

    // 2. Create Task 1 with requirements
    let task = create_task(CreateTask {
        project_id: project.id,
        title: "Project Setup & Research",
        description: req.requirements,
        column_id: BACKLOG_COLUMN_ID,
    });

    // 3. Move to Research column (triggers agent)
    move_task_to_column(task.id, RESEARCH_COLUMN_ID);

    // 4. Auto-start workspace session
    let session = start_workspace_session(task.id, CLAUDE_CODE_EXECUTOR);

    return FunctionalProjectResponse { project, task, session };
}
```

### Implementation Tasks

- [ ] Design "New Functional Project" UI flow
- [ ] Create `POST /api/projects/functional` endpoint
- [ ] Implement auto-start logic
- [ ] Add Research board template selection
- [ ] Test end-to-end flow
- [ ] Add loading/progress indicators

---

## Phase 4: Testing & Refinement

### 4.1 Test Scenarios

1. **Simple project**: "Build a CLI todo app in Rust"
2. **Web app**: "Build a real-time chat application with React and WebSockets"
3. **Complex project**: "Build a multi-tenant SaaS platform with billing and team management"
4. **Existing repo**: Clone and bootstrap an existing GitHub repo

### 4.2 Metrics to Track

- Time from prompt to first task ready
- Number of clarifying questions asked
- Quality of generated board/workflow
- Task breakdown granularity
- ADR completeness

### 4.3 Iteration Points

- System prompt refinement based on output quality
- Board template library expansion
- Task breakdown heuristics
- Integration with external tools (GitHub, Jira, etc.)

---

## File Changes Summary

| File | Changes |
|------|---------|
| `crates/server/src/mcp/task_server.rs` | Add 10+ new MCP tools |
| `crates/db/migrations/YYYYMMDD_seed_research_bot.sql` | Research Bot agent |
| `crates/db/migrations/YYYYMMDD_seed_research_board.sql` | Research board template |
| `crates/server/src/routes/projects.rs` | Add functional project endpoint |
| `frontend/src/components/dialogs/NewProjectDialog.tsx` | Functional project UI |
| `shared/types.ts` | Generated types for new endpoints |

---

## Dependencies

- Phase 1 must complete before Phase 2 (MCP tools needed for Research Bot)
- Phase 2 must complete before Phase 3 (Research Bot needed for auto-start)
- Phase 4 runs continuously as we iterate

---

## Estimated Effort

| Phase | Complexity | Estimate |
|-------|------------|----------|
| Phase 1: MCP Extensions | Medium | Core work - 10+ tool implementations |
| Phase 2: Research Bot | Medium | Agent config + system prompt tuning |
| Phase 3: Init Flow | Low | UI + endpoint wiring |
| Phase 4: Testing | Ongoing | Continuous refinement |

---

## Next Steps

1. Start Phase 1: Implement `create_project` and `update_project` MCP tools
2. Test tools via Claude Code manually
3. Proceed to board/column/transition tools
4. Create Research Bot agent once tools are ready
