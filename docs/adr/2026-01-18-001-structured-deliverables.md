# ADR 2026-01-18-001: Structured Deliverables and Transition Builder

## Status
Implemented

## Context
Currently, column deliverables are freeform text fields, and state transitions use manual `condition_key`/`condition_value` pairs. This creates several problems:

1. **No validation**: Agents don't know what values are valid for their output
2. **Error-prone**: Typos in condition values break routing silently
3. **Poor UX**: Users must manually type matching values in both deliverable prompts and transition conditions
4. **No discoverability**: When creating transitions, users don't know what options exist

## Decision
Replace freeform deliverables with a structured system:

### 1. Deliverable Schema (on Columns)
```rust
struct DeliverableDefinition {
    variable_name: String,        // e.g., "decision", "quality_score"
    options: Vec<DeliverableOption>,
    required: bool,
}

struct DeliverableOption {
    value: String,                // e.g., "approve", "reject"
    label: Option<String>,        // Human-friendly label
    description: Option<String>,  // Help text for agent
}
```

### 2. Agent Prompt Integration
When a column has a structured deliverable, the agent system prompt automatically includes:
```
You must set `{variable_name}` to one of: {options.join(", ")}
```

### 3. Transition Builder UX
When creating transitions FROM a column with a structured deliverable:
- UI shows all defined options
- For each option, user selects the target column
- Optional: set max_failures and escalation per option
- Creates multiple transitions in one operation

### 4. Schema Changes
```sql
-- Option A: JSON column on kanban_columns
ALTER TABLE kanban_columns
ADD COLUMN deliverable_schema JSONB;

-- Option B: Separate table for better querying
CREATE TABLE deliverable_options (
    id UUID PRIMARY KEY,
    column_id UUID REFERENCES kanban_columns(id) ON DELETE CASCADE,
    variable_name TEXT NOT NULL,
    option_value TEXT NOT NULL,
    option_label TEXT,
    option_description TEXT,
    sort_order INTEGER DEFAULT 0
);
```

## Consequences

### Positive
- Clear contract between workflow design and agent behavior
- Reduced configuration errors
- Better UX for transition creation
- Enables validation of agent outputs

### Negative
- Migration complexity for existing deliverables
- More rigid than freeform (may not suit all use cases)
- Requires UI work for both column editor and transition builder

### Neutral
- Existing `condition_key`/`condition_value` fields remain for backward compatibility
- Freeform deliverable text can coexist (for instructions vs. structured output)

## Implementation Notes
1. Start with JSON column approach (simpler)
2. Add structured deliverable editor to column settings
3. Modify transition creation to read deliverable options
4. Update agent prompt builder to include deliverable instructions
5. Validation of agent output via auto-retry prompt (see below)

## Agent Prompt Injection

When a column has a `deliverable_variable` with `deliverable_options`, the system automatically injects decision instructions into the agent's prompt via `build_decision_instructions()` in `container.rs`.

### Injected Prompt Format
```markdown
---

## Decision Required

Before committing your changes, write your decision to `.vibe/decision.json`:

- `{"decision": "approve"}` → **Done**
- `{"decision": "reject"}` → **Development** (has retry path)

Example:
```json
{"decision": "approve"}
```
```

### Decision File Format
Agents write to `.vibe/decision.json` in the worktree:
```json
{
  "decision": "approve"
}
```

For rejection/feedback scenarios:
```json
{
  "decision": "reject",
  "feedback": "Missing error handling for edge cases"
}
```

The `feedback` field is passed to subsequent agents via the injected prompt when they pick up the task.

## Validation: Decision Variable Check

When an agent completes work in a column with a required deliverable variable, the system validates that the variable was set correctly.

### Implementation (`container.rs`)

```rust
/// Result of validating the decision variable
pub enum DecisionValidationResult {
    NotRequired,       // Column has no deliverable_variable
    Valid,             // Variable set with valid value
    MissingFile,       // .vibe/decision.json doesn't exist
    MissingVariable,   // File exists but variable not present
    InvalidValue,      // Variable present but value not in valid_options
}
```

The `validate_decision_variable()` function:
1. Checks if the column has a `deliverable_variable`
2. Parses the `deliverable_options` JSON array
3. Validates the decision file against expected variable and options
4. Returns a result that includes error messages for the agent

### Validation Flow

1. **After Agent Commits**: `try_auto_transition()` calls `validate_decision_variable()`
2. **If Validation Fails**:
   - Logs a warning with details
   - Creates a `DecisionValidationFailed` task event with error message
   - The event metadata contains the error prompt for potential auto-retry
3. **Transition Proceeds**: Even with validation failure, transitions still evaluate
   - If no condition matches → `else_column_id` path (retry)
   - After `max_failures` → `escalation_column_id` path (human review)

### Error Messages

The validation generates helpful error messages for agents:

```
The required variable 'decision' was not set.

Please create `.vibe/decision.json` and set 'decision' to one of: approve, reject

Example:
```json
{"decision": "approve"}
```
```

### Task Event Recording

A `DecisionValidationFailed` event is recorded when validation fails:
- `event_type`: `decision_validation_failed`
- `workspace_id`: The workspace where the failure occurred
- `metadata.error`: The full error message with instructions
- `metadata.type`: `"decision_validation_failed"`

This allows the frontend to:
- Show the error to users
- Display the expected variable and valid options
- Potentially trigger a retry with the error context

This provides "soft enforcement" - the agent is guided to compliance but work is never lost.

### Why Soft Enforcement

- Hard blocking could strand work indefinitely
- The `else_column_id` provides a graceful fallback (retry path)
- The `escalation_column_id` provides human intervention after N failures
- Agents are LLMs and may occasionally not follow instructions - the system should be resilient

## Transition Evaluation

When a task completes (agent commits), `try_auto_transition()` evaluates routes:

1. Read `.vibe/decision.json` from workspace
2. For each transition from the current column:
   - If `condition_key`/`condition_value` match decision → Success path (`to_column_id`)
   - If no match and under `max_failures` → Else path (`else_column_id`)
   - If no match and at/over `max_failures` → Escalation path (`escalation_column_id`)
3. If no transition matches and no else path → Task stays in column

## Related
- ADR 2026-01-18-002: Task Auto-start Triggers
- ADR 2026-01-18-003: Agent File Locking
