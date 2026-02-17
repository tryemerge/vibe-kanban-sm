# ADR 011: Boards Are Templates — Remove Redundant Template System

**Date:** 2026-02-16
**Status:** Proposed
**Author:** User + Claude

## Context

The current system has two overlapping concepts:

1. **Boards** — Reusable workflow configurations (columns + transitions + agent assignments). A project points to a board via `board_id`.
2. **Board Templates** — A mechanism that copies a board's configuration into a new board when applied to a project.

This is redundant. A board is already a reusable configuration. Multiple projects can point to the same `board_id` and share the same workflow. There's no reason to create copies.

The current behavior causes:
- Duplicate boards with identical configurations cluttering the board list
- Confusion about which board is the "real" one vs a copy
- Test/setup scripts creating new boards every run instead of reusing existing ones

## Decision

**Boards are templates.** Remove the separate template/copy mechanism.

- A board is created once with its columns, transitions, and agent assignments
- Any project that wants that workflow assigns the board's ID
- No copying — projects share boards directly
- The board list IS the template library

## Consequences

### Positive
- Simpler mental model: one concept instead of two
- No duplicate boards
- Changes to a board's configuration affect all projects using it (which is usually what you want)
- Board list stays clean

### Negative
- If a project needs a slight variation of an existing board, it must create a new board (can't "fork" a template)
- Changing a shared board affects all projects using it (could be surprising)

### Future Work
- "Clone board" as an explicit user action when you want a variation
- Board versioning if shared-board mutation becomes a problem
