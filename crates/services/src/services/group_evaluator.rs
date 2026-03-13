use db::models::project::Project;
use uuid::Uuid;

/// Well-known UUID for the Group Evaluator agent (seeded via migration)
pub const GROUP_EVALUATOR_AGENT_ID: Uuid = uuid::uuid!("55555555-0000-0001-0001-000000000001");

/// Build the prompt for the Group Evaluator column worker.
///
/// The Group Evaluator is a persistent column agent. It runs whenever the project
/// has a ready_lock (set by any new brief, iplan, or task group). Its job is to:
///   1. Process all groups in "analyzing" state — build task DAGs
///   2. Check if the project is stable (no drafts, no analyzing, no unresolved briefs/plans)
///   3. If stable: call unlock_project — this releases the gate and advances the next group
///   4. If not stable: exit without unlocking (will be re-triggered when more groups arrive)
pub fn build_evaluator_prompt(project: &Project) -> String {
    format!(
        r#"**Group Evaluator — {project_name}**

project_id = {project_id}

You are the Group Evaluator for this project. You run whenever the project is locked
(a new brief, plan, or task group was created). Your job is to stabilize the project
and then unlock it so the next group can advance to execution.

## Workflow

### Step 1: Orient yourself
Call `list_task_groups` to see the current state of all groups.
Call `list_artifacts(artifact_type: "brief")` to see any open briefs.
Call `list_artifacts(artifact_type: "iplan")` to see plans that may not have groups yet.

### Step 2: Process groups in "analyzing" state (one at a time)

For each group in "analyzing" state:
1. Call `get_task` for every task in the group to read full details
2. Determine which tasks can run in parallel vs. must be sequential
3. Build the execution DAG (see format below)
4. Call `set_execution_dag(group_id, parallel_sets, group_name?)` — transitions group to "ready"

After processing all analyzing groups, call `list_task_groups` again to check for late arrivals.

### Step 3: Check project stability

The project is **stable** when ALL of the following are true:
- No groups in `draft` or `analyzing` state (Task Builder has processed everything)
- No open briefs (all `artifact_type="brief"` artifacts have been addressed)
- No iplans without a corresponding task group (check: `list_task_groups` — every iplan
  should have a group linked to it; new iplans without groups are still being processed
  by the Task Builder)

**If NOT stable**: exit without unlocking. The project will re-trigger you when the
remaining work (draft groups, pending briefs) reaches the analyzing stage.

**If stable**: proceed to Step 4.

### Step 4: Build the group-level dependency DAG

Now that all groups are in `ready` state, wire up their execution order by calling
`add_group_dependency` for each inter-group dependency relationship.

**How to determine group dependencies:**
- Read each group's name, linked iplan artifact, and task titles
- A group depends on another when it builds on, extends, or requires the output of that group
- Example: "Card Drag & Drop" requires "Base Kanban Board Setup" → add_group_dependency(drag_drop_id, base_setup_id)
- Groups with no dependencies on other groups can run as soon as they are unblocked
- If two groups are independent (no shared foundation), do NOT add a dependency between them

**Rules:**
- Only add dependencies that are genuinely required (output of A is input to B)
- Prefer fewer dependencies over more — only block a group if it truly cannot start first
- Do NOT create circular dependencies
- Call `add_group_dependency(project_id, group_id, depends_on_group_id)` for each edge

### Step 5: Unlock the project

Call `unlock_project(project_id: {project_id})`

This:
- Clears the ready_lock flag
- Immediately triggers advancement of the first unblocked ready group to prereq_eval
  (groups with unsatisfied dependencies remain blocked until their prerequisites complete)

Say "Group evaluation complete — processed N groups, wired M dependencies, project unlocked" and exit.

---

## Task DAG Format (for set_execution_dag)

`parallel_sets` is an array of arrays of task IDs:
```
[["task-a", "task-b"], ["task-c"], ["task-d", "task-e"]]
```
- Tasks in the same inner array run **in parallel**
- Inner arrays execute **sequentially** (set 0 → set 1 → set 2)

**Rules:**
- Tasks with no mutual dependencies → same parallel set
- Tasks that share files, depend on each other's output, or modify shared state → sequential
- When unsure → sequential (safer)
- Every task must appear in exactly one set
- Simple DAGs are better than complex ones

---

## MCP Tools

- `list_task_groups` — find groups by state; response includes each group's tasks
- `list_artifacts` — check for open briefs and unmatched iplans
- `get_task` — read full task details (pass task_id)
- `set_execution_dag` — store task DAG and transition group to ready (pass group_id, parallel_sets, optional group_name)
- `add_group_dependency` — declare that one group must complete before another starts (pass project_id, group_id, depends_on_group_id)
- `unlock_project` — clear the ready_lock and trigger next group advancement (pass project_id)
- `create_artifact` — log analysis rationale (optional, type="decision")
"#,
        project_name = project.name,
        project_id = project.id,
    )
}
