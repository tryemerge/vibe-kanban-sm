# IMPL-016: Persistent Project Agent

## Problem

Users need a way to chat with an AI agent at the project level to manage tasks, update groups, ask questions about the codebase, and capture project knowledge. Currently, agents only exist at the task level (column agents) or group level (Group Evaluator, PreReq Evaluator). There's no persistent, project-scoped agent the user can interact with directly.

## Design

### Core Concept

One persistent chat agent per project, accessible as a side panel on the project tasks page. The agent has MCP tools for project management (task CRUD, group management, artifact creation) and can read the codebase. It does NOT write code.

### Key Design Decisions

- **Lazy creation**: Agent workspace created on first use, not on project creation
- **Persistent via `--session-id`**: Conversation continues across browser sessions using Claude Code's built-in session resumption
- **System task pattern**: Workspace requires `task_id` (NOT NULL). Use a hidden system task filtered from UI by title regex (same pattern as Task Grouper)
- **Lightweight workspace**: `container_ref` pre-set to project repo path, no git worktree (same pattern as Group Evaluator / PreReq Evaluator)
- **Reuse existing infrastructure**: Follow-up API, streaming logs, chat input UI all already exist and are workspace-agnostic

### Architecture

```
Project
  └─ agent_workspace_id (FK, nullable, lazy)
       └─ Workspace (lightweight, container_ref = project repo)
            └─ System Task ("Project Agent — {name}", hidden from UI)
            └─ Session
                 └─ ExecutionProcess (one per agent turn)
                      └─ CodingAgentTurn (agent_session_id for resumption)
```

### What Already Exists (Reused As-Is)

| Primitive | Location |
|-----------|----------|
| Agent launch pipeline | `container.rs`: `launch_agent_in_workspace` |
| Session continuity (`--session-id`) | `coding_agent_turn.rs`, `coding_agent_follow_up.rs` |
| Follow-up message API | `POST /sessions/{id}/follow-up` |
| Message queue (send while agent runs) | `QueuedMessageService` |
| Streaming log view | `VirtualizedList.tsx`, `useConversationHistory.ts` |
| Chat input UI | `TaskFollowUpSection.tsx` |
| Draft persistence | `useScratch` + backend `Scratch` model |
| MCP tools (30+ tools) | `task_server.rs` |
| Project context injection | `ContextArtifact::build_full_context` |
| Lightweight workspace pattern | `create_lightweight_group_workspace` |

## Implementation

### 1. Database Migration

**File**: `crates/db/migrations/20260303000001_add_project_agent.sql`

```sql
ALTER TABLE projects ADD COLUMN agent_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL;
```

Plus seed a well-known "Project Agent" in the `agents` table:
- UUID: `55555555-0000-0001-0002-000000000001`
- Name: "Project Agent"
- Role: "Project Manager"
- System prompt: MCP tool usage instructions, codebase reading, no code writing
- Executor: `CLAUDE_CODE`
- Color: `#6366f1`

### 2. Project Model Update

**File**: `crates/db/src/models/project.rs`

- Add `agent_workspace_id: Option<Uuid>` to `Project` struct
- Update all `query_as!` macros (SELECT, RETURNING) to include the column
- Add `set_agent_workspace_id(pool, project_id, workspace_id)` method

### 3. Project Agent Service

**New file**: `crates/services/src/services/project_agent.rs`

```rust
pub const PROJECT_AGENT_ID: Uuid = uuid::uuid!("55555555-0000-0001-0002-000000000001");

pub fn build_initial_prompt(project: &Project) -> String {
    // Tells agent it's the project manager, working directory is the repo, wait for user input
}
```

### 4. Backend Endpoint

**File**: `crates/server/src/routes/projects.rs`

`POST /projects/{project_id}/agent/start` (idempotent):

