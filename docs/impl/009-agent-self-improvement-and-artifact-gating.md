# Implementation Plan: Agent Self-Improvement Protocol & Conditional Artifact Gating (ADR-009)

## Overview

Two features, three phases:
1. **Knowledge prompt on columns** — new column field + injection into agent start command
2. **Conditional artifact gating** — parse `metadata.conditions` during `build_full_context()`
3. **MCP + UI** — expose conditions in create_artifact tool and knowledge prompt in column settings

## Phase 1: Knowledge Production Prompt

### 1.1 Migration: Add `knowledge_prompt` to `kanban_columns`

**File:** `crates/db/migrations/20260215000001_add_knowledge_prompt_to_columns.sql`

```sql
ALTER TABLE kanban_columns ADD COLUMN knowledge_prompt TEXT;
```

No default value — null means "no knowledge production instruction for this stage." Existing columns continue working as-is.

### 1.2 Update Rust model

**File:** `crates/db/src/models/kanban_column.rs`

- Add `pub knowledge_prompt: Option<String>` to `KanbanColumn` struct
- Add `pub knowledge_prompt: Option<String>` to `CreateKanbanColumn`
- Add `pub knowledge_prompt: Option<String>` to `UpdateKanbanColumn`
- Update all SQLx queries that SELECT/INSERT/UPDATE kanban_columns to include the new field
- Search for all struct constructions: `grep -r "CreateKanbanColumn {" crates/` and `grep -r "UpdateKanbanColumn {" crates/` — add `knowledge_prompt: None` to each

### 1.3 Inject knowledge prompt into agent context

**File:** `crates/services/src/services/container.rs`

In the agent context assembly block (~line 1213), after combining `start_command` with `decision_instructions`, also append the column's `knowledge_prompt` if present:

```rust
// Combine start_command + decision_instructions + knowledge_prompt
let start_command = {
    let mut parts = Vec::new();
    if let Some(cmd) = &expanded_start_command {
        parts.push(cmd.clone());
    }
    if let Some(instructions) = &decision_instructions {
        parts.push(instructions.clone());
    }
    if let Some(knowledge) = &column.knowledge_prompt {
        parts.push(knowledge.clone());
    }
    if parts.is_empty() { None } else { Some(parts.join("\n")) }
};
```

