# ADR 008: Task Lifecycle Observability

**Date:** 2026-02-14
**Status:** Accepted
**Author:** User + Claude

## Context

InDusk has a rich event system (12 event types covering column transitions, agent starts/completions, commits, artifact creation, decision validation failures, and escalations) and a full context artifact API. However, **none of this is visible to the user.**

- A `TaskEventTimeline` frontend component was built but never wired into the UI.
- Context artifacts have full CRUD endpoints but zero frontend representation.
- There is no way to see what context an agent received when it started, what it produced, or why a transition fired.

Without visibility into the pipeline, there's no way to diagnose agent failures, understand why an artifact was or wasn't included in context, measure whether knowledge compounding is working, or identify improvements to the workflow.

## Decision

Add three observability surfaces:

### 1. Task-level tabs (Timeline + Context)

Add tabs to the TaskPanel component:
- **Overview** — existing content (description, labels, attempts)
- **Timeline** — wire in the existing `TaskEventTimeline` component showing the full event audit trail for a task
- **Context** — new panel showing artifacts produced by this task and a preview of the assembled context an agent would receive

### 2. Context preview endpoint

Add `GET /api/context-artifacts/preview-context?project_id=...&task_id=...` that calls `build_full_context()` and returns the assembled context string along with metadata: tokens used, token budget, artifacts included vs total available.

This lets users see exactly what an agent receives — and what was excluded due to budget.

### 3. Project-level Knowledge Base page

Add a settings page showing all context artifacts across the project:
- Filterable by type (ADR, Pattern, ModuleMemory, etc.) and scope (Global, Task, Path)
- Shows token estimates, source tasks, versions
- Allows deletion of stale artifacts
- Summary stats (total artifacts, total tokens)

## Consequences

### Positive
- Users can finally see the full task lifecycle: events, decisions, context, artifacts
- Context budget usage becomes visible — can identify when important artifacts are being excluded
- Knowledge Base page enables cleanup of stale or low-value artifacts
- Foundation for future analytics (artifact hit rates, failure patterns, context effectiveness)

### Negative
- Adds frontend complexity (new components, API client, hooks)
- Context preview endpoint runs `build_full_context()` on demand — minor compute cost

### Future Work
- **Context diff**: Compare what two different tasks received as context
- **Artifact effectiveness tracking**: Did including artifact X correlate with agent success?
- **Real-time event stream**: WebSocket updates to the timeline as agents work
- **Token budget tuning UI**: Let users adjust the 8000-token budget and scope percentages per project
