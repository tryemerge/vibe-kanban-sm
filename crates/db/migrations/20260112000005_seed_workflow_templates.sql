-- Template group IDs (deterministic UUIDs for each template)
-- code-review: 11111111-1111-1111-1111-111111111111
-- simple-kanban: 22222222-2222-2222-2222-222222222222

-- ============================================
-- CODE REVIEW PIPELINE TEMPLATE
-- ============================================

-- Template Board
INSERT INTO boards (id, name, description, is_template, template_group_id, template_name, template_description, template_icon)
VALUES (
    '11111111-0000-0000-0000-000000000001',
    'Code Review Pipeline Template',
    NULL,
    TRUE,
    '11111111-1111-1111-1111-111111111111',
    'Code Review Pipeline',
    'Developer → Reviewer flow with approve/reject routing',
    'GitPullRequest'
);

-- Template Agents
INSERT INTO agents (id, name, role, system_prompt, start_command, color, executor, is_template, template_group_id)
VALUES
    ('11111111-0000-0000-0001-000000000001', 'Developer', 'Software Developer',
     'You are a skilled software developer. Write clean, well-tested code that follows best practices. Focus on implementing the requirements correctly and writing comprehensive tests.',
     'Implement the task requirements. Write tests to verify your implementation. Commit your changes when complete.',
     '#3b82f6', 'CLAUDE_CODE', TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0001-000000000002', 'Code Reviewer', 'Senior Code Reviewer',
     'You are a thorough code reviewer. Review code for correctness, security, performance, and best practices. Provide constructive feedback.',
     'Review the code changes in this worktree. Check for bugs, security issues, and best practices. Write your decision to .vibe/decision.json: {"decision": "approve"} or {"decision": "reject", "feedback": "reason"}',
     '#8b5cf6', 'CLAUDE_CODE', TRUE, '11111111-1111-1111-1111-111111111111');

-- Template Columns (agent_id references template agents)
INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, deliverable, is_template, template_group_id)
VALUES
    ('11111111-0000-0000-0002-000000000001', '11111111-0000-0000-0000-000000000001', 'Backlog', 'backlog', 0, '#6b7280', TRUE, FALSE, FALSE, 'todo', NULL, NULL, TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0002-000000000002', '11111111-0000-0000-0000-000000000001', 'Development', 'development', 1, '#3b82f6', FALSE, FALSE, TRUE, 'in_progress', '11111111-0000-0000-0001-000000000001', 'Working code with tests', TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0002-000000000003', '11111111-0000-0000-0000-000000000001', 'Code Review', 'code-review', 2, '#8b5cf6', FALSE, FALSE, FALSE, 'in_progress', '11111111-0000-0000-0001-000000000002', 'Approval or rejection with feedback', TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0002-000000000004', '11111111-0000-0000-0000-000000000001', 'Done', 'done', 3, '#22c55e', FALSE, TRUE, FALSE, 'done', NULL, NULL, TRUE, '11111111-1111-1111-1111-111111111111');

-- Template Transitions
INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, condition_key, condition_value, is_template, template_group_id)
VALUES
    ('11111111-0000-0000-0003-000000000001', '11111111-0000-0000-0000-000000000001', '11111111-0000-0000-0002-000000000001', '11111111-0000-0000-0002-000000000002', 'Start Development', NULL, NULL, TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0003-000000000002', '11111111-0000-0000-0000-000000000001', '11111111-0000-0000-0002-000000000002', '11111111-0000-0000-0002-000000000003', 'Request Review', NULL, NULL, TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0003-000000000003', '11111111-0000-0000-0000-000000000001', '11111111-0000-0000-0002-000000000003', '11111111-0000-0000-0002-000000000004', 'Approve', 'decision', 'approve', TRUE, '11111111-1111-1111-1111-111111111111'),
    ('11111111-0000-0000-0003-000000000004', '11111111-0000-0000-0000-000000000001', '11111111-0000-0000-0002-000000000003', '11111111-0000-0000-0002-000000000002', 'Request Changes', 'decision', 'reject', TRUE, '11111111-1111-1111-1111-111111111111');

-- ============================================
-- SIMPLE KANBAN TEMPLATE (No agents)
-- ============================================

INSERT INTO boards (id, name, description, is_template, template_group_id, template_name, template_description, template_icon)
VALUES (
    '22222222-0000-0000-0000-000000000001',
    'Simple Kanban Template',
    NULL,
    TRUE,
    '22222222-2222-2222-2222-222222222222',
    'Simple Kanban',
    'Basic To Do → In Progress → Done board',
    'LayoutGrid'
);

INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, is_template, template_group_id)
VALUES
    ('22222222-0000-0000-0002-000000000001', '22222222-0000-0000-0000-000000000001', 'To Do', 'todo', 0, '#6b7280', TRUE, FALSE, FALSE, 'todo', NULL, TRUE, '22222222-2222-2222-2222-222222222222'),
    ('22222222-0000-0000-0002-000000000002', '22222222-0000-0000-0000-000000000001', 'In Progress', 'in-progress', 1, '#3b82f6', FALSE, FALSE, TRUE, 'in_progress', NULL, TRUE, '22222222-2222-2222-2222-222222222222'),
    ('22222222-0000-0000-0002-000000000003', '22222222-0000-0000-0000-000000000001', 'Done', 'done', 2, '#22c55e', FALSE, TRUE, FALSE, 'done', NULL, TRUE, '22222222-2222-2222-2222-222222222222');

INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, name, is_template, template_group_id)
VALUES
    ('22222222-0000-0000-0003-000000000001', '22222222-0000-0000-0000-000000000001', '22222222-0000-0000-0002-000000000001', '22222222-0000-0000-0002-000000000002', 'Start Work', TRUE, '22222222-2222-2222-2222-222222222222'),
    ('22222222-0000-0000-0003-000000000002', '22222222-0000-0000-0000-000000000001', '22222222-0000-0000-0002-000000000002', '22222222-0000-0000-0002-000000000003', 'Complete', TRUE, '22222222-2222-2222-2222-222222222222');
