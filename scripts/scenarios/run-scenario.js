#!/usr/bin/env node
'use strict';

const path = require('path');
const fs = require('fs');
const api = require('../lib/api-client');

// ── Colors ──
const C = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  red: '\x1b[31m',
  magenta: '\x1b[35m',
};

function ok(msg) { console.log(`  ${C.green}✓${C.reset} ${msg}`); }
function info(msg) { console.log(`  ${C.dim}${msg}${C.reset}`); }
function warn(msg) { console.log(`  ${C.yellow}⚠ ${msg}${C.reset}`); }
function header(msg) { console.log(`\n${C.bold}${C.blue}── ${msg} ──${C.reset}`); }
function bigHeader(msg) { console.log(`\n${C.bold}═══ ${msg} ═══${C.reset}`); }

// ── Main ──
async function main() {
  const scenarioName = process.argv[2];
  if (!scenarioName) {
    console.error('Usage: node run-scenario.js <scenario-name>');
    console.error('  Example: node run-scenario.js baseline-api-tasks');
    process.exit(1);
  }

  const defPath = path.join(__dirname, 'definitions', `${scenarioName}.json`);
  if (!fs.existsSync(defPath)) {
    console.error(`Scenario not found: ${defPath}`);
    process.exit(1);
  }

  const scenario = JSON.parse(fs.readFileSync(defPath, 'utf8'));
  bigHeader(`InDusk Test Scenario: ${scenario.name}`);
  if (scenario.description) info(scenario.description);

  // State tracking
  const state = {
    projectId: null,
    boardId: null,
    columns: {},    // slug → column object
    taskRefs: {},   // task_ref → task object
    labelRefs: {},  // label name → label object
  };

  // ── Step 1: Create project ──
  header('Setup: Project');
  const project = await api.createProject(`Test: ${scenario.name}`);
  state.projectId = project.id;
  ok(`Project created (${project.id})`);

  // ── Step 2: Create board ──
  if (scenario.board) {
    header('Setup: Board');
    const board = await api.createBoard(
      scenario.board.name || 'Test Board',
      scenario.board.description
    );
    state.boardId = board.id;
    ok(`Board created: ${board.name} (${board.id})`);

    // Assign board to project
    await api.updateProject(state.projectId, { board_id: board.id });
    ok('Board assigned to project');

    // Create columns
    if (scenario.board.columns) {
      for (let i = 0; i < scenario.board.columns.length; i++) {
        const colDef = scenario.board.columns[i];
        const col = await api.createColumn(board.id, {
          name: colDef.name,
          slug: colDef.slug,
          position: i,
          color: colDef.color || null,
          status: colDef.status || 'todo',
          is_initial: colDef.is_initial || false,
          is_terminal: colDef.is_terminal || false,
          starts_workflow: colDef.starts_workflow || false,
          deliverable: colDef.deliverable || null,
          deliverable_variable: colDef.deliverable_variable || null,
          deliverable_options: colDef.deliverable_options
            ? JSON.stringify(colDef.deliverable_options)
            : null,
        });
        state.columns[colDef.slug] = col;
        ok(`Column: ${col.name} [${colDef.status}]`);
      }
    }

    // Create transitions
    if (scenario.board.transitions) {
      for (const tDef of scenario.board.transitions) {
        const fromCol = state.columns[tDef.from];
        const toCol = state.columns[tDef.to];
        if (!fromCol || !toCol) {
          warn(`Transition skipped: unknown column slug (${tDef.from} → ${tDef.to})`);
          continue;
        }
        const tData = {
          from_column_id: fromCol.id,
          to_column_id: toCol.id,
          name: tDef.name || null,
          condition_key: tDef.condition_key || null,
          condition_value: tDef.condition_value || null,
          max_failures: tDef.max_failures || null,
        };
        await api.createTransition(board.id, tData);
        const condStr = tDef.condition_key
          ? ` (${tDef.condition_key}=${tDef.condition_value})`
          : '';
        ok(`Transition: ${tDef.from} → ${tDef.to}${condStr}`);
      }
    }
  }

  // ── Step 3: Execute steps ──
  header('Execution');
  for (let i = 0; i < scenario.steps.length; i++) {
    const step = scenario.steps[i];

    switch (step.action) {
      case 'create_task':
        await handleCreateTask(state, step);
        break;
      case 'simulate_agent_output':
        await handleSimulateOutput(state, step);
        break;
      case 'checkpoint':
        await handleCheckpoint(state, step);
        break;
      default:
        warn(`Unknown step action: ${step.action}`);
    }
  }

  // ── Done ──
  bigHeader('Scenario Complete');
  console.log(`  Project ID: ${C.cyan}${state.projectId}${C.reset}`);
  console.log(`  Teardown:   ${C.dim}node scripts/scenarios/teardown-scenario.js ${state.projectId}${C.reset}`);
  console.log();
}

// ── Step Handlers ──

