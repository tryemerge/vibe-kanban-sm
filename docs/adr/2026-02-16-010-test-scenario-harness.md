# ADR 010: Test Scenario Harness

**Date:** 2026-02-16
**Status:** Accepted
**Author:** User + Claude

## Context

### What InDusk is really about

InDusk's core value isn't "making AI agents better at building software." That problem is being solved inside the models themselves. InDusk solves a different problem:

**Humans can't fully explain what they want upfront.** Requirements are discovered through the process of building. A human sees an agent's output and thinks "no, not like that — like *this*." That correction is where the real insight lives.

InDusk provides a structured collaboration surface between humans and AI agents where:

1. **Agents work through observable stages** — humans see what's happening at each step rather than getting a black-box result
2. **Intervention points are built into the workflow** — review columns, rejection paths, escalation to human review
3. **Human corrections persist as context** — rejections with feedback flow back to agents, decisions become artifacts
4. **The system gets progressively better at reflecting what the human actually wants** — through accumulated context that encodes preferences, decisions, and corrections

The question isn't "can AI write code?" It's "can a human and AI collaborate to build the *right* software, and does the system get better at understanding what 'right' means over time?"

### Why we need a test harness

We can't answer whether this collaboration loop works because we have no way to measure it:

- Does context from a completed task help future agents align with what the human wants?
- When a human rejects a task with feedback, does the retry incorporate that feedback?
- Does accumulated knowledge reduce the amount of human correction needed over time?
- Would different workflow structures or task granularity improve collaboration quality?

## Decision

Build a **scenario-based test harness** as scripts in `scripts/scenarios/`. Each scenario is a JSON file describing a complete test — board, tasks, simulated agent outputs, simulated human interventions, and measurement checkpoints. A runner script creates the environment via the REST API and reports what happened.

### Design Principles

1. **Simple boards** — Test scenarios use minimal boards (Backlog → Work → Done) to test the core engine, not workflow configuration complexity
2. **Model both sides** — Scenarios simulate agent outputs AND human interventions (rejections with feedback, corrections)
3. **Declarative** — Scenarios are data (JSON), not code. Easy to version, iterate, and compare
4. **Repeatable** — Fresh project every run, identical starting conditions
5. **Evolvable** — Easy to create variants that test different hypotheses (task granularity, knowledge prompts, gating)

### Scenario Format

Steps are ordered actions modeling the collaboration loop:

- `create_task` — Human creates a task (partial intent)
- `simulate_agent_output` — Agent produces artifacts and/or a decision
- `checkpoint` — Measure: artifact counts, context previews, token usage

### Implementation

Plain Node.js scripts (CommonJS, native fetch) matching existing `scripts/` patterns:

- `scripts/lib/api-client.js` — REST client for the InDusk API
- `scripts/scenarios/run-scenario.js` — Reads scenario JSON, executes steps, reports measurements
- `scripts/scenarios/teardown-scenario.js` — Cleanup by project ID
- `scripts/scenarios/definitions/*.json` — Scenario data files

## Consequences

### Positive
- First mechanism to measure whether InDusk's collaboration loop works
- Models both sides — agent outputs and human interventions
- Repeatable for before/after comparisons
- Declarative format scales to many hypotheses without new code
- No external dependencies

### Negative
- Simulated artifacts don't test agent quality — only context plumbing
- Requires running dev server
- No automated comparison between runs yet

### Future Work
- **Scenario comparison mode**: Run two variants, diff results
- **Real agent execution**: Replace simulations with actual agent runs
- **Metric persistence**: Store results for trend analysis
- **Interactive mode**: Pause at checkpoints for actual human review
