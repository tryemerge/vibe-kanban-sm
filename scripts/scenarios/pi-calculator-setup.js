#!/usr/bin/env node
'use strict';

/**
 * Pi Calculator — Live Context Test
 *
 * This script sets up a real project with a 4-column board
 * (Backlog → Plan → Develop → Complete), creates 3 tasks
 * with triggers so they chain automatically, then kicks off
 * the first task.
 *
 * The goal is to observe what actually happens when real agents
 * run through the workflow — what context they receive, what
 * they produce, and whether each step sets up the next one
 * for success.
 *
 * Usage:
 *   node scripts/scenarios/pi-calculator-setup.js
 *
 * After running, watch the agents work in the UI and use the
 * reporter to capture context snapshots:
 *   node scripts/scenarios/pi-calculator-report.js <project-id>
 */

const api = require('../lib/api-client');

const PI_REPO_PATH = '/tmp/pi-calculator';

// Agent IDs — using Strategic Planner for Plan, Developer for Develop
// These are the first instances from list_agents
const PLANNER_AGENT_ID = '4a78808f-0b5d-4efc-bf08-ce5989035dd3'; // Strategic Planner
const DEVELOPER_AGENT_ID = 'eb0e075c-c070-4a7d-9304-6489f4751a43'; // Developer