1. If `project.agent_workspace_id` is set → return existing `workspace_id`
2. Otherwise:
   - Create hidden system task: `"Project Agent — {project.name}"`
   - Create lightweight workspace (`container_ref` = project repo path)
   - Set `project.agent_workspace_id = workspace_id`
   - Launch agent via `launch_agent_in_workspace`
   - Return `{ workspace_id, created: true }`

Follows the `analyze_backlog` pattern in `task_groups.rs`.

### 5. Workspace Cleanup Exclusion

**File**: `crates/db/src/models/workspace.rs`

Exclude agent workspaces from the cleanup query:
```sql
AND w.id NOT IN (SELECT agent_workspace_id FROM projects WHERE agent_workspace_id IS NOT NULL)
```

### 6. Frontend: API Client

**File**: `frontend/src/lib/api.ts`

```typescript
projectsApi.startAgent(projectId): Promise<{ workspace_id: string; created: boolean }>
```

### 7. Frontend: ProjectAgentPanel Component

**New file**: `frontend/src/components/panels/ProjectAgentPanel.tsx`

Thin wrapper around `TaskAttemptPanel`:
- Takes `projectId` and `workspaceId`
- Uses `useTaskAttemptWithSession(workspaceId)` for workspace/session
- Fetches system task via `attempt.task_id` for follow-up section
- Renders logs + follow-up input (no timeline/context tabs)

### 8. Frontend: ProjectTasks Integration

**File**: `frontend/src/pages/ProjectTasks.tsx`

- State: `isAgentPanelOpen`, `agentWorkspaceId` (seeded from `project.agent_workspace_id`)
- Toggle button to open/close panel
- On first open: calls `startAgent()`, stores workspace_id
- Renders `ProjectAgentPanel` as 400px right-side panel
- Extends system task filter: `!/^Project Agent — /i.test(t.title)`

### 9. Type Generation

```bash
pnpm run prepare-db      # SQLx cache for new migration + queries
pnpm run generate-types  # shared/types.ts gets agent_workspace_id on Project
```

## Files

| File | Change |
|------|--------|
| `crates/db/migrations/20260303000001_add_project_agent.sql` | **New** |
| `crates/db/src/models/project.rs` | Add field, update queries |
| `crates/db/src/models/workspace.rs` | Exclude from cleanup |
| `crates/services/src/services/project_agent.rs` | **New** |
| `crates/services/src/services/mod.rs` | Register module |
| `crates/server/src/routes/projects.rs` | Add endpoint |
| `frontend/src/lib/api.ts` | Add `startAgent` |
| `frontend/src/components/panels/ProjectAgentPanel.tsx` | **New** |
| `frontend/src/pages/ProjectTasks.tsx` | Panel integration |

## MCP Context Resolution

The MCP server resolves `project_id` by looking up `container_ref` → workspace → task → project. The Project Agent's system task has `project_id` set correctly, so MCP context resolution works out of the box. No MCP changes needed.

## Conversation Flow

```
User clicks "Chat" button
  → POST /projects/{id}/agent/start (idempotent)
  → Returns workspace_id
  → Frontend opens panel, streams workspace session

User types message
  → POST /sessions/{session_id}/follow-up { prompt }
  → Backend finds latest agent_session_id from CodingAgentTurn
  → Launches Claude Code with --session-id (conversation continues)
  → Response streams to VirtualizedList

User closes browser, comes back later
  → project.agent_workspace_id still points to same workspace
  → Same session, same agent_session_id → conversation resumes
```

## Verification

1. `pnpm run prepare-db` — migration applies
2. `pnpm run check` — TS + Rust compile
3. Open project → click Chat → panel opens, agent initializes
4. Send "List all tasks" → agent uses `list_tasks` MCP tool
5. Send "Create a task called 'Add login page'" → task appears in kanban
6. Close panel, reopen → conversation continues
7. Reload page → workspace reconnects from `agent_workspace_id`
8. System task hidden from task list and ungrouped sidebar
