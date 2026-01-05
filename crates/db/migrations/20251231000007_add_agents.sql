-- Agents: Configurable AI coding agents
-- Ported from vibe-factory to enable agent configuration and automation

CREATE TABLE agents (
    id              BLOB PRIMARY KEY,
    name            TEXT NOT NULL,
    role            TEXT NOT NULL,              -- e.g., "Developer", "Reviewer", "Architect"
    system_prompt   TEXT NOT NULL,              -- The agent's system prompt/instructions
    capabilities    TEXT,                       -- JSON array of capabilities
    tools           TEXT,                       -- JSON array of allowed tools
    description     TEXT,                       -- Human-readable description
    context_files   TEXT,                       -- JSON array of file paths to include as context
    executor        TEXT NOT NULL DEFAULT 'CLAUDE_CODE'
                        CHECK (executor IN ('CLAUDE_CODE', 'CODEX', 'GEMINI', 'CURSOR_AGENT', 'OPENCODE')),
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Index for listing agents
CREATE INDEX idx_agents_name ON agents(name);
CREATE INDEX idx_agents_executor ON agents(executor);