async function handleCreateTask(state, step) {
  console.log(`\n  ${C.bold}Task: ${step.title}${C.reset}`);

  const task = await api.createTask({
    project_id: state.projectId,
    title: step.title,
    description: step.description || '',
  });
  state.taskRefs[step.task_ref] = task;
  ok(`Created (${task.id})`);

  // Handle labels
  if (step.labels && step.labels.length > 0) {
    for (const labelName of step.labels) {
      // Create label if we haven't seen it
      if (!state.labelRefs[labelName]) {
        try {
          const label = await api.createLabel(state.projectId, {
            name: labelName,
            color: hashColor(labelName),
          });
          state.labelRefs[labelName] = label;
        } catch (e) {
          // Label may already exist, try to continue
          warn(`Label creation note: ${e.message}`);
        }
      }
      // Assign label to task
      if (state.labelRefs[labelName]) {
        try {
          await api.assignLabel(task.id, state.labelRefs[labelName].id);
        } catch (e) {
          warn(`Label assign note: ${e.message}`);
        }
      }
    }
    info(`Labels: ${step.labels.join(', ')}`);
  }
}

async function handleSimulateOutput(state, step) {
  const task = state.taskRefs[step.task_ref];
  if (!task) {
    warn(`simulate_agent_output: unknown task_ref "${step.task_ref}"`);
    return;
  }

  console.log(`\n  ${C.bold}Simulate: ${step.description || step.task_ref}${C.reset}`);

  // Create artifacts
  if (step.artifacts && step.artifacts.length > 0) {
    for (const artDef of step.artifacts) {
      const metadata = artDef.metadata ? JSON.stringify(artDef.metadata) : null;
      const artifact = await api.createArtifact({
        project_id: state.projectId,
        artifact_type: artDef.artifact_type,
        title: artDef.title,
        content: artDef.content,
        scope: artDef.scope || 'global',
        path: artDef.path || null,
        source_task_id: task.id,
        metadata,
      });
      const tokens = artifact.token_estimate || Math.round(artDef.content.length / 4);
      ok(`Artifact: ${artDef.title} [${artDef.artifact_type}, ${artDef.scope || 'global'}, ~${tokens} tokens]`);
    }
  } else {
    info('No artifacts produced');
  }

  // Log decision
  if (step.decision) {
    const decStr = JSON.stringify(step.decision);
    if (step.decision.feedback) {
      info(`Decision: ${step.decision.decision || '?'} — feedback: "${step.decision.feedback}"`);
    } else {
      info(`Decision: ${decStr}`);
    }
  }
}

async function handleCheckpoint(state, step) {
  console.log(`\n  ${C.bold}${C.magenta}▶ Checkpoint: ${step.name}${C.reset}`);

  for (const m of step.measurements) {
    switch (m.type) {
      case 'artifact_count': {
        const artifacts = await api.listArtifacts(state.projectId);
        console.log(`    ${C.cyan}Artifacts in project:${C.reset} ${artifacts.length}`);
        if (artifacts.length > 0) {
          for (const a of artifacts) {
            console.log(`      [${a.scope}] ${a.artifact_type}: ${a.title} (~${a.token_estimate} tokens)`);
          }
        }
        break;
      }
      case 'context_preview': {
        const taskRef = m.for_task;
        const task = state.taskRefs[taskRef];
        if (!task) {
          warn(`context_preview: unknown task_ref "${taskRef}"`);
          break;
        }
        console.log(`    ${C.cyan}Context preview for "${task.title}":${C.reset}`);
        if (m.description) console.log(`    ${C.dim}${m.description}${C.reset}`);
        try {
          const preview = await api.previewContext(state.projectId, task.id);
          console.log(`      Budget: ${preview.tokens_used}/${preview.token_budget} tokens (${Math.round(preview.tokens_used / preview.token_budget * 100)}%)`);
          console.log(`      Artifacts included: ${preview.artifacts_included} of ${preview.artifacts_total}`);
          if (preview.context) {
            // Show first few lines of context
            const lines = preview.context.split('\n').filter(l => l.trim());
            const previewLines = lines.slice(0, 8);
            console.log(`      ${C.dim}--- context preview ---${C.reset}`);
            for (const line of previewLines) {
              console.log(`      ${C.dim}${line}${C.reset}`);
            }
            if (lines.length > 8) {
              console.log(`      ${C.dim}... (${lines.length - 8} more lines)${C.reset}`);
            }
            console.log(`      ${C.dim}--- end preview ---${C.reset}`);
          }
        } catch (e) {
          warn(`Context preview failed: ${e.message}`);
        }
        break;
      }
      default:
        warn(`Unknown measurement type: ${m.type}`);
    }
  }
}

// ── Helpers ──

function hashColor(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  const h = Math.abs(hash) % 360;
  return `hsl(${h}, 60%, 45%)`;
}

// ── Run ──
main().catch(err => {
  console.error(`\n${C.red}Scenario failed: ${err.message}${C.reset}`);
  if (err.stack) console.error(C.dim + err.stack + C.reset);
  process.exit(1);
});
