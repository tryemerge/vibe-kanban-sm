# InDusk: Structured AI Engineering at Scale

**The orchestration layer for building real software with AI agents**

---

## Executive Summary

AI coding tools today are impressive at individual tasks. Give an agent a function to write, a bug to fix, a test to create — it delivers. But building real software isn't about individual tasks. It's about hundreds of interconnected decisions, accumulated context, and coordinated execution across a codebase that evolves over weeks and months.

InDusk is the missing layer between "AI can write code" and "AI can build software." It provides structured workflow pipelines, persistent knowledge that compounds across tasks, and multi-agent coordination — turning AI coding assistants from isolated tools into engineering teams that learn.

---

## The Problem

### AI Coding Has a Memory Problem

Every AI coding session starts cold. The agent doesn't know what was built yesterday, what architecture decisions were made last week, or what patterns the team established last month. You, the developer, become the memory layer — manually pasting context, re-explaining decisions, and hoping the agent doesn't contradict something it built three tasks ago.

This works for one-off tasks. It collapses for real projects.

### Real Software Requires Coordination

A SaaS product isn't one feature — it's billing integration, user management, API design, webhook handling, permission systems, and dozens of other pieces that must work together coherently. When one agent builds authentication with JWTs and another agent builds user profiles assuming session cookies, you don't have a product. You have a mess.

Traditional AI workflows give you no mechanism for this coordination. You are the coordinator. You are the context bridge. You are the one making sure agent #15 knows what agent #1 decided.

### Teams Can't Scale AI Without Structure

Individual developers using AI assistants see productivity gains. But those gains don't compound. There's no institutional memory, no shared patterns, no quality pipeline. Each developer runs their own AI sessions in isolation. The organization learns nothing.

---

## The InDusk Approach

InDusk treats AI-assisted software development as a structured engineering process rather than a collection of isolated prompting sessions.

At its core, InDusk is a **hierarchical orchestration system** for AI agents:

```
                    PROJECT
                       |
        ┌──────────────┼──────────────┐
        |              |              |
   Task Group      Task Group     Task Group
     (Auth)        (Billing)      (Admin)
        |              |              |
    ┌───┼───┐      ┌───┼───┐      ┌───┼───┐
    |   |   |      |   |   |      |   |   |
  Task Task Task  Task Task Task  Task Task Task
    |   |   |      |   |   |      |   |   |
    ↓   ↓   ↓      ↓   ↓   ↓      ↓   ↓   ↓
  Pipeline       Pipeline       Pipeline
  (workflow)     (workflow)     (workflow)
```

**Three layers:**
1. **Task Pipeline** — Each task flows through workflow states (Research → Dev → Test → Review)
2. **Task Group DAG** — Groups of related tasks with internal dependencies (sequential/parallel)
3. **Inter-Group DAG** — Groups depend on other groups (Auth before Billing before Admin)

Each layer has its own orchestration logic, its own agent behaviors, and its own observability surface.

Five core systems make this work:

### 1. Workflow Pipelines — Not Just Kanban

InDusk boards aren't visual task trackers. They're **state machines** where each column represents a stage in your engineering process, with a specific agent, a specific deliverable, and rules that determine what happens next.

A typical pipeline:

```
Research → Development → Testing → Review → Done
```

Each stage has purpose:

| Stage | Agent | Deliverable |
|-------|-------|-------------|
| Research | Analyst | Architecture decision record + implementation plan |
| Development | Coding agent | Working implementation with tests |
| Testing | Test runner | Pass/fail verdict on the test suite |
| Review | Code reviewer | Approve, reject, or request changes |

Tasks flow through this pipeline automatically. When the Research agent writes `{"decision": "ready_to_build"}`, InDusk evaluates transition rules and moves the task to Development. When Testing fails, the task routes back to Development with failure context. After three failed attempts, it escalates to a human.

This isn't CI/CD. CI/CD moves code through stages. InDusk moves **intelligence** through stages — agents make decisions, those decisions route tasks, and the knowledge produced feeds back into the system.

**Why this matters:** You configure the pipeline once. Every task gets the same engineering rigor — research before coding, testing before review, escalation before infinite loops. The process is repeatable, auditable, and consistent.

### 2. Context Compounding — Not Just Memory

InDusk maintains a structured knowledge base that grows with every completed task. This isn't RAG — it's not dumping similar documents into a prompt. It's a typed, scoped, versioned, token-budgeted context system.

