# InDusk Experiments & Observations

> This document tracks whether InDusk's approach is actually working.
> Since InDusk IS a knowledge management system, this is the meta-layer:
> we're using structured observation to evaluate a system designed for structured observation.

**Last Updated:** 2026-02-07

---

## Hypothesis 1: Context Compounding Improves Agent Output

**Status:** Untested

**Claim:** Agents that receive context artifacts (ADRs, patterns, module memories) from previous tasks produce better, more consistent output than agents starting from scratch.

**How to test:**
- Run the same type of task twice: once with no artifacts, once with artifacts from a prior task
- Compare output quality, adherence to patterns, and time to completion
- Track whether agents reference injected context in their work

**Observations:**
- *No observations yet. Need to run controlled experiments.*

---

## Hypothesis 2: Workflow State Machine Reduces Manual Overhead

**Status:** Partially Validated

**Claim:** Defining transitions, conditions, and auto-routing reduces the manual work of moving tasks between columns and re-running agents.

**How to test:**
- Set up a Code Review Pipeline template
- Run 5+ tasks through it end-to-end
- Count manual interventions required vs expected automatic routing

**Observations:**
- The routing logic exists and works in code (conditional transitions, else paths, escalation)
- Haven't yet run enough real tasks through the pipeline to measure overhead reduction
- The template system makes setup fast -- applying "Code Review Pipeline" creates the full workflow

---

## Hypothesis 3: Structured Deliverables Improve Agent Compliance

**Status:** Untested

**Claim:** When agents are told exactly what `.vibe/decision.json` values are valid, they comply more reliably than with freeform instructions.

**How to test:**
- Run agents on columns with and without `deliverable_variable` + `deliverable_options`
- Track how often agents produce valid vs invalid decision files
- Measure retry rate (how often tasks hit the `else_column_id` path)

**Observations:**
- `build_decision_instructions()` generates clear markdown instructions for agents
- `validate_decision_variable()` provides specific error messages when agents fail
- *No real-world compliance data yet*

---

## Hypothesis 4: Functional App Design (ADR-006) Can Bootstrap Projects

**Status:** Planning

**Claim:** A single prompt ("Build me a todo app with auth") can trigger an intelligent bootstrapping process that generates boards, tasks, documentation, and ADRs.

**Setup:** ADR-006 describes a two-phase approach:
1. Research Bot agent analyzes requirements using MCP tools
2. Generated tasks flow through the workflow pipeline

**What's built:**
- MCP tools for create_board, create_column, create_transition, create_task all exist
- Research board template seeded
- Implementation plan at `docs/impl/006-functional-application-design.md`

**What's missing:**
- The Research Bot agent itself
- End-to-end testing of the bootstrap flow

**Observations:**
- *No observations yet. This is the most ambitious hypothesis.*

---

## Meta: Is the Documentation System Working?

**Claim:** This three-layer documentation system (AGENTS.md pointer -> ARCHITECTURE.md -> STATUS.md/EXPERIMENTS.md) provides enough context for any new agent to be productive immediately.

**How to test:**
- Start a new conversation with a fresh agent
- Give it a non-trivial task
- Observe: Does it read the docs? Does it find the right files? Does it understand the architecture?
- Track time-to-productivity

**Observations:**
- 2026-02-07: System just created. Will track from here.

---

## Patterns Observed

*Record recurring patterns in agent behavior here. What works well? What trips agents up?*

- Agents reliably follow `CLAUDE.md` instructions (it's auto-loaded)
- Agents often don't know to look at docs/ unless pointed there
- Long files (>500 lines) reduce agent effectiveness -- agents lose track of structure
- Structured output (JSON with defined schema) works better than freeform markdown for machine-readable results

---

## Anti-Patterns Discovered

*Record things that don't work well here.*

- Putting too much content in CLAUDE.md wastes context window on every prompt
- Agents creating context artifacts without clear schemas leads to inconsistent data
- Mixing "user-facing docs" (Mintlify) with "agent-facing docs" (ARCHITECTURE.md) in the same files causes confusion about audience

---

## Experiment Log

*Chronological log of experiments run and results observed.*

| Date | Experiment | Result | Notes |
|------|-----------|--------|-------|
| 2026-02-07 | Created documentation system | Pending | Will observe agent behavior in future sessions |
| | | | |
