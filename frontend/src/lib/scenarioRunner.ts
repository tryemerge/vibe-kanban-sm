/**
 * Browser-side scenario runner.
 * Mirrors the logic in scripts/scenarios/run-scenario.js but uses fetch instead of node.
 */

async function initRepo(folderName: string): Promise<boolean> {
  try {
    const res = await fetch('/api/repos/init', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ parent_path: '/tmp', folder_name: folderName }),
    });
    return res.ok;
  } catch {
    return false;
  }
}

export interface ScenarioStep {
  action: string;
  // create_task
  task_ref?: string;
  title?: string;
  description?: string;
  labels?: string[];
  // create_artifact
  artifact_ref?: string;
  artifact_type?: string;
  content?: string;
  scope?: string;
  path?: string;
  chain_id_ref?: string;
  supersedes_ref?: string;
  // create_task_group
  group_ref?: string;
  name?: string;
  color?: string;
  // add_task_to_group / finalize_task_group
  // (reuse task_ref, group_ref)
}

export interface ScenarioDef {
  name: string;
  description?: string;
  /** If set, assign this existing board to the project instead of creating a new one. */
  board_id?: string;
  /**
   * If set, a git repo is initialized at /tmp/{repo_slug} and linked to the project.
   * Defaults to a slugified version of the scenario name if not provided.
   * Pass `false` to skip repo initialization entirely.
   */
  repo_slug?: string | false;
  steps: ScenarioStep[];
}

export interface RunLog {
  level: 'info' | 'ok' | 'warn' | 'error';
  message: string;
}

async function apiFetch(method: string, path: string, body?: unknown): Promise<unknown> {
  const res = await fetch(path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let json: { success?: boolean; data?: unknown; message?: string; error?: string };
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(`${method} ${path} returned non-JSON (${res.status}): ${text.slice(0, 200)}`);
  }
  if (!res.ok || json.success === false) {
    throw new Error(json.message || json.error || `${method} ${path} failed (${res.status})`);
  }
  return json.data !== undefined ? json.data : json;
}

export async function runScenario(
  scenario: ScenarioDef,
  onLog: (log: RunLog) => void
): Promise<{ projectId: string }> {
  const log = (level: RunLog['level'], message: string) => onLog({ level, message });

  const state: {
    projectId: string;
    taskRefs: Record<string, { id: string; title: string }>;
    artifactRefs: Record<string, { id: string; chain_id: string | null }>;
    groupRefs: Record<string, { id: string; name: string }>;
  } = {
    projectId: '',
    taskRefs: {},
    artifactRefs: {},
    groupRefs: {},
  };

  // Init git repo unless explicitly disabled
  const repoSlug = scenario.repo_slug !== false
    ? (typeof scenario.repo_slug === 'string' ? scenario.repo_slug : scenario.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, ''))
    : null;

  let repoPath: string | null = null;
  if (repoSlug) {
    log('info', `Initializing git repo at /tmp/${repoSlug}`);
    const ok = await initRepo(repoSlug);
    if (ok) {
      repoPath = `/tmp/${repoSlug}`;
      log('ok', `Repo ready at ${repoPath}`);
    } else {
      log('warn', `Repo init failed for /tmp/${repoSlug} — project will have no repo`);
    }
  }

  // Create project
  log('info', `Creating project: Test: ${scenario.name}`);
  const repositories = repoPath
    ? [{ display_name: repoSlug!, git_repo_path: repoPath }]
    : [];
  const project = await apiFetch('POST', '/api/projects', {
    name: `Test: ${scenario.name}`,
    repositories,
    board_id: scenario.board_id ?? null,
  }) as { id: string };
  state.projectId = project.id;
  log('ok', `Project created (${project.id})${scenario.board_id ? ` — board: ${scenario.board_id}` : ''}`);

  // Execute steps
  for (const step of scenario.steps) {
    switch (step.action) {
      case 'create_task': {
        const task = await apiFetch('POST', '/api/tasks', {
          project_id: state.projectId,
          title: step.title,
          description: step.description || '',
        }) as { id: string; title: string };
        if (step.task_ref) state.taskRefs[step.task_ref] = task;
        log('ok', `Task: ${task.title} (${task.id})`);
        break;
      }

      case 'create_artifact': {
        const chainIdRef = step.chain_id_ref ? state.artifactRefs[step.chain_id_ref] : null;
        const artifact = await apiFetch('POST', '/api/context-artifacts', {
          project_id: state.projectId,
          artifact_type: step.artifact_type,
          title: step.title,
          content: step.content,
          scope: step.scope || 'global',
          path: step.path || null,
          chain_id: chainIdRef?.chain_id || null,
          supersedes_id: step.supersedes_ref ? state.artifactRefs[step.supersedes_ref]?.id : null,
        }) as { id: string; chain_id: string | null; artifact_type: string; title: string };
        if (step.artifact_ref) state.artifactRefs[step.artifact_ref] = artifact;
        log('ok', `Artifact [${artifact.artifact_type}] ${artifact.title}`);
        break;
      }

      case 'create_task_group': {
        const artifactId = step.artifact_ref ? state.artifactRefs[step.artifact_ref]?.id : null;
        const group = await apiFetch('POST', `/api/projects/${state.projectId}/task-groups`, {
          name: step.name,
          color: step.color || null,
          is_backlog: false,
          artifact_id: artifactId || null,
        }) as { id: string; name: string };
        if (step.group_ref) state.groupRefs[step.group_ref] = group;
        log('ok', `Group: ${group.name}${artifactId ? ' (linked to IMPL doc)' : ''}`);
        break;
      }

      case 'add_task_to_group': {
        const task = step.task_ref ? state.taskRefs[step.task_ref] : null;
        const group = step.group_ref ? state.groupRefs[step.group_ref] : null;
        if (!task) { log('warn', `add_task_to_group: unknown task_ref "${step.task_ref}"`); break; }
        if (!group) { log('warn', `add_task_to_group: unknown group_ref "${step.group_ref}"`); break; }
        await apiFetch('POST', `/api/tasks/${task.id}/task-group/${group.id}`, {});
        log('ok', `  "${task.title}" → "${group.name}"`);
        break;
      }

      case 'finalize_task_group': {
        const group = step.group_ref ? state.groupRefs[step.group_ref] : null;
        if (!group) { log('warn', `finalize_task_group: unknown group_ref "${step.group_ref}"`); break; }
        await apiFetch('POST', `/api/task-groups/${group.id}/transition`, { from: 'draft', to: 'analyzing' });
        log('ok', `Finalized "${group.name}" → analyzing`);
        break;
      }

      default:
        log('warn', `Unknown step action: ${step.action}`);
    }
  }

  log('ok', `Done — project ${state.projectId}`);
  return { projectId: state.projectId };
}