The `column` variable is already loaded at this point (it's used for deliverable, agent_id, etc.), so no additional query needed.

### 1.4 Update MCP column creation

**File:** `crates/server/src/mcp/task_server.rs`

Add `knowledge_prompt` as an optional string parameter to the `create_column` MCP tool definition and pass it through to `CreateKanbanColumn`.

### 1.5 Regenerate types

```bash
DATABASE_URL="sqlite:///Users/the_dusky/code/emerge/vibe-kanban-sm/dev_assets/db.sqlite" pnpm run generate-types
```

### 1.6 Seed template knowledge prompts

**File:** `crates/db/migrations/20260215000002_seed_knowledge_prompts_on_templates.sql`

Update the existing template columns with default knowledge prompts:

```sql
-- Research column template: produce ADRs and implementation plans
UPDATE kanban_columns
SET knowledge_prompt = '## Knowledge Production

As you analyze the codebase and plan implementation, capture your findings as artifacts:

- **ADR**: For architecture decisions (which approach to use and why)
- **Implementation Plan (iplan)**: Step-by-step build plan for the development agent

Include artifacts in your `.vibe/decision.json`:
```json
{
  "decision": "ready_to_build",
  "artifact_type": "adr",
  "title": "ADR: Your Decision Title",
  "content": "## Context\n...\n## Decision\n...\n## Consequences\n...",
  "scope": "global"
}
```

Or use the `create_artifact` MCP tool during your work to store knowledge as you discover it.'
WHERE slug = 'research' AND is_template = true;

-- Development column template: produce patterns and module memories
UPDATE kanban_columns
SET knowledge_prompt = '## Knowledge Production

As you implement, look for reusable knowledge to capture:

- **Pattern**: Code patterns future tasks should follow (e.g., error handling, API response shape)
- **Module Memory**: What a file/module does, key functions, important decisions

Include in `.vibe/decision.json` or use the `create_artifact` MCP tool at any time.

Prefer capturing knowledge over skipping it — artifacts that aren''t useful can be cleaned up later.'
WHERE slug IN ('development', 'inprogress') AND is_template = true;

-- Review column template: produce patterns observed during review
UPDATE kanban_columns
SET knowledge_prompt = '## Knowledge Production

If you observe a pattern worth preserving for the project (good or bad), create a `pattern` artifact.

Example: If the code establishes a new convention (webhook handling pattern, test structure, etc.), capture it so future agents follow the same approach.'
WHERE slug = 'review' AND is_template = true;
```

## Phase 2: Conditional Artifact Gating

### 2.1 Define condition types

**File:** `crates/db/src/models/context_artifact.rs`

Add a struct for parsing conditions from the metadata field:

```rust
/// Conditions that control when an artifact is injected into agent context.
/// Stored in the artifact's `metadata` JSON field under the key "conditions".
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConditions {
    /// Task must have at least one of these labels
    #[serde(default)]
    pub labels: Vec<String>,

    /// Task must involve files matching these path prefixes
    #[serde(default)]
    pub paths: Vec<String>,

    /// Task title/description must contain at least one of these keywords (case-insensitive)
    #[serde(default)]
    pub keywords_in_task: Vec<String>,
}

impl ArtifactConditions {
    /// Parse conditions from artifact metadata JSON
    pub fn from_metadata(metadata: &Option<String>) -> Option<Self> {
        let meta_str = metadata.as_ref()?;
        let meta: serde_json::Value = serde_json::from_str(meta_str).ok()?;
        let conditions = meta.get("conditions")?;
        serde_json::from_value(conditions.clone()).ok()
    }

    /// Returns true if all conditions are empty (no filtering)
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
            && self.paths.is_empty()
            && self.keywords_in_task.is_empty()
    }

    /// Check if conditions match the given task context
    pub fn matches(&self, ctx: &ArtifactMatchContext) -> bool {
        if self.is_empty() {
            return true;
        }

        // All non-empty condition types must match (AND)
        // Within each type, any value matches (OR)

        if !self.labels.is_empty() {
            let has_label = self.labels.iter().any(|l| {
                ctx.task_labels.iter().any(|tl| tl.eq_ignore_ascii_case(l))
            });
            if !has_label { return false; }
        }

        if !self.paths.is_empty() {
            let has_path = self.paths.iter().any(|p| {
                ctx.task_paths.iter().any(|tp| tp.starts_with(p))
            });
            if !has_path { return false; }
        }

        if !self.keywords_in_task.is_empty() {
            let task_text = ctx.task_text_lower.as_str();
            let has_keyword = self.keywords_in_task.iter().any(|kw| {
                task_text.contains(&kw.to_lowercase())
            });
            if !has_keyword { return false; }
        }

        true
    }
}

/// Context about the current task, used for evaluating artifact conditions
pub struct ArtifactMatchContext {
    pub task_labels: Vec<String>,
    pub task_paths: Vec<String>,
    pub task_text_lower: String, // lowercased title + " " + description
}
```

### 2.2 Integrate gating into `build_full_context()`

**File:** `crates/db/src/models/context_artifact.rs`

Update the `build_full_context()` signature to accept an optional `ArtifactMatchContext`:

```rust
pub async fn build_full_context(
    pool: &PgPool,
    project_id: Uuid,
    task_id: Option<Uuid>,
    paths: &[String],
    match_context: Option<&ArtifactMatchContext>, // NEW
) -> Result<String, sqlx::Error> {
```

In the greedy fill loop, before checking token budget, check conditions:

```rust
// Skip if artifact has conditions that don't match
if let Some(ctx) = match_context {
    if let Some(conditions) = ArtifactConditions::from_metadata(&artifact.metadata) {
        if !conditions.matches(ctx) {
            continue;
        }
    }
}
```

Also update `build_full_context_with_stats()` with the same parameter and logic.

### 2.3 Pass match context from container.rs

**File:** `crates/services/src/services/container.rs`

When calling `build_full_context()` (~line 1249), build the match context from the task:

```rust
// Build match context for conditional artifact gating (ADR-009)
let task_labels = TaskLabelAssignment::find_labels_for_task(pool, task.id)
    .await
    .unwrap_or_default()
    .iter()
    .map(|l| l.name.clone())
    .collect();

let task_text = format!(
    "{} {}",
    task.title.as_deref().unwrap_or(""),
    task.description.as_deref().unwrap_or("")
).to_lowercase();

let match_context = ArtifactMatchContext {
    task_labels,
    task_paths: vec![], // populated from agent context files if available
    task_text_lower: task_text,
};

let project_context = match ContextArtifact::build_full_context(
    pool,
    task.project_id,
    Some(task.id),
    &[],
    Some(&match_context), // NEW
).await {
```

### 2.4 Update preview endpoint

**File:** `crates/server/src/routes/context_artifacts.rs`

Add optional `task_id` to the preview query params. When provided, load the task's labels and text to build `ArtifactMatchContext` so the preview reflects actual gating behavior.

## Phase 3: MCP + UI

### 3.1 Update `create_artifact` MCP tool

**File:** `crates/server/src/mcp/task_server.rs`

Add an optional `conditions` parameter (JSON object) to the `create_artifact` tool. When provided, merge it into the artifact's `metadata` field:

```rust
// If conditions provided, wrap in metadata.conditions
let metadata = match conditions {
    Some(conds) => Some(serde_json::json!({ "conditions": conds })),
    None => input_metadata,
};
```

### 3.2 Update `try_create_artifact_from_decision()`

**File:** `crates/services/src/services/container.rs`

When parsing decision.json, also extract `conditions` and merge into metadata:

```rust
// Extract optional conditions for artifact gating (ADR-009)
let conditions = decision.get("conditions");

let metadata = match (decision.get("metadata"), conditions) {
    (Some(meta), Some(conds)) => {
        let mut m = meta.clone();
        m.as_object_mut()
            .map(|obj| obj.insert("conditions".to_string(), conds.clone()));
        Some(m)
    }
    (None, Some(conds)) => Some(serde_json::json!({ "conditions": conds })),
    (Some(meta), None) => Some(meta.clone()),
    (None, None) => None,
};
```

### 3.3 Frontend: Knowledge prompt in column settings

**File:** `frontend/src/pages/settings/BoardSettings.tsx`

Add a textarea field for `knowledge_prompt` in the column edit form. Label it "Knowledge Production Prompt" with helper text: "Instructions for agents about what knowledge to capture during this workflow stage. Leave empty to disable."

### 3.4 Frontend: Conditions display in Knowledge Base

**File:** `frontend/src/pages/settings/KnowledgeBaseSettings.tsx`

When displaying artifacts that have `metadata.conditions`, show a small badge/chip indicating the conditions (e.g., "Labels: billing, stripe" or "Keywords: payment"). This helps users understand why certain artifacts might not appear for certain tasks.

## Verification

### Phase 1
- `pnpm run prepare-db` — migration passes
- `cargo build` — compiles with new field
- `pnpm run generate-types` — types regenerate
- Create a column with knowledge_prompt set, start a task, verify the prompt appears in the agent's start command

### Phase 2
- Create an artifact with conditions: `{"conditions": {"labels": ["billing"]}}`
- Build context for a task WITH the billing label → artifact included
- Build context for a task WITHOUT the billing label → artifact excluded
- Build context with no match_context (existing callers) → artifact included (backward compat)
- Context preview endpoint reflects gating correctly

### Phase 3
- Create artifact via MCP with conditions → conditions stored in metadata
- Create artifact via decision.json with conditions → conditions stored
- Column settings UI shows knowledge_prompt textarea
- Knowledge Base page shows condition badges

## Files Changed Summary

| File | Change |
|------|--------|
| `crates/db/migrations/20260215000001_*.sql` | Add `knowledge_prompt` column |
| `crates/db/migrations/20260215000002_*.sql` | Seed template knowledge prompts |
| `crates/db/src/models/kanban_column.rs` | Add field to structs + queries |
| `crates/db/src/models/context_artifact.rs` | Add `ArtifactConditions`, `ArtifactMatchContext`, update `build_full_context()` |
| `crates/services/src/services/container.rs` | Inject knowledge_prompt, build match context |
| `crates/server/src/mcp/task_server.rs` | Add params to create_column + create_artifact |
| `crates/server/src/routes/context_artifacts.rs` | Update preview endpoint |
| `frontend/src/pages/settings/BoardSettings.tsx` | Knowledge prompt textarea |
| `frontend/src/pages/settings/KnowledgeBaseSettings.tsx` | Conditions display |
| `shared/types.ts` | Regenerated |

## Estimated Complexity

- **Phase 1**: Small — one migration, one new field threaded through, prompt concatenation in container.rs
- **Phase 2**: Medium — new struct + matching logic, signature change on build_full_context (need to update all callers)
- **Phase 3**: Small — MCP param additions, UI textarea + badges