**How it works:**

When agents complete work, they can produce **context artifacts** — structured pieces of knowledge stored in the project database:

| Artifact Type | Purpose | Example |
|---------------|---------|---------|
| Architecture Decision Record | Project-wide technical decisions | "Use PostgreSQL with JSONB for flexible metadata" |
| Pattern | Reusable code conventions | "All API endpoints return `{data, error, meta}` shape" |
| Module Memory | File/directory-specific knowledge | "src/auth/ uses jose library for JWT validation" |
| Implementation Plan | Step-by-step build guides | "1. Create schema, 2. Add migrations, 3. Build service" |
| Decision | Specific choices made | "Chose Tailwind over styled-components for CSS" |

**Scoping prevents noise:**

Not all knowledge belongs everywhere. Artifacts have scope:
- **Global** — always injected for every agent (architecture decisions, project patterns)
- **Path** — injected only when working on matching files (module-specific knowledge)
- **Task** — injected only for a specific task (temporary context)

**Token budgets prevent bloat:**

As the knowledge base grows, InDusk doesn't blindly dump everything into prompts. A configurable token budget (default 8,000 tokens) allocates capacity across scopes — 50% global, 30% task, 20% path. Within each scope, artifacts are prioritized by type (ADRs outrank changelog entries) and recency (latest version wins). Unused budget in one scope rolls over to the next.

**The compound effect:**

Task 1 builds Stripe billing and produces an ADR explaining the approach, a pattern for webhook handling, and module memory for the new service file. Task 2 — plan management — starts with all of this context automatically. The agent knows which Stripe API to use, how webhooks work in this codebase, and what already exists. Task 3 is even faster.

By the tenth task on a project, agents have a richer understanding of the codebase than most new team members would after a week of onboarding.

### 3. Hierarchical Task Orchestration — Not Just Sequential Execution

This is where InDusk diverges from every other AI coding tool. Most systems treat tasks as a flat list: task 1, task 2, task 3, all independent or manually ordered. InDusk organizes work as a **three-layer hierarchical DAG** (Directed Acyclic Graph) that mirrors how real engineering teams structure projects:

```
Layer 1: Task Pipeline        (each task flows through workflow states)
              ↓
Layer 2: Task Group DAG        (groups of related tasks, internal dependencies)
              ↓
Layer 3: Inter-Group DAG       (groups depend on other groups)
```

#### Layer 1: Individual Task Pipelines

Each task flows through your configured workflow pipeline (Research → Development → Testing → Review). This is the single-task execution model described in section 1.

#### Layer 2: Task Groups with Internal Dependencies

Related tasks are organized into **task groups** — project-scoped collections that execute as coordinated units:

**Example: "Authentication Feature" group**
```
├─ Add auth types
├─ Add auth API (depends on types)
├─ Add auth middleware (depends on API)
└─ Add auth integration tests (depends on middleware)
```

When you add tasks to a group, InDusk automatically creates dependencies between them. But here's where it gets powerful: **groups have an analysis phase**.

Before execution begins, an orchestrator agent reviews the group's tasks and:
1. **Identifies gaps** — "You have API and UI tasks, but you're missing the types definition"
2. **Finds parallelization opportunities** — "Types and config can run concurrently since they don't overlap"
3. **Moves out-of-scope work** — "The CI pipeline task doesn't belong in auth — moved to infrastructure backlog"
4. **Builds an internal execution DAG** — Determines optimal execution order

The result is a DAG within the group:
```
Parallel Set 1: [Auth types task, Auth config task]
        ↓
Sequential Set 2: [Auth API task]
        ↓
Parallel Set 3: [Auth middleware task, Auth UI task]
        ↓
Sequential Set 4: [Integration tests task]
```

Tasks that can safely run in parallel (no file conflicts, no dependency relationships) do so. Tasks with dependencies run in order. **This is automatic.** You don't draw the graph. The analysis agent does.

#### Layer 3: Inter-Group Dependencies

Task groups themselves form a higher-level dependency graph:

```
[Auth Feature Group]
       ↓
[User Profile Group] ──→ [Billing Integration Group]
       ↓                          ↓
    [Admin Dashboard Group]
```

The inter-group DAG determines **execution priority** across the entire project. Billing can't start until Auth completes. User Profiles and Billing can run in parallel once Auth is done. Admin Dashboard waits for both.

