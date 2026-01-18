# ADR 2026-01-18-002: Task Auto-start Triggers

## Status
Proposed

## Context
Tasks often have logical dependencies: "start the dashboard task after auth is complete." Currently, users must manually start dependent tasks or remember to do so. This leads to:

1. **Manual overhead**: Watching for task completion to start the next one
2. **Missed handoffs**: Forgetting to start dependent tasks
3. **No workflow automation**: Can't build task chains

Note: This is NOT a hard dependency system. Users can still manually start any task at any time.

## Decision
Implement "soft" auto-start triggers on tasks:

### 1. Trigger Model
```rust
struct TaskTrigger {
    id: Uuid,
    task_id: Uuid,              // The task that will auto-start
    trigger_task_id: Uuid,      // The task to watch for completion
    trigger_on: TriggerCondition,
    created_at: DateTime<Utc>,
}

enum TriggerCondition {
    Completed,                  // Any successful completion
    CompletedWithStatus(String), // Specific status (e.g., "approved")
    Merged,                     // PR merged
}
```

### 2. Behavior
- When `trigger_task_id` completes matching `trigger_on`, automatically start `task_id`
- "Start" means: move to workflow-starting column, begin agent execution
- If user manually starts a task with pending triggers, show warning: "Task X is still running. Start anyway?"
- Triggers are one-shot (removed after firing) or persistent (configurable)

### 3. Schema Changes
```sql
CREATE TABLE task_triggers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    trigger_task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    trigger_on TEXT NOT NULL DEFAULT 'completed',
    is_persistent BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(task_id, trigger_task_id)
);
```

### 4. UI Integration
- Task detail view: "Start after..." dropdown to select trigger task
- Task list: indicator showing "Waiting for: Task X"
- When starting manually with active trigger: confirmation dialog

### 5. Execution Flow
```
1. Task A completes
2. System queries: SELECT * FROM task_triggers WHERE trigger_task_id = A.id
3. For each matching trigger:
   a. If trigger_on matches completion status
   b. Auto-start the dependent task
   c. If not persistent, delete the trigger
4. Record event in task_events
```

## Consequences

### Positive
- Enables workflow automation without hard blocking
- Users retain full control (can override)
- Simple mental model: "start this after that"

### Negative
- Adds complexity to task completion flow
- Could create unintended chains if misconfigured
- Need clear UI to show what's waiting on what

### Neutral
- Does not prevent parallel execution (that's what file locks are for)
- Works alongside manual task management

## Implementation Notes
1. Add task_triggers table
2. Hook into task completion flow (status change to Done)
3. Add trigger management to task detail UI
4. Add "waiting for" indicator to task cards
5. Consider: circular trigger detection (A triggers B triggers A)

## Related
- ADR 2026-01-18-001: Structured Deliverables
- ADR 2026-01-18-003: Agent File Locking
