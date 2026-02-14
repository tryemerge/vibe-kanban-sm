-- Setup Board: Default starting board for all new projects
-- This board enables single-prompt project bootstrapping via Claude Code

-- ============================================
-- RESEARCH BOT AGENT
-- ============================================

-- Research Bot Agent - Claude Code configured for project research and setup
INSERT INTO agents (id, name, role, system_prompt, description, start_command, color, executor, is_template, template_group_id)
VALUES (
    '33333333-0000-0001-0006-000000000001',
    'Research Bot',
    'Project Bootstrapper',
    E'You are a Research Bot responsible for analyzing project requirements and setting up development infrastructure. You have access to the vibe-kanban MCP server tools to manage boards, columns, transitions, and tasks.

## Your Mission

When a user creates a new project with a description, you:
1. Analyze the requirements
2. Research the codebase (if existing repo)
3. Design an appropriate workflow board
4. Create initial tasks
5. Set up project configuration

## Available MCP Tools

You have access to these vibe-kanban MCP tools:
- `list_boards`, `create_board`, `get_board` - Manage board templates
- `create_column` - Add columns to boards
- `create_transition` - Define state transitions
- `list_agents` - See available agents to assign to columns
- `create_task` - Create tasks in the project
- `update_project` - Configure project settings

## Workflow Design Process

### 1. Analyze Requirements
- What type of project is this? (feature, bug fix, research, refactor)
- What are the key deliverables?
- What review/approval process is needed?

### 2. Design Board Structure
For simple projects:
- Backlog → Implementation → Review → Done

For complex projects with research:
- Backlog → Research → Planning → Implementation → Test → Review → Done

For bug fixes:
- Backlog → Reproduce → Fix → Test → Review → Done

### 3. Configure Agents
Assign appropriate agents to columns:
- Research columns → Research agents
- Implementation columns → Claude Code (default)
- Review columns → Review agents (Security, Performance, etc.)

### 4. Create Initial Tasks
Break down the project into actionable tasks with clear descriptions.

## Decision Output

When research is complete, you can either:

### Option A: Complete the task (research finished the work)
```json
{
  "decision": "complete",
  "summary": "Project infrastructure set up with X board, Y columns, Z initial tasks"
}
```

### Option B: Pass to implementation
```json
{
  "decision": "ready",
  "summary": "Research complete, board and tasks created, ready for implementation"
}
```

## Philosophy

**Make the first task do the heavy lifting.**

Good project setup prevents thrashing later. Invest time upfront to understand requirements, design appropriate workflows, and create clear tasks.',
    'Analyzes project requirements and sets up development infrastructure using MCP tools',
    'Analyze the project requirements. Use MCP tools to design an appropriate workflow board, configure agents, and create initial tasks. Complete when project infrastructure is ready.',
    '#7c3aed',
    'CLAUDE_CODE',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- ============================================
-- SETUP BOARD
-- ============================================

-- Setup Board template - default for all new projects
INSERT INTO boards (id, name, description, is_template, template_group_id, template_name, template_description, template_icon)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    'Setup Board',
    'Default starting board for new projects. The Research column analyzes requirements and bootstraps project infrastructure.',
    TRUE,
    '33333333-3333-3333-3333-333333333333',
    'Setup Starter',
    'Default board for new projects. Research Bot analyzes requirements and creates custom workflows.',
    'Sparkles'
)
ON CONFLICT DO NOTHING;

-- ============================================
-- SETUP BOARD COLUMNS (Research column uses Research Bot)
-- ============================================

-- Backlog - Initial column where tasks start
INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-000000000001',
    '00000000-0000-0000-0000-000000000002',
    'Backlog',
    'backlog',
    0,
    '#6b7280',
    TRUE,
    FALSE,
    FALSE,
    'todo',
    NULL,
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Research - Where Research Bot analyzes and sets up project
INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, deliverable, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-000000000002',
    '00000000-0000-0000-0000-000000000002',
    'Research',
    'research',
    1,
    '#7c3aed',
    FALSE,
    FALSE,
    TRUE,
    'inprogress',
    '33333333-0000-0001-0006-000000000001',
    'Project analysis and infrastructure setup. Deliverables:
- Board configuration (if custom workflow needed)
- Initial task breakdown
- Project settings (agent working dir, scripts)

Output decision.json with:
- decision: "complete" (if research finished the work)
- decision: "ready" (if passing to implementation)',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Complete - Terminal column for finished tasks
INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-000000000003',
    '00000000-0000-0000-0000-000000000002',
    'Complete',
    'complete',
    2,
    '#22c55e',
    FALSE,
    TRUE,
    FALSE,
    'done',
    NULL,
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Cancelled - Terminal column for cancelled tasks
INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-000000000004',
    '00000000-0000-0000-0000-000000000002',
    'Cancelled',
    'cancelled',
    3,
    '#ef4444',
    FALSE,
    TRUE,
    FALSE,
    'cancelled',
    NULL,
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- ============================================
-- SETUP BOARD TRANSITIONS
-- ============================================

-- Backlog → Research (start research)
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-100000000001',
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0002-000000000001',
    '00000000-0000-0000-0002-000000000002',
    'Start Research',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Backlog → Complete (skip research for simple tasks)
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-100000000002',
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0002-000000000001',
    '00000000-0000-0000-0002-000000000003',
    'Skip to Complete',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Research → Complete (research finished the task)
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-100000000003',
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0002-000000000002',
    '00000000-0000-0000-0002-000000000003',
    'Complete',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Research → Backlog (needs more info)
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-100000000004',
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0002-000000000002',
    '00000000-0000-0000-0002-000000000001',
    'Back to Backlog',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Backlog → Cancelled
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-100000000005',
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0002-000000000001',
    '00000000-0000-0000-0002-000000000004',
    'Cancel',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;

-- Research → Cancelled
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES (
    '00000000-0000-0000-0002-100000000006',
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0002-000000000002',
    '00000000-0000-0000-0002-000000000004',
    'Cancel',
    TRUE,
    '33333333-3333-3333-3333-333333333333'
)
ON CONFLICT DO NOTHING;
