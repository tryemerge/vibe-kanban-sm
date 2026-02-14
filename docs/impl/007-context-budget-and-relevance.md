# Implementation Plan: Context Budget & Relevance System (ADR-007)

## Overview

Add token-budgeted context injection to `build_full_context()`, deduplicate by chain, prioritize by type, and wire `container.rs` to use it.

## Steps

### 1. Migration: Add `token_estimate` column

**File:** `crates/db/migrations/20260208000001_add_token_estimate.sql`

```sql
ALTER TABLE context_artifacts ADD COLUMN token_estimate INTEGER NOT NULL DEFAULT 0;
UPDATE context_artifacts SET token_estimate = LENGTH(content) / 4;
```

### 2. Update Rust model

**File:** `crates/db/src/models/context_artifact.rs`

- Add `token_estimate: i32` to `ContextArtifact` struct
- Add all SQLx query annotations for the new field
- Compute `token_estimate` in `create()` and `update()` methods
- Add `type_priority()` helper on `ArtifactType` returning the ordering (1=ADR, 2=Pattern, etc.)
- Rewrite `build_full_context()` to:
  1. Query each scope with `ORDER BY` type priority + recency
  2. Filter to latest version per chain_id
  3. Fill greedily up to scope budget
  4. Roll over unused budget

### 3. Wire container.rs

**File:** `crates/services/src/services/container.rs`

- Replace `build_project_context()` call with `ContextArtifact::build_full_context()`
- Pass `task_id` and relevant paths (from task context files)
- Remove the standalone `build_project_context()` function

### 4. Update generate_types.rs

No new exported types needed — `ContextArtifact` already exported, just gains the `token_estimate` field.

### 5. Verification

- `pnpm run prepare-db` — migration passes
- `cargo build` — compiles
- `pnpm run generate-types` — types regenerate with new field
