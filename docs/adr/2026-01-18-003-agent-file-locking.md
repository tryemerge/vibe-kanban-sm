# ADR 2026-01-18-003: Agent File Locking

## Status
Proposed

## Context
When multiple tasks run in parallel on the same repository, they may modify the same files. While git worktrees provide branch isolation, merge conflicts at PR time create rework:

1. **Conflict at merge**: Two tasks modify the same file, one must be rebased
2. **Context drift**: Task B's changes assume Task A's code, but A isn't merged yet
3. **Wasted work**: May need to redo work after conflict resolution

File locking allows agents to claim files they're about to modify, preventing parallel conflicting changes.

## Decision
Implement agent-managed file locking with automatic release:

### 1. Lock Model
```rust
struct FileLock {
    id: Uuid,
    project_id: Uuid,
    file_path: String,           // Relative to repo root, supports glob patterns
    task_id: Uuid,
    workspace_id: Uuid,
    acquired_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,  // Optional TTL
}
```

### 2. Lock Semantics
- **Acquire**: Agent requests lock before modifying file(s)
- **Check**: Before writing, agent checks if file is locked by another task
- **Wait**: If locked, agent enters wait mode (poll or notification)
- **Release**: Automatic on task completion (success/fail/cancel)
- **Scope**: Per-project, not per-repo (same file in multiple repos = different locks)

### 3. Glob Pattern Support
```
src/auth/*        - Lock all files in auth directory
src/types/*.ts    - Lock all .ts files in types
**/*.config.js    - Lock all config files anywhere
```

### 4. Schema Changes
```sql
CREATE TABLE file_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    acquired_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    UNIQUE(project_id, file_path)
);

-- Index for fast conflict checking
CREATE INDEX idx_file_locks_project_path ON file_locks(project_id, file_path);
```

### 5. MCP Tools for Agents
```typescript
// Acquire locks (fails if any already locked)
acquire_file_locks({
  paths: ["src/auth/*", "src/types/user.ts"],
  wait_if_locked: boolean,  // true = block until available
  timeout_ms?: number       // max wait time
})

// Check lock status without acquiring
check_file_locks({
  paths: ["src/auth/login.ts"]
}) -> { locked: boolean, holder_task_id?: string, holder_task_title?: string }

// Release locks early (normally automatic)
release_file_locks({
  paths: ["src/auth/*"]
})
```

### 6. Conflict Resolution Flow
```
Agent B wants to modify src/auth/login.ts:
1. Calls acquire_file_locks(["src/auth/login.ts"], wait=true)
2. Lock exists for Task A → Agent B enters wait mode
3. Task A completes → Locks auto-released
4. Agent B woken → Lock acquired → Proceeds with work
```

### 7. Automatic Release Triggers
- Task status changes to Done/Cancelled
- Workspace is deleted
- Workspace is cancelled
- TTL expires (if set)

## Consequences

### Positive
- Prevents conflicting parallel modifications
- Agents self-coordinate without user intervention
- Clear feedback: "Waiting for Task A to release src/auth/*"

### Negative
- Risk of deadlock (A locks X, waits for Y; B locks Y, waits for X)
- Overly broad locks reduce parallelism
- Stale locks from crashed agents (mitigated by TTL)

### Neutral
- Opt-in: Agents must explicitly acquire locks
- Doesn't prevent all merge conflicts (only those on locked paths)
- Works with existing git worktree isolation

## Implementation Notes
1. Add file_locks table with cleanup triggers
2. Implement MCP tools in task_server
3. Add deadlock detection (cycle in lock wait graph)
4. Add lock status to workspace/task UI
5. Consider: implicit locking (auto-lock on file write) vs explicit (agent calls acquire)
6. Consider: lock inheritance (child tasks inherit parent's locks)

## Deadlock Prevention
```rust
// Before acquiring, check for cycles
fn would_create_deadlock(task_id: Uuid, requested_paths: &[String]) -> bool {
    // Build graph: task -> tasks it's waiting for
    // Check if adding this edge creates a cycle
}
```

## Related
- ADR 2026-01-18-001: Structured Deliverables
- ADR 2026-01-18-002: Task Auto-start Triggers