async function main() {
  console.log('');
  console.log('═══ Pi Calculator — Live Context Test ═══');
  console.log('  Setting up a real project to observe context flow through agents.');
  console.log('');

  // 1. Create project
  console.log('── Step 1: Create Project ──');
  const project = await api.createProject('Pi Calculator Test', [
    { display_name: 'pi-calculator', git_repo_path: PI_REPO_PATH },
  ]);
  const projectId = project.id;
  console.log(`  ✓ Project created (${projectId})`);

  // 2. Create board: Backlog → Plan → Develop → Complete
  console.log('');
  console.log('── Step 2: Create Board ──');
  const board = await api.createBoard('Plan-Develop Pipeline', 'Backlog → Plan → Develop → Complete. Plan agent designs, Develop agent builds.');
  const boardId = board.id;
  console.log(`  ✓ Board created (${boardId})`);

  // Assign board to project
  await api.updateProject(projectId, { board_id: boardId });
  console.log('  ✓ Board assigned to project');

  // Create columns
  const backlog = await api.createColumn(boardId, {
    name: 'Backlog',
    slug: 'backlog',
    status: 'todo',
    position: 0,
    is_initial: true,
  });
  console.log(`  ✓ Column: Backlog [todo, initial]`);

  const plan = await api.createColumn(boardId, {
    name: 'Plan',
    slug: 'plan',
    status: 'inprogress',
    position: 1,
    starts_workflow: true,
    agent_id: PLANNER_AGENT_ID,
    deliverable: 'Create an implementation plan for this task ONLY. Do not plan for other tasks or the overall project. Describe the approach, the files that will need to be created or modified, and any key decisions. Write your plan clearly so a developer agent can execute it in the next stage.',
    deliverable_variable: 'decision',
    deliverable_options: '["done"]',
  });
  console.log(`  ✓ Column: Plan [inprogress, starts_workflow, Strategic Planner agent]`);

  const develop = await api.createColumn(boardId, {
    name: 'Develop',
    slug: 'develop',
    status: 'inprogress',
    position: 2,
    starts_workflow: true,
    agent_id: DEVELOPER_AGENT_ID,
    deliverable: 'Implement the task according to the plan from the previous stage. Write working code, create necessary files, and ensure the implementation is complete.',
    deliverable_variable: 'decision',
    deliverable_options: '["done"]',
  });
  console.log(`  ✓ Column: Develop [inprogress, starts_workflow, Developer agent]`);

  const complete = await api.createColumn(boardId, {
    name: 'Complete',
    slug: 'complete',
    status: 'done',
    position: 3,
    is_terminal: true,
  });
  console.log(`  ✓ Column: Complete [done, terminal]`);

  // Create transitions: Backlog → Plan → Develop → Complete
  await api.createTransition(boardId, {
    from_column_id: backlog.id,
    to_column_id: plan.id,
    name: 'Start Planning',
  });
  console.log(`  ✓ Transition: Backlog → Plan`);

  await api.createTransition(boardId, {
    from_column_id: plan.id,
    to_column_id: develop.id,
    name: 'Start Development',
    condition_key: 'decision',
    condition_value: 'done',
  });
  console.log(`  ✓ Transition: Plan → Develop (on decision=done)`);

  await api.createTransition(boardId, {
    from_column_id: develop.id,
    to_column_id: complete.id,
    name: 'Mark Complete',
    condition_key: 'decision',
    condition_value: 'done',
  });
  console.log(`  ✓ Transition: Develop → Complete (on decision=done)`);

  // 3. Create tasks
  console.log('');
  console.log('── Step 3: Create Tasks ──');

  const task1 = await api.createTask({
    project_id: projectId,
    title: 'Build the Pi calculation engine',
    description: `Create a Pi digit calculation engine that can compute Pi digits incrementally.

Requirements:
- Use a suitable algorithm (e.g., Bailey–Borwein–Plouffe or a spigot algorithm) that can compute Pi digits one at a time
- The engine should be able to pause and resume calculation from where it left off
- Store the current state (computed digits so far, algorithm state) so it can be resumed
- Export a clean API: start(), stop(), getDigits(), getState(), resume(state)
- Write this as a standalone module that the UI will import later

The starting display should be "3.14" and then each new digit gets appended.`,
  });
  console.log(`  ✓ Task 1: Build the Pi calculation engine (${task1.id})`);

  const task2 = await api.createTask({
    project_id: projectId,
    title: 'Build the UI with start/stop controls',
    description: `Create a simple web UI for the Pi calculator.

Requirements:
- Display the current Pi digits computed so far (starting from "3.14")
- A "Start" button that begins computation
- A "Stop" button that pauses computation
- When stopped and started again, computation resumes from where it left off
- Show a visual indicator that computation is running
- Use vanilla HTML/CSS/JS or a simple framework — keep it minimal
- The UI should import and use the calculation engine from Task 1`,
  });
  console.log(`  ✓ Task 2: Build the UI with start/stop controls (${task2.id})`);

  const task3 = await api.createTask({
    project_id: projectId,
    title: 'Integrate engine and UI with state persistence',
    description: `Wire the Pi calculation engine to the UI and add state persistence.

Requirements:
- Connect the calculation engine to the UI controls
- When the user clicks Stop, save the current state (digits computed, algorithm state) to localStorage
- When the page reloads, restore the previous state and display the digits computed so far
- When the user clicks Start after a reload, resume computation from the saved state
- Add a "Reset" button that clears saved state and starts fresh from "3.14"
- Ensure the display updates smoothly as new digits are computed`,
  });
  console.log(`  ✓ Task 3: Integrate engine and UI with state persistence (${task3.id})`);

  // 4. Create triggers: Task 1 → Task 2 → Task 3
  console.log('');
  console.log('── Step 4: Wire Triggers ──');

  await api.createTrigger(task2.id, {
    trigger_task_id: task1.id,
    trigger_on: 'completed',
  });
  console.log(`  ✓ Trigger: Task 1 completion → starts Task 2`);

  await api.createTrigger(task3.id, {
    trigger_task_id: task2.id,
    trigger_on: 'completed',
  });
  console.log(`  ✓ Trigger: Task 2 completion → starts Task 3`);

  // 5. Summary
  console.log('');
  console.log('═══ Setup Complete ═══');
  console.log('');
  console.log('  Project ID: ' + projectId);
  console.log('  Board: Backlog → Plan → Develop → Complete');
  console.log('  Tasks: 3 (chained via triggers)');
  console.log('');
  console.log('  What happens next:');
  console.log('  1. Move Task 1 from Backlog to Plan (drag in UI or use API)');
  console.log('     This starts the Strategic Planner agent on Task 1');
  console.log('  2. When Plan agent completes (decision=complete), Task 1 auto-moves to Develop');
  console.log('  3. Developer agent builds it, completes → Task 1 moves to Complete');
  console.log('  4. Task 1 completion triggers Task 2 → auto-moves to Plan');
  console.log('  5. Repeat for Task 2, then Task 3');
  console.log('');
  console.log('  To observe context at any point:');
  console.log(`    node scripts/scenarios/pi-calculator-report.js ${projectId}`);
  console.log('');
  console.log('  To teardown:');
  console.log(`    node scripts/scenarios/teardown-scenario.js ${projectId}`);
  console.log('');
}

main().catch(e => {
  console.error(`\n  ✗ Setup failed: ${e.message}`);
  process.exit(1);
});
