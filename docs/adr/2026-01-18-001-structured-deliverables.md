# ADR 2026-01-18-001: Structured Deliverables and Transition Builder

## Status
Proposed

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
5. Consider: validation of agent output against schema (warning vs. error)

## Related
- ADR 2026-01-18-002: Task Auto-start Triggers
- ADR 2026-01-18-003: Agent File Locking
