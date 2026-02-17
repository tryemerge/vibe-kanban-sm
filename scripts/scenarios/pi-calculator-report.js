#!/usr/bin/env node
'use strict';

/**
 * Pi Calculator — Context Reporter
 *
 * Run this at any point during the Pi Calculator test to see:
 * - Current state of each task (which column, what status)
 * - What artifacts exist in the project
 * - What context each task would receive if it started right now
 * - Workflow history (what happened in prior columns)
 *
 * This is the observation tool. Run it after each agent completes
 * a column to see how context compounds.
 *
 * Usage:
 *   node scripts/scenarios/pi-calculator-report.js <project-id>
 */

const api = require('../lib/api-client');

const BASE_URL = `http://localhost:${process.env.BACKEND_PORT || 3501}`;

async function request(method, path) {
  const url = `${BASE_URL}${path}`;
  const res = await fetch(url, { method, headers: { 'Content-Type': 'application/json' } });
  const json = await res.json();
  if (!res.ok || json.success === false) {
    throw new Error(json.message || json.error || `${method} ${path} failed (${res.status})`);
  }
  return json.data !== undefined ? json.data : json;
}

async function getTaskDetails(taskId) {
  return request('GET', `/api/tasks/${taskId}`);
}

async function getTaskAttempts(taskId) {
  return request('GET', `/api/tasks/${taskId}/attempts`);
}

async function main() {
  const projectId = process.argv[2];
  if (!projectId) {
    console.error('Usage: node pi-calculator-report.js <project-id>');
    process.exit(1);
  }

  console.log('');
  console.log('═══ Pi Calculator — Context Report ═══');
  console.log(`  Project: ${projectId}`);
  console.log(`  Time: ${new Date().toLocaleTimeString()}`);
  console.log('');

  // 1. Get all tasks
  const tasks = await request('GET', `/api/tasks?project_id=${projectId}`);
  if (!tasks || tasks.length === 0) {
    console.log('  No tasks found.');
    return;
  }

  // 2. Task status overview
  console.log('── Task Status ──');
  for (const task of tasks) {
    const statusIcon = {
      'todo': '⬜',
      'inprogress': '🔄',
      'inreview': '👀',
      'done': '✅',
      'cancelled': '❌',
    }[task.status] || '❓';

    const agentStatus = task.agent_status ? ` [agent: ${task.agent_status}]` : '';
    const columnInfo = task.column_name ? ` → ${task.column_name}` : '';
    console.log(`  ${statusIcon} ${task.title}${columnInfo}${agentStatus}`);
    console.log(`     ID: ${task.id}`);
  }

  // 3. Artifacts in the project
  console.log('');
  console.log('── Artifacts Created ──');
  const artifacts = await api.listArtifacts(projectId);
  if (!artifacts || artifacts.length === 0) {
    console.log('  None yet.');
  } else {
    for (const art of artifacts) {
      const scopeTag = `[${art.scope}]`;
      const tokens = art.token_estimate ? `~${art.token_estimate} tokens` : '';
      console.log(`  ${scopeTag} ${art.artifact_type}: ${art.title} (${tokens})`);
      // Show first 3 lines of content
      const lines = art.content.split('\n').slice(0, 3);
      for (const line of lines) {
        if (line.trim()) console.log(`    │ ${line.trim()}`);
      }
      if (art.content.split('\n').length > 3) {
        console.log(`    │ ... (${art.content.split('\n').length} lines total)`);
      }
      console.log('');
    }
  }

  // 4. For each task, show what context it would receive
  console.log('── Context Preview Per Task ──');
  for (const task of tasks) {
    console.log(`  ┌─ ${task.title}`);
    console.log(`  │  Status: ${task.status}${task.column_name ? ', Column: ' + task.column_name : ''}`);

    try {
      const preview = await api.previewContext(projectId, task.id);
      if (preview && preview.context) {
        const contextLines = preview.context.split('\n');
        const tokenInfo = preview.stats
          ? `${preview.stats.total_tokens}/${preview.stats.budget} tokens (${Math.round(preview.stats.total_tokens / preview.stats.budget * 100)}%)`
          : 'unknown';
        const artifactCount = preview.stats
          ? `${preview.stats.included_count}/${preview.stats.total_count} artifacts`
          : 'unknown';

        console.log(`  │  Context: ${tokenInfo}, ${artifactCount}`);
        console.log(`  │`);

        // Show the full context with indentation
        for (const line of contextLines) {
          console.log(`  │  ${line}`);
        }
      } else {
        console.log(`  │  Context: empty (no artifacts match)`);
      }
    } catch (e) {
      console.log(`  │  Context preview error: ${e.message}`);
    }

    // Show workflow history via task events
    try {
      const events = await api.listTaskEvents(task.id);
      if (events && events.length > 0) {
        console.log(`  │`);
        console.log(`  │  Event Log (${events.length} events):`);
        for (const event of events.slice(-10)) { // last 10 events
          const time = new Date(event.created_at).toLocaleTimeString();
          const type = event.event_type;
          const col = event.column_name || '';
          const commit = event.commit_message ? `: ${event.commit_message}` : '';
          console.log(`  │    ${time} ${type} ${col}${commit}`);
        }
      }
    } catch (e) {
      // Events endpoint might not exist or task has no events
    }

    console.log(`  └─`);
    console.log('');
  }

  // 5. Summary
  console.log('── Summary ──');
  const done = tasks.filter(t => t.status === 'done').length;
  const inProgress = tasks.filter(t => t.status === 'inprogress').length;
  const todo = tasks.filter(t => t.status === 'todo').length;
  console.log(`  Tasks: ${todo} todo, ${inProgress} in progress, ${done} done`);
  console.log(`  Artifacts: ${artifacts ? artifacts.length : 0}`);
  console.log(`  Run this command again after each agent completes to track context growth.`);
  console.log('');
}

main().catch(e => {
  console.error(`\n  ✗ Report failed: ${e.message}`);
  process.exit(1);
});
