#!/usr/bin/env node
'use strict';

const path = require('path');

const BASE_URL = `http://localhost:${process.env.BACKEND_PORT || 3501}`;
const DEFAULT_REPO_PATH = path.resolve(__dirname, '..', '..');

async function request(method, path, body) {
  const url = `${BASE_URL}${path}`;
  const opts = {
    method,
    headers: { 'Content-Type': 'application/json' },
  };
  if (body) opts.body = JSON.stringify(body);

  const res = await fetch(url, opts);
  const text = await res.text();

  let json;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(`${method} ${path} returned non-JSON (${res.status}): ${text.slice(0, 200)}`);
  }

  if (!res.ok || json.success === false) {
    const msg = json.message || json.error || `${method} ${path} failed (${res.status})`;
    throw new Error(msg);
  }

  return json.data !== undefined ? json.data : json;
}

const api = {
  // Projects
  createProject(name, repos) {
    return request('POST', '/api/projects', {
      name,
      repositories: repos || [{ display_name: 'test-repo', git_repo_path: DEFAULT_REPO_PATH }],
    });
  },
  updateProject(id, data) {
    return request('PUT', `/api/projects/${id}`, data);
  },
  deleteProject(id) {
    return request('DELETE', `/api/projects/${id}`);
  },

  // Boards
  createBoard(name, description) {
    return request('POST', '/api/boards', { name, description });
  },

  // Columns
  createColumn(boardId, data) {
    return request('POST', `/api/boards/${boardId}/columns`, data);
  },

  // Transitions
  createTransition(boardId, data) {
    return request('POST', `/api/boards/${boardId}/transitions`, data);
  },

  // Tasks
  createTask(data) {
    return request('POST', '/api/tasks', data);
  },

  // Artifacts
  createArtifact(data) {
    return request('POST', '/api/context-artifacts', data);
  },
  listArtifacts(projectId, artifactType) {
    let path = `/api/context-artifacts?project_id=${projectId}`;
    if (artifactType) path += `&artifact_type=${artifactType}`;
    return request('GET', path);
  },
  previewContext(projectId, taskId) {
    let path = `/api/context-artifacts/preview-context?project_id=${projectId}`;
    if (taskId) path += `&task_id=${taskId}`;
    return request('GET', path);
  },

  // Triggers
  createTrigger(taskId, data) {
    return request('POST', `/api/tasks/${taskId}/triggers`, { task_id: taskId, ...data });
  },
  listTriggers(taskId) {
    return request('GET', `/api/tasks/${taskId}/triggers`);
  },

  // Task events
  listTaskEvents(taskId) {
    return request('GET', `/api/tasks/${taskId}/events`);
  },

  // Labels
  createLabel(projectId, data) {
    return request('POST', `/api/projects/${projectId}/labels`, data);
  },
  assignLabel(taskId, labelId) {
    return request('POST', `/api/tasks/${taskId}/labels/${labelId}`, {});
  },
};

module.exports = api;
