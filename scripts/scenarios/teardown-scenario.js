#!/usr/bin/env node
'use strict';

const api = require('../lib/api-client');

async function main() {
  const projectId = process.argv[2];
  if (!projectId) {
    console.error('Usage: node teardown-scenario.js <project-id>');
    process.exit(1);
  }

  try {
    await api.deleteProject(projectId);
    console.log(`Project ${projectId} deleted.`);
  } catch (e) {
    console.error(`Failed to delete project: ${e.message}`);
    process.exit(1);
  }
}

main();