This is the missing abstraction for large AI-driven projects. Without it, you're manually sequencing dozens or hundreds of tasks, trying to remember which features depend on which foundations. With it, the system handles orchestration while you focus on defining the work.

#### Backlog Groups and Auto-Promotion

During execution, agents discover work that wasn't in the original plan. When a developer agent realizes "we need a migration script" or "this needs a webhook handler," it doesn't silently add inline comments or wait for you to notice. It creates a **backlog task group**.

Backlog groups are tagged and tracked. When their dependencies clear (e.g., the Auth group completes), they **automatically promote** into the analysis phase and enter the pipeline.

This creates a continuous, self-organizing workflow:
1. Active groups execute
2. Agents discover gaps and create backlog groups
3. Backlog groups auto-promote when unblocked
4. New groups enter analysis, get organized, and execute
5. Cycle continues

The system manages the queue. You manage the priorities.

#### Shared Workspaces Within Groups (Planned)

Currently, each task runs in its own git worktree (layer 1 isolation). The next evolution: **task groups share a single worktree and branch**. All tasks within "Auth Feature" work on the same checkout, with the parallel sets executing concurrently and sequential sets building on each other's changes. One branch, one PR for the entire feature.

This is the hardest unsolved problem in AI coding tools — coordinated multi-agent work on shared state. InDusk's hierarchical orchestration makes it tractable: the analysis phase determines safe parallelism, the internal DAG prevents race conditions, and the group lifecycle ensures no mid-execution modifications.

### 4. Observability at the Orchestration Level

Here's the insight that makes this whole system trustworthy: **you don't need to watch every agent's terminal output**. You need to see orchestration decisions.

InDusk maintains an **orchestration event log** separate from task-level execution logs:

- When a group enters analysis: what the planner decided, what tasks were added/removed, what the execution plan is
- When tasks move between parallel sets: why this task started now, what's next
- When agents discover out-of-scope work: what backlog group was created, why
- When backlog groups auto-promote: which dependencies cleared, what triggered it
- When inter-group dependencies satisfy: which groups are now unblocked

Every event has a human-readable summary: *"Planner identified missing auth types task and added it to the group."* *"Billing group promoted from backlog because Auth group completed."* *"Developer agent moved CI task to infrastructure backlog."*

This is the **decision layer**. You can step away from the terminal and come back to see *what happened* and *what's happening next* without reading thousands of lines of agent stream-of-consciousness.

The orchestration feed answers the question that makes AI-assisted development feel safe: "Is the system making sensible decisions?"

### 5. Isolated Workspaces — Safe Parallelism

Every task runs in its own git worktree on its own branch. Agents can't interfere with each other. When a task moves between workflow stages, the same branch follows — Research and Development operate on the same checkout.

This enables true parallel execution. Five tasks can be in-flight simultaneously, each in its own isolated workspace, each with its own branch ready for a pull request.

File locking adds another layer — agents can claim exclusive access to files or directories, preventing conflicts when parallel tasks touch overlapping areas of the codebase.

---

## What InDusk Is Not

**InDusk is not a personal AI assistant.** Tools like OpenClaw consolidate your messaging apps, control your devices, and automate your daily life across WhatsApp, Slack, email, and more. InDusk doesn't do any of that. InDusk is laser-focused on one problem: building software well with AI agents.

**InDusk is not a prompt engineering tool.** You don't craft elaborate prompts for each task. You describe what you want in a sentence — "Add Stripe billing integration" — and the pipeline handles research, implementation, testing, and review. The structure is in the workflow, not in your prompt.

**InDusk is not tied to one AI model.** The system is LLM-agnostic. Use Claude Code for development, a different model for code review, a lightweight model for test running. Each workflow stage can have a different agent with a different persona and different capabilities. InDusk orchestrates them.

---

## The Value at Different Scales

### Solo Developer

You're building a side project or a startup MVP. You want AI to handle implementation while you focus on product decisions.

InDusk gives you a repeatable pipeline. Create a task, drag it into the workflow, and let agents research, implement, test, and review the work. The knowledge base grows as you build — by the time you're adding your tenth feature, agents understand your codebase's patterns, your architecture decisions, and your coding conventions.

You stop being the AI babysitter and start being the technical director.

### Small Team (2-10 engineers)

