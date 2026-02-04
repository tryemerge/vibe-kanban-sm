# ADR 006: Functional Application Design

**Date:** 2026-01-26
**Status:** Draft
**Author:** User + Claude

## Context

The current vibe-kanban workflow requires manual setup of projects, boards, columns, agents, and tasks. Users must understand the system's architecture before they can be productive. We want a "functional" approach where users simply describe what they want to build, and the system bootstraps everything automatically.

## Decision

Implement a **Functional Application Design** workflow where a single prompt triggers an intelligent bootstrapping process that sets up an entire development environment.

The key insight: **This is just Task 1 running Claude Code with the existing MCP server**. We don't need a new agent type - we need to expand the MCP server's capabilities.

## Workflow Overview

### 1. Project Initiation
- User creates a new project with a single prompt describing what they want to build
- This prompt becomes **Task 1** in the backlog
- Example: "Build a real-time collaborative document editor with version history and comments"

### 2. Research Bot Column
The board has a minimal initial structure:
- **Backlog** (initial)
- **Research** (triggers Claude Code with research/setup instructions)
- **Complete** (terminal)
- **Cancelled** (terminal)

When Task 1 moves to Research, Claude Code activates with access to the vibe-kanban MCP server.

### 3. Task 1 (Claude Code) Responsibilities

Claude Code uses MCP tools to:

#### a) Board & Workflow Design
- Analyze the project requirements from Task 1 description
- Select an existing board template OR create a new board
- Define columns with appropriate statuses (todo, inprogress, inreview, done)
- Configure state transitions and escalation paths
- Set up conditional routing (e.g., review → approved/rejected)

#### b) Agent Assignment
- Assign agents to workflow columns
- Configure column-specific agent settings

#### c) Project Configuration
- Update project settings (board, scripts, copy files)
- Set agent working directory
- Configure setup/cleanup scripts

#### d) Repository Scaffolding
- Initialize repo structure using standard tools (Write, Bash)
- Create `/docs` directory for documentation
- Set up `/docs/adr/` directory for architecture decisions
- Set up `/docs/impl/` directory for implementation plans

#### e) Initial Documentation
- Create foundational ADRs
- Create implementation plans

#### f) Task Generation
- Break down the project into actionable tasks using `create_task`
- Set up task dependencies and triggers

## MCP Server Gap Analysis

### Current MCP Tools ✅

| Tool | Description |
|------|-------------|
| `get_context` | Get workspace/project/task context |
| `create_task` | Create task with labels |
| `list_projects` | List all projects |
| `list_repos` | List repos for a project |
| `list_tasks` | List tasks with filtering |
| `get_task` | Get task details |
| `start_workspace_session` | Launch a task attempt |
| `update_task` | Update task title/description/status |
| `delete_task` | Delete a task |

### Missing MCP Tools 🔴

| Tool | Priority | Description |
|------|----------|-------------|
| `create_project` | 🔴 High | Create a new project |
| `update_project` | 🔴 High | Update project settings (board, scripts, agent_working_dir) |
| `list_boards` | 🔴 High | List available board templates |
| `create_board` | 🔴 High | Create a new board template |
| `get_board` | 🟡 Medium | Get board with columns and transitions |
| `create_column` | 🔴 High | Add column to a board |
| `update_column` | 🟡 Medium | Update column settings (agent, status) |
| `delete_column` | 🟢 Low | Remove column from board |
| `create_transition` | 🔴 High | Define state transition between columns |
| `delete_transition` | 🟢 Low | Remove transition |
| `list_agents` | 🔴 High | List available agents and their capabilities |
| `assign_board_to_project` | 🔴 High | Set a project's board |
| `create_task_trigger` | 🟡 Medium | Add trigger to a task |

### API Routes That Already Exist

Most of these operations already have REST API routes - we just need MCP wrappers:

| Route | Method | MCP Tool Needed |
|-------|--------|-----------------|
| `/api/projects` | POST | `create_project` |
| `/api/projects/:id` | PUT | `update_project` |
| `/api/boards` | GET | `list_boards` |
| `/api/boards` | POST | `create_board` |
| `/api/boards/:id` | GET | `get_board` |
| `/api/boards/:id/columns` | POST | `create_column` |
| `/api/boards/:id/columns/:id` | PUT | `update_column` |
| `/api/boards/:id/transitions` | POST | `create_transition` |
| `/api/agents` | GET | `list_agents` |
| `/api/tasks/:id/triggers` | POST | `create_task_trigger` |

## Technical Implementation

### Phase 1: MCP Server Extensions
Add the missing MCP tools as wrappers around existing REST APIs.

### Phase 2: Research Board Template
Create a default "Research" board template with:
- Backlog (initial, todo)
- Research (inprogress, agent: Claude Code with research prompt)
- Complete (terminal, done)
- Cancelled (terminal, cancelled)

### Phase 3: Research Agent Configuration
Configure Claude Code for the Research column with:
- System prompt for project bootstrapping
- Access to all MCP tools
- Instructions for creating boards, tasks, docs

### Phase 4: Project Initialization Flow
When user creates project:
1. Apply Research board template
2. Create Task 1 with user's prompt
3. Auto-start Task 1 in Research column

## Open Questions

1. Should Task 1 ask clarifying questions interactively or make assumptions?
2. How much board customization should happen vs using templates?
3. Should we have a "dry run" mode that shows what would be created?

## References

- [MCP Task Server Implementation](../../crates/server/src/mcp/task_server.rs)
- [Board Settings UI](../../frontend/src/pages/settings/BoardSettings.tsx)
- [Task Triggers Model](../../crates/db/src/models/task_trigger.rs)
- [Implementation Plan](../impl/006-functional-application-design.md)
