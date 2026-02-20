---
title: "Welcome to InDusk"
description: "An advanced fork of Vibe Kanban with workflow automation, agent orchestration, and continuous knowledge improvement"
---

<InDuskHeader title="Welcome to InDusk" subtitle="Intelligent orchestration for AI-assisted development" badge="InDusk" />

## What is InDusk?

**InDusk** is a heavily enhanced fork of [Vibe Kanban](https://github.com/BloopAI/vibe-kanban). While Vibe Kanban provides solid foundations for AI agent orchestration, InDusk adds sophisticated features for workflow automation, multi-agent coordination, and continuous knowledge improvement.

::: info
**Fork Date:** December 31, 2025
**Codename:** Infinite Dusky -> InDusk
:::

---

## What InDusk Adds

<CardGrid :cols="2">
  <Card title="Workflow Engine" icon="diagram-project">
    Columns become states in a state machine. Define transitions, conditions, and automatic routing based on agent decisions.
  </Card>

  <Card title="Context Compounding" icon="brain">
    Every task can improve project knowledge. Agents learn from past work through a structured artifact system.
  </Card>

  <Card title="Structured Deliverables" icon="list-check">
    Define exactly what agents should output. Validate decisions against allowed options.
  </Card>

  <Card title="Task Triggers" icon="bolt">
    Create soft dependencies between tasks. "Start Task B after Task A completes."
  </Card>

  <Card title="File Locking" icon="lock">
    Prevent parallel agents from conflicting on the same files. Automatic release on task completion.
  </Card>

  <Card title="Swim Lanes & Labels" icon="tags">
    Organize tasks visually with labels and collapsible swim lanes.
  </Card>
</CardGrid>

---

## Base Vibe Kanban vs InDusk

| Feature | Base Vibe Kanban | InDusk |
|---------|------------------|--------|
| Git worktree isolation | Yes | Yes |
| Multi-agent support | Yes | Yes |
| Visual code review | Yes | Yes |
| GitHub integration | Yes | Yes |
| **Workflow state machine** | No | Yes |
| **Conditional routing** | No | Yes |
| **Context artifacts** | No | Yes |
| **Knowledge compounding** | No | Yes |
| **Structured deliverables** | No | Yes |
| **Task triggers** | No | Yes |
| **File locking** | No | Yes |
| **Board templates** | No | Yes |
| **Task labels & swim lanes** | No | Yes |

---

## Architecture Philosophy

InDusk is built on the **Agent-as-Context** model:

> Agents are not sub-process invocations but **specialized contexts** that shape LLM behaviour.

When a task enters a column:
1. **Agent's system prompt** establishes persona and expertise
2. **Task description + workflow history** provides context
3. **Column's deliverable** sets expectations
4. **Project context artifacts** inject accumulated knowledge

This approach is LLM-agnostic--works with Claude Code, Gemini, Codex, Cursor, and others.

---

## The Knowledge Loop

The key innovation in InDusk is the **continuous improvement cycle**:

```mermaid
flowchart LR
    T1[Task 1] -->|"Produces"| A1[Artifact]
    A1 -->|"Stored"| DB[(Context DB)]
    DB -->|"Injected into"| T2[Task 2]
    T2 -->|"Produces"| A2[Artifact]
    A2 -->|"Stored"| DB
    DB -->|"Injected into"| T3[Task 3]

    style DB fill:#e1f5fe
```

Every completed task can add:
- **Architecture Decision Records (ADRs)**
- **Patterns and best practices**
- **Module-specific knowledge**
- **Implementation plans**

Future agents automatically receive this context, making them smarter with each task.

---

## Explore InDusk Features

<CardGrid :cols="3">
  <Card title="How It Works" icon="circle-play" href="/indusk/how-it-works">
    Visual overview of the entire system
  </Card>

  <Card title="Workflow Engine" icon="gears" href="/indusk/workflow-engine">
    Columns, transitions, and automation
  </Card>

  <Card title="Context System" icon="brain" href="/indusk/context-system">
    Knowledge compounding in detail
  </Card>
</CardGrid>