Your team is using AI coding tools individually, but there's no shared learning. One developer's Claude session doesn't know about another developer's Cursor session.

InDusk creates a shared knowledge layer. Architecture decisions, code patterns, and module documentation accumulate in the project's context database. When any team member's agent starts a task, it receives the team's collective knowledge. Consistency emerges naturally — not because you wrote a style guide and hoped people would read it, but because the AI agents literally receive the patterns as context.

Task triggers create soft dependencies — "start the frontend task after the API task completes." The workflow engine handles coordination that previously required standups and Slack messages.

### Growing Organization

You're scaling engineering and need AI-assisted development to be predictable and auditable.

Board templates encode your engineering process as reusable workflow configurations. Every project gets the same quality pipeline — research, implementation, testing, review, escalation. Task events provide an audit trail of every transition, decision, and agent execution.

Context artifacts become institutional knowledge. When a team ships a billing system and produces ADRs and patterns, those artifacts can inform future projects. The organization builds a knowledge base of engineering decisions that survives team turnover.

---

## Architecture

InDusk is built on a Rust backend with a React frontend, deployed locally or on infrastructure you control.

**Core abstractions:**

- **Boards** — Configurable workflow templates with columns, transitions, and agents
- **Columns** — Workflow states with assigned agents, deliverables, and structured output expectations
- **Transitions** — Rules for task routing, with conditions, fallbacks, and escalation paths
- **Context Artifacts** — Typed, scoped, versioned knowledge entries with token budgets
- **Workspaces** — Isolated git worktrees per task with file locking

**Agent-as-Context model:**

Agents in InDusk are not sub-process invocations. They're specialized contexts that shape LLM behavior. When a task enters a workflow column, InDusk assembles:

1. The agent's system prompt (persona and expertise)
2. The task description and workflow history
3. The column's deliverable (expected output)
4. Project context artifacts (accumulated knowledge)

This assembled context is handed to whatever executor runs the agent — Claude Code, Codex, Gemini, Cursor, or others. InDusk doesn't care which LLM powers the agent. It cares about the structure around it.

**The `.vibe/` convention:**

Agents communicate through a standard directory in the workspace:
- `.vibe/decision.json` — Routing decisions and optional artifact creation
- `.vibe/summary.md` — Human-readable task completion summary
- `.vibe/context.md` — Discovered context to preserve

This convention is executor-agnostic. Any tool that can write a JSON file can participate in InDusk workflows.

---

## The Thesis

Software engineering is not a collection of isolated coding tasks. It's a process — a structured, iterative, collaborative process of research, implementation, verification, and accumulated learning.

The current generation of AI coding tools treats each task as independent. InDusk treats them as part of a continuum. Every task inherits the project's history. Every completion enriches the project's future. The workflow enforces quality gates. The context system prevents amnesia. The workspace isolation enables parallelism without chaos.

The question isn't whether AI can write code. It obviously can. The question is whether AI can **build software** — with the consistency, coordination, and institutional memory that real projects demand.

InDusk is our answer: give AI agents the same structure that makes human engineering teams effective, and the results compound.

---

## Current Status

InDusk is an active, open-source project (MIT licensed), forked from Vibe Kanban in December 2025.

**Implemented and working:**
- Workflow engine with configurable pipelines, conditional routing, and escalation paths
- Context artifacts with scoped injection (global/task/path), token budgets, and type priorities
- Structured deliverables via `.vibe/decision.json` convention
- Task triggers (soft dependencies between tasks)
- File locking for parallel execution safety
- Board templates for reusable workflow configurations
- **Task groups (ADR-012)** — project-scoped grouping with auto-dependencies and immutability
- Inter-group dependency DAG for coordinating multiple groups
- Swim lane visualization by task group

**Designed and planned (implementation in progress):**
- **Group lifecycle orchestration (ADR-014)** — analysis phase, internal execution DAGs, backlog auto-promotion
- **Orchestration event system (ADR-014)** — decision-level logging and observability feed
- **Group-scoped context (ADR-013)** — knowledge inheritance along the inter-group DAG
- Shared workspaces per group (multiple agents, one branch)

The project is under active development with a focus on validating the hierarchical orchestration and context compounding theses through real-world usage.

**Get involved:** [GitHub Repository](https://github.com/anthropics/vibe-kanban) (upstream), InDusk fork coming soon

---

*InDusk: Because building software is harder than writing code.*
