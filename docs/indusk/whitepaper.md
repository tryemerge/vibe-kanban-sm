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

Three core systems make this work:

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

### 3. Isolated Workspaces — Safe Parallelism

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

InDusk is an active, open-source project (MIT licensed), forked from Vibe Kanban in December 2025. The core systems — workflow engine, context compounding, structured deliverables, task triggers, file locking, board templates — are built and functional.

The project is under active development with a focus on validating the context compounding thesis through real-world usage.

**Get involved:** [GitHub Repository](https://github.com/example/indusk)

---

*InDusk: Because building software is harder than writing code.*
