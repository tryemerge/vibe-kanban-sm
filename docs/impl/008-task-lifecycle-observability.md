# Implementation Plan: Task Lifecycle Observability (ADR-008)

## Overview

Wire the existing TaskEventTimeline into the UI, add a context preview endpoint, build a TaskContextPanel, and add a project-level Knowledge Base settings page.

## Existing Assets (reuse)

| Asset | Location |
|-------|----------|
| TaskEventTimeline component | `frontend/src/components/tasks/TaskDetails/TaskEventTimeline.tsx` |
| useTaskEvents hook | `frontend/src/hooks/useTaskEvents.ts` |
| taskEventsApi | `frontend/src/lib/api.ts:1670` |
| Context artifact REST API | `crates/server/src/routes/context_artifacts.rs` |
| build_full_context | `crates/db/src/models/context_artifact.rs:525` |
| ContextArtifact TS type | `shared/types.ts` |

## Steps

### 1. Backend: Add preview-context endpoint

**File:** `crates/server/src/routes/context_artifacts.rs`

Add `PreviewContextQuery` struct and `preview_context()` handler:

```rust
struct PreviewContextQuery {
    project_id: Uuid,
    task_id: Option<Uuid>,
}

struct ContextPreviewResponse {
    context: String,
    tokens_used: i32,
    token_budget: i32,
    artifacts_included: i32,
    artifacts_total: i32,
}
```

Calls `ContextArtifact::build_full_context()` with the provided project/task IDs. To get the metadata (tokens_used, artifacts counts), modify `build_full_context()` to return a struct instead of just a String, or add a parallel method `build_full_context_with_stats()`.

Register route: `.route("/preview-context", get(preview_context))`

### 2. Frontend: Add contextArtifactsApi

**File:** `frontend/src/lib/api.ts`

```typescript
export const contextArtifactsApi = {
  list: async (projectId: string, artifactType?: string): Promise<ContextArtifact[]> => { ... },
  delete: async (artifactId: string): Promise<void> => { ... },
  previewContext: async (projectId: string, taskId?: string): Promise<ContextPreviewResponse> => { ... },
};
```

### 3. Frontend: Add useContextArtifacts hook

**File:** `frontend/src/hooks/useContextArtifacts.ts` (new)

React Query hooks:
- `useContextArtifacts(projectId, artifactType?)` — list artifacts
- `useContextPreview(projectId, taskId?)` — preview assembled context

### 4. Frontend: Add tabs to TaskPanel

**File:** `frontend/src/components/panels/TaskPanel.tsx`

Wrap existing content in shadcn `<Tabs>`:
- **Overview** tab — existing content unchanged
- **Timeline** tab — `<TaskEventTimeline taskId={taskId} />`
- **Context** tab — `<TaskContextPanel taskId={taskId} projectId={projectId} />`

### 5. Frontend: Build TaskContextPanel

**File:** `frontend/src/components/tasks/TaskDetails/TaskContextPanel.tsx` (new)

**Section A: "Produced by this task"**
- Filter `contextArtifactsApi.list()` where `source_task_id === taskId`
- Each artifact: type badge, title, scope badge, token estimate, timestamp
- Expandable content preview
- Empty state: "This task hasn't produced any artifacts yet"

**Section B: "Context injected"**
- Call `contextArtifactsApi.previewContext(projectId, taskId)`
- Show token usage bar: "3,200 / 8,000 tokens"
- Render assembled markdown in a code block or prose view
- Empty state: "No context artifacts exist for this project yet"

### 6. Frontend: Knowledge Base settings page

**File:** `frontend/src/pages/settings/KnowledgeBaseSettings.tsx` (new)

- Summary header: artifact count + total token estimate
- Filter row: type dropdown + scope dropdown
- Table with columns: Type, Title, Scope, Source Task, Tokens, Version, Updated
- Expandable rows showing full content
- Delete button per artifact
- Uses `useContextArtifacts(projectId, selectedType)` hook

**File:** `frontend/src/pages/settings/SettingsLayout.tsx`

- Add "Knowledge Base" nav item with Brain icon
- Add route for the new page

## Verification

1. `cargo check --workspace` — backend compiles
2. `pnpm run check` — frontend type checks
3. Navigate to task → see Overview/Timeline/Context tabs
4. Timeline tab shows event history (agent starts, transitions, artifacts)
5. Context tab shows artifacts from this task + assembled context preview with token budget
6. Settings → Knowledge Base shows all project artifacts with filters
7. Create artifact via MCP `create_artifact` tool → appears in both views
