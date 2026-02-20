import { useState, useCallback } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import {
  boardsApi,
  contextArtifactsApi,
  attemptsApi,
  evaluateRunsApi,
  projectsApi,
  repoApi,
  taskEventsApi,
  taskDependenciesApi,
  taskTriggersApi,
  tasksApi,
} from '@/lib/api';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Loader2,
  Square,
  CheckCircle2,
  XCircle,
  Clock,
  Eye,
  ChevronDown,
  ChevronRight,
  Trash2,
  Play,
  History,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  TaskWithAttemptStatus,
  ContextArtifact,
  TaskEventWithNames,
  Board,
  EvaluateRun,
  EvaluateRunSummary,
  JsonValue,
} from 'shared/types';

const STORAGE_KEY = 'evaluate-test-project';

interface TestCase {
  id: string;
  name: string;
  description: string;
  projectName: string;
}

const TEST_CASES: TestCase[] = [
  {
    id: 'counter',
    name: 'Counter App',
    description:
      'Single task: build a page with a button that increments a number.',
    projectName: 'Counter App Test',
  },
  {
    id: 'pi-calculator',
    name: 'Pi Calculator',
    description:
      '3 chained tasks: calculation engine, UI, integration with state persistence.',
    projectName: 'Pi Calculator Test',
  },
];

interface StoredTestProject {
  projectId: string;
  boardId?: string | null; // deprecated — boards are global
  createdAt: string;
  testCaseName: string;
}

function loadStoredProject(): StoredTestProject | null {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    // Check legacy key (plain string ID)
    const legacyId = localStorage.getItem('evaluate-test-project-id');
    if (legacyId) {
      const stored: StoredTestProject = {
        projectId: legacyId,
        boardId: null,
        createdAt: new Date().toISOString(),
        testCaseName: 'Pi Calculator Test',
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
      localStorage.removeItem('evaluate-test-project-id');
      return stored;
    }
    return null;
  }
  try {
    return JSON.parse(raw) as StoredTestProject;
  } catch {
    return null;
  }
}

const STATUS_ICONS: Record<string, typeof Square> = {
  todo: Square,
  inprogress: Loader2,
  inreview: Eye,
  done: CheckCircle2,
  cancelled: XCircle,
};

const TYPE_COLORS: Record<string, string> = {
  adr: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  pattern: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  iplan: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
  module_memory: 'bg-green-500/10 text-green-600 border-green-500/20',
  decision: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  dependency: 'bg-slate-500/10 text-slate-600 border-slate-500/20',
  changelog_entry: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
};

const SCOPE_COLORS: Record<string, string> = {
  global: 'bg-emerald-500/10 text-emerald-600 border-emerald-500/20',
  task: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
  path: 'bg-sky-500/10 text-sky-600 border-sky-500/20',
};

function TaskStatusRow({ task }: { task: TaskWithAttemptStatus }) {
  const Icon = STATUS_ICONS[task.status] || Clock;
  const isSpinning = task.status === 'inprogress';

  return (
    <div className="flex items-center gap-3 py-2 px-3 rounded border">
      <Icon
        className={cn('h-4 w-4 flex-shrink-0', isSpinning && 'animate-spin')}
      />
      <span className="text-sm font-medium flex-1 truncate">{task.title}</span>
      <Badge variant="outline" className="text-[10px]">
        {task.status}
      </Badge>
      {task.task_state && task.task_state !== 'queued' && (
        <Badge variant="secondary" className="text-[10px]">
          {task.task_state}
        </Badge>
      )}
    </div>
  );
}

function ArtifactRow({ artifact }: { artifact: ContextArtifact }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border rounded-lg">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 p-3 w-full text-left hover:bg-accent/50 rounded transition-colors"
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
        )}
        <Badge
          variant="outline"
          className={cn(
            'text-[10px] px-1.5 py-0 flex-shrink-0',
            TYPE_COLORS[artifact.artifact_type] || ''
          )}
        >
          {artifact.artifact_type}
        </Badge>
        <Badge
          variant="outline"
          className={cn(
            'text-[10px] px-1.5 py-0 flex-shrink-0',
            SCOPE_COLORS[artifact.scope] || ''
          )}
        >
          {artifact.scope}
        </Badge>
        <span className="text-sm font-medium truncate flex-1">
          {artifact.title}
        </span>
        <span className="text-xs text-muted-foreground flex-shrink-0">
          ~{artifact.token_estimate} tok
        </span>
      </button>
      {expanded && (
        <div className="px-3 pb-3 border-t">
          <pre className="mt-2 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/50 rounded p-2 max-h-64 overflow-auto">
            {artifact.content}
          </pre>
        </div>
      )}
    </div>
  );
}

function EventRow({ event }: { event: TaskEventWithNames }) {
  const time = new Date(event.created_at).toLocaleTimeString();
  return (
    <div className="flex items-center gap-3 py-1.5 text-xs font-mono">
      <span className="text-muted-foreground w-20 flex-shrink-0">{time}</span>
      <Badge variant="outline" className="text-[10px] px-1.5 py-0">
        {event.event_type}
      </Badge>
      {event.to_column_name && (
        <span className="text-muted-foreground">{event.to_column_name}</span>
      )}
      {event.commit_message && (
        <span className="text-muted-foreground truncate flex-1">
          {event.commit_message}
        </span>
      )}
    </div>
  );
}

function RunHistoryRow({
  run,
  onDelete,
}: {
  run: EvaluateRun;
  onDelete: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const summary = run.summary as EvaluateRunSummary;
  const startedAt = new Date(run.started_at);
  const completedAt = new Date(run.completed_at);
  const durationMs = completedAt.getTime() - startedAt.getTime();
  const durationMin = Math.round(durationMs / 60000);

  return (
    <div className="border rounded-lg">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-3 p-3 w-full text-left hover:bg-accent/50 rounded transition-colors"
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
        )}
        <span className="text-xs text-muted-foreground w-28 flex-shrink-0">
          {startedAt.toLocaleDateString()}{' '}
          {startedAt.toLocaleTimeString([], {
            hour: '2-digit',
            minute: '2-digit',
          })}
        </span>
        {run.commit_hash && (
          <code className="text-[10px] text-muted-foreground font-mono flex-shrink-0">
            {run.commit_hash.slice(0, 7)}
          </code>
        )}
        <span className="text-sm flex-1 truncate">
          {summary.stats.tasks_completed}/{summary.stats.total_tasks} tasks
        </span>
        <span className="text-xs text-muted-foreground flex-shrink-0">
          {summary.stats.total_artifacts} artifacts
        </span>
        <span className="text-xs text-muted-foreground flex-shrink-0">
          {summary.stats.total_tokens.toLocaleString()} tok
        </span>
        <span className="text-xs text-muted-foreground flex-shrink-0">
          {durationMin}m
        </span>
      </button>
      {expanded && (
        <div className="px-3 pb-3 border-t space-y-3">
          {run.commit_message && (
            <p className="text-xs text-muted-foreground mt-2 font-mono">
              {run.commit_message}
            </p>
          )}
          {run.notes && (
            <p className="text-sm mt-2 italic text-muted-foreground">
              {run.notes}
            </p>
          )}
          {/* Task breakdown with agent summaries */}
          <div>
            <p className="text-xs font-medium mb-1">Tasks</p>
            <div className="space-y-2">
              {summary.tasks.map((task, i) => (
                <div key={i} className="space-y-1">
                  <div className="flex items-center gap-2 text-xs">
                    <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                      {task.status}
                    </Badge>
                    {/* Show task_state, falling back to agent_status for old runs */}
                    {(() => {
                      const state = task.task_state || (task as Record<string, unknown>).agent_status as string | undefined;
                      return state && state !== 'queued' ? (
                        <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                          {state}
                        </Badge>
                      ) : null;
                    })()}
                    <span className="truncate">{task.title}</span>
                  </div>
                  {task.attempts?.map((attempt, j) =>
                    attempt.completion_summary || attempt.final_context ? (
                      <div
                        key={j}
                        className="ml-6 text-xs text-muted-foreground bg-muted/50 rounded p-2 space-y-1"
                      >
                        {attempt.completion_summary && (
                          <p>{attempt.completion_summary}</p>
                        )}
                        {attempt.final_context && (
                          <pre className="whitespace-pre-wrap font-mono text-[10px] max-h-32 overflow-auto">
                            {attempt.final_context}
                          </pre>
                        )}
                      </div>
                    ) : null
                  )}
                </div>
              ))}
            </div>
          </div>
          {/* Artifact breakdown */}
          {summary.artifacts.length > 0 && (
            <div>
              <p className="text-xs font-medium mb-1">Artifacts</p>
              <div className="space-y-1">
                {summary.artifacts.map((art, i) => (
                  <div key={i} className="flex items-center gap-2 text-xs">
                    <Badge
                      variant="outline"
                      className={cn(
                        'text-[10px] px-1.5 py-0',
                        TYPE_COLORS[art.artifact_type] || ''
                      )}
                    >
                      {art.artifact_type}
                    </Badge>
                    <span className="truncate flex-1">{art.title}</span>
                    <span className="text-muted-foreground">
                      ~{art.token_estimate} tok
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className="flex justify-end">
            <Button
              variant="ghost"
              size="sm"
              className="text-destructive hover:text-destructive"
              onClick={(e) => {
                e.stopPropagation();
                onDelete(run.id);
              }}
            >
              <Trash2 className="h-3.5 w-3.5 mr-1" />
              Delete
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

export function EvaluateSettings() {
  const { t } = useTranslation('settings');
  const queryClient = useQueryClient();
  const [storedProject, setStoredProject] = useState<StoredTestProject | null>(
    loadStoredProject
  );
  const testProjectId = storedProject?.projectId ?? null;
  const [creating, setCreating] = useState(false);
  const [destroying, setDestroying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTaskId, setSelectedTaskId] = useState<string | undefined>();
  const [notes, setNotes] = useState('');
  const [selectedTestCase, setSelectedTestCase] = useState('counter');
  const [selectedBoardId, setSelectedBoardId] = useState<string>('');

  // Fetch available boards
  const { data: boards = [] } = useQuery({
    queryKey: ['boards'],
    queryFn: () => boardsApi.list(),
  });

  // Auto-select first board
  if (boards.length > 0 && !selectedBoardId) {
    setSelectedBoardId(boards[0].id);
  }

  // Fetch run history (always, not gated by active project)
  const { data: runs = [] } = useQuery({
    queryKey: ['evaluate-runs'],
    queryFn: () => evaluateRunsApi.list(),
  });

  // Fetch tasks for the test project
  const { data: tasks = [], isLoading: tasksLoading } = useQuery({
    queryKey: ['evaluate-tasks', testProjectId],
    queryFn: () => tasksApi.listByProject(testProjectId!),
    enabled: !!testProjectId,
    refetchInterval: 5000,
  });

  // Fetch artifacts for the test project
  const { data: artifacts = [], isLoading: artifactsLoading } = useQuery({
    queryKey: ['evaluate-artifacts', testProjectId],
    queryFn: () => contextArtifactsApi.list(testProjectId!),
    enabled: !!testProjectId,
    refetchInterval: 10000,
  });

  // Fetch context preview for the selected task
  const { data: contextPreview, isLoading: previewLoading } = useQuery({
    queryKey: ['evaluate-preview', testProjectId, selectedTaskId],
    queryFn: () =>
      contextArtifactsApi.previewContext(testProjectId!, selectedTaskId),
    enabled: !!testProjectId && !!selectedTaskId,
  });

  // Fetch events for all tasks
  const { data: allEvents = [], isLoading: eventsLoading } = useQuery({
    queryKey: ['evaluate-events', testProjectId, tasks.map((t) => t.id)],
    queryFn: async () => {
      const results = await Promise.all(
        tasks.map((task) =>
          taskEventsApi
            .getByTaskId(task.id)
            .then((events) => events.map((e) => ({ ...e, taskTitle: task.title })))
        )
      );
      return results
        .flat()
        .sort(
          (a, b) =>
            new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
        );
    },
    enabled: !!testProjectId && tasks.length > 0,
    refetchInterval: 5000,
  });

  const totalTokens = artifacts.reduce(
    (sum, a) => sum + (a.token_estimate || 0),
    0
  );

  const handleCreate = useCallback(async () => {
    if (!selectedBoardId) {
      setError('Please select a board first');
      return;
    }
    setCreating(true);
    setError(null);

    try {
      const testCase = TEST_CASES.find((t) => t.id === selectedTestCase)!;

      // 1. Initialize git repo (creates directory + git init if not already present)
      const repoSlug = testCase.id;
      try {
        await repoApi.init({
          parent_path: '/tmp',
          folder_name: repoSlug,
        });
      } catch {
        // Repo may already exist from a prior run — that's fine
      }

      // 2. Create project
      const project = await projectsApi.create({
        name: testCase.projectName,
        repositories: [
          {
            display_name: repoSlug,
            git_repo_path: `/tmp/${repoSlug}`,
          },
        ],
      });

      // 3. Assign the selected board (replace auto-created one)
      if (project.board_id && project.board_id !== selectedBoardId) {
        await boardsApi.delete(project.board_id).catch(() => {});
      }
      await projectsApi.update(project.id, {
        name: null,
        board_id: selectedBoardId,
        dev_script: null,
        dev_script_working_dir: null,
        default_agent_working_dir: null,
      });

      // 4. Create tasks based on selected test case
      if (selectedTestCase === 'counter') {
        await tasksApi.create({
          project_id: project.id,
          title: 'Build a counter page',
          description:
            'Create a simple HTML page with a button. Every time the button is clicked, add 1 to a counter and display the updated number on the page.\n\nRequirements:\n- A single HTML file with embedded CSS and JS\n- A button labeled "Click me" or "Increment"\n- A visible counter that starts at 0\n- Each button click adds 1 to the counter\n- The updated number is displayed immediately\n- Use vanilla HTML/CSS/JS, no frameworks',
          status: null,
          column_id: null,
          parent_workspace_id: null,
          image_ids: null,
          shared_task_id: null,
          task_group_id: null,
        });
      } else {
        // Pi Calculator: 3 chained tasks with dependencies and triggers
        const task1 = await tasksApi.create({
          project_id: project.id,
          title: 'Build the Pi calculation engine',
          description:
            'Create a Pi digit calculation engine that can compute Pi digits incrementally.\n\nRequirements:\n- Use a suitable algorithm (e.g., Bailey-Borwein-Plouffe or a spigot algorithm)\n- The engine should pause and resume calculation\n- Export a clean API: start(), stop(), getDigits(), getState(), resume(state)\n- Write this as a standalone module',
          status: null,
          column_id: null,
          parent_workspace_id: null,
          image_ids: null,
          shared_task_id: null,
          task_group_id: null,
        });

        const task2 = await tasksApi.create({
          project_id: project.id,
          title: 'Build the UI with start/stop controls',
          description:
            'Create a simple web UI for the Pi calculator.\n\nRequirements:\n- Display computed Pi digits\n- Start and Stop buttons\n- Resume computation when restarted\n- Show running indicator\n- Use vanilla HTML/CSS/JS',
          status: null,
          column_id: null,
          parent_workspace_id: null,
          image_ids: null,
          shared_task_id: null,
          task_group_id: null,
        });

        const task3 = await tasksApi.create({
          project_id: project.id,
          title: 'Integrate engine and UI with state persistence',
          description:
            'Wire the Pi calculation engine to the UI and add state persistence via localStorage.\n\nRequirements:\n- Connect engine to UI controls\n- Save/restore state on stop/reload\n- Add Reset button\n- Smooth display updates',
          status: null,
          column_id: null,
          parent_workspace_id: null,
          image_ids: null,
          shared_task_id: null,
          task_group_id: null,
        });

        // Wire dependencies
        await taskDependenciesApi.create(task2.id, {
          depends_on_task_id: task1.id,
        });
        await taskDependenciesApi.create(task3.id, {
          depends_on_task_id: task2.id,
        });

        // Wire triggers
        await taskTriggersApi.create(task2.id, {
          trigger_task_id: task1.id,
          trigger_on: 'completed',
          is_persistent: false,
        });
        await taskTriggersApi.create(task3.id, {
          trigger_task_id: task2.id,
          trigger_on: 'completed',
          is_persistent: false,
        });
      }

      // Save project reference
      const stored: StoredTestProject = {
        projectId: project.id,
        createdAt: new Date().toISOString(),
        testCaseName: testCase.projectName,
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
      setStoredProject(stored);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create test project');
    } finally {
      setCreating(false);
    }
  }, [selectedTestCase, selectedBoardId]);

  const handleDestroy = useCallback(async () => {
    if (!testProjectId || !storedProject) return;
    setDestroying(true);
    setError(null);

    try {
      // Snapshot current state before destroying
      const [snapTasks, snapArtifacts] = await Promise.all([
        tasksApi.listByProject(testProjectId).catch(() => []),
        contextArtifactsApi.list(testProjectId).catch(() => []),
      ]);

      // Get events and attempts for all tasks
      const [snapEventArrays, snapAttemptArrays] = await Promise.all([
        Promise.all(
          snapTasks.map((task) =>
            taskEventsApi.getByTaskId(task.id).catch(() => [])
          )
        ),
        Promise.all(
          snapTasks.map((task) =>
            attemptsApi.getAll(task.id).catch(() => [])
          )
        ),
      ]);
      const snapEvents = snapEventArrays.flat();

      // Try to get current git commit
      let commitHash: string | null = null;
      let commitMessage: string | null = null;
      try {
        const healthRes = await fetch('/api/health');
        const health = await healthRes.json();
        commitHash = health.data?.commit_hash || null;
        commitMessage = health.data?.commit_message || null;
      } catch {
        // Not critical
      }

      // Build summary
      const summary: EvaluateRunSummary = {
        tasks: snapTasks.map((t, i) => ({
          title: t.title,
          status: t.status,
          task_state: t.task_state,
          attempts: (snapAttemptArrays[i] || []).map((a) => ({
            branch: a.branch,
            completion_summary: a.completion_summary || null,
            final_context: a.final_context || null,
          })),
        })),
        artifacts: snapArtifacts.map((a) => ({
          artifact_type: a.artifact_type,
          scope: a.scope,
          title: a.title,
          token_estimate: a.token_estimate || 0,
          content: a.content,
        })),
        events: snapEvents.map((e) => ({
          event_type: e.event_type,
          column_name: e.to_column_name || null,
          commit_message: e.commit_message || null,
          created_at:
            e.created_at instanceof Date
              ? e.created_at.toISOString()
              : String(e.created_at),
        })),
        stats: {
          total_tasks: snapTasks.length,
          tasks_completed: snapTasks.filter((t) => t.status === 'done').length,
          total_artifacts: snapArtifacts.length,
          total_tokens: snapArtifacts.reduce(
            (sum, a) => sum + (a.token_estimate || 0),
            0
          ),
          total_events: snapEvents.length,
        },
      };

      // Save run to DB
      await evaluateRunsApi.create({
        commit_hash: commitHash,
        commit_message: commitMessage,
        project_name: storedProject.testCaseName || 'Test Run',
        started_at: storedProject.createdAt,
        summary: summary as unknown as JsonValue,
        notes: notes.trim() || null,
      });

      // Destroy the project (board is global, don't delete it)
      await projectsApi.delete(testProjectId);
      localStorage.removeItem(STORAGE_KEY);
      setStoredProject(null);
      setSelectedTaskId(undefined);
      setNotes('');
      queryClient.invalidateQueries({ queryKey: ['evaluate-tasks'] });
      queryClient.invalidateQueries({ queryKey: ['evaluate-artifacts'] });
      queryClient.invalidateQueries({ queryKey: ['evaluate-events'] });
      queryClient.invalidateQueries({ queryKey: ['evaluate-runs'] });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to destroy test project');
    } finally {
      setDestroying(false);
    }
  }, [testProjectId, storedProject, queryClient]);

  const handleDeleteRun = useCallback(
    async (runId: string) => {
      try {
        await evaluateRunsApi.delete(runId);
        queryClient.invalidateQueries({ queryKey: ['evaluate-runs'] });
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to delete run'
        );
      }
    },
    [queryClient]
  );

  return (
    <div className="space-y-6">
      {/* Test Project Management */}
      <Card>
        <CardHeader>
          <CardTitle>{t('integrations.evaluate.title')}</CardTitle>
          <CardDescription>
            {t('integrations.evaluate.description')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {error && (
            <div className="text-sm text-destructive bg-destructive/10 rounded p-3">
              {error}
            </div>
          )}

          {testProjectId ? (
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <Badge variant="default" className="bg-green-600">
                  {t('integrations.evaluate.testProject.active')}
                </Badge>
                <span className="text-sm font-medium">
                  {storedProject?.testCaseName || 'Test'}
                </span>
                <code className="text-xs text-muted-foreground">
                  {testProjectId}
                </code>
                {storedProject?.createdAt && (
                  <span className="text-xs text-muted-foreground">
                    Started{' '}
                    {new Date(storedProject.createdAt).toLocaleString()}
                  </span>
                )}
              </div>
              <textarea
                className="w-full rounded border bg-background px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                placeholder="Notes about this run (e.g. what changed, what you're testing)..."
                rows={2}
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
              />
              <Button
                variant="destructive"
                size="sm"
                onClick={handleDestroy}
                disabled={destroying}
              >
                {destroying ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Trash2 className="h-4 w-4 mr-2" />
                )}
                {t('integrations.evaluate.testProject.destroyButton')}
              </Button>
            </div>
          ) : (
            <div className="space-y-3">
              <div className="space-y-2">
                <label className="text-sm font-medium">Board</label>
                <Select
                  value={selectedBoardId}
                  onValueChange={setSelectedBoardId}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select a board..." />
                  </SelectTrigger>
                  <SelectContent>
                    {boards.map((b: Board) => (
                      <SelectItem key={b.id} value={b.id}>
                        {b.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                {boards.length === 0 && (
                  <p className="text-xs text-destructive">
                    No boards found. Create a board in Board Settings first.
                  </p>
                )}
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Test Case</label>
                <Select
                  value={selectedTestCase}
                  onValueChange={setSelectedTestCase}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TEST_CASES.map((tc) => (
                      <SelectItem key={tc.id} value={tc.id}>
                        {tc.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {TEST_CASES.find((t) => t.id === selectedTestCase)
                    ?.description}
                </p>
              </div>
              <Button
                onClick={handleCreate}
                disabled={creating || !selectedBoardId}
              >
                {creating ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Play className="h-4 w-4 mr-2" />
                )}
                {creating
                  ? t('integrations.evaluate.testProject.creating')
                  : t('integrations.evaluate.testProject.createButton')}
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Only show remaining cards if a test project exists */}
      {testProjectId && (
        <>
          {/* Task Status */}
          <Card>
            <CardHeader>
              <CardTitle>{t('integrations.evaluate.tasks.title')}</CardTitle>
            </CardHeader>
            <CardContent>
              {tasksLoading ? (
                <div className="flex items-center justify-center py-6">
                  <Loader2 className="h-5 w-5 animate-spin" />
                </div>
              ) : tasks.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  {t('integrations.evaluate.tasks.empty')}
                </p>
              ) : (
                <div className="space-y-2">
                  {tasks.map((task) => (
                    <TaskStatusRow key={task.id} task={task} />
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Context Artifacts */}
          <Card>
            <CardHeader>
              <CardTitle>
                {t('integrations.evaluate.artifacts.title')}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {artifactsLoading ? (
                <div className="flex items-center justify-center py-6">
                  <Loader2 className="h-5 w-5 animate-spin" />
                </div>
              ) : artifacts.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  {t('integrations.evaluate.artifacts.empty')}
                </p>
              ) : (
                <>
                  <div className="space-y-2">
                    {artifacts.map((artifact) => (
                      <ArtifactRow key={artifact.id} artifact={artifact} />
                    ))}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {t('integrations.evaluate.artifacts.total', {
                      count: artifacts.length,
                      tokens: totalTokens.toLocaleString(),
                    })}
                  </p>
                </>
              )}
            </CardContent>
          </Card>

          {/* Context Preview */}
          <Card>
            <CardHeader>
              <CardTitle>
                {t('integrations.evaluate.contextPreview.title')}
              </CardTitle>
              <CardDescription>
                {t('integrations.evaluate.contextPreview.description')}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Select
                value={selectedTaskId || ''}
                onValueChange={(v) => setSelectedTaskId(v || undefined)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue
                    placeholder={t(
                      'integrations.evaluate.contextPreview.selectTask'
                    )}
                  />
                </SelectTrigger>
                <SelectContent>
                  {tasks.map((task) => (
                    <SelectItem key={task.id} value={task.id}>
                      {task.title}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              {selectedTaskId && (
                <>
                  {previewLoading ? (
                    <div className="flex items-center justify-center py-6">
                      <Loader2 className="h-5 w-5 animate-spin" />
                    </div>
                  ) : contextPreview ? (
                    <div className="space-y-3">
                      <div className="flex items-center gap-4 text-xs text-muted-foreground">
                        <span>
                          {t(
                            'integrations.evaluate.contextPreview.tokens',
                            {
                              used: contextPreview.tokens_used,
                              budget: contextPreview.token_budget,
                              percent: Math.round(
                                (contextPreview.tokens_used /
                                  contextPreview.token_budget) *
                                  100
                              ),
                            }
                          )}
                        </span>
                        <span>
                          {t(
                            'integrations.evaluate.contextPreview.artifacts',
                            {
                              included: contextPreview.artifacts_included,
                              total: contextPreview.artifacts_total,
                            }
                          )}
                        </span>
                      </div>
                      <pre className="text-xs whitespace-pre-wrap font-mono bg-muted/50 rounded p-3 max-h-96 overflow-auto">
                        {contextPreview.context || t('integrations.evaluate.contextPreview.empty')}
                      </pre>
                    </div>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      {t('integrations.evaluate.contextPreview.empty')}
                    </p>
                  )}
                </>
              )}
            </CardContent>
          </Card>

          {/* Event Log */}
          <Card>
            <CardHeader>
              <CardTitle>{t('integrations.evaluate.events.title')}</CardTitle>
            </CardHeader>
            <CardContent>
              {eventsLoading ? (
                <div className="flex items-center justify-center py-6">
                  <Loader2 className="h-5 w-5 animate-spin" />
                </div>
              ) : allEvents.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  {t('integrations.evaluate.events.empty')}
                </p>
              ) : (
                <div className="space-y-1 max-h-96 overflow-auto">
                  {allEvents.map((event, i) => (
                    <EventRow key={event.id || i} event={event} />
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </>
      )}

      {/* Run History - filtered by selected test case */}
      {(() => {
        const activeTestCase = TEST_CASES.find(
          (tc) => tc.id === selectedTestCase
        )!;
        const filteredRuns = runs.filter(
          (r) => r.project_name === activeTestCase.projectName
        );
        return (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <History className="h-5 w-5" />
                {activeTestCase.name} Runs
              </CardTitle>
              <CardDescription>{activeTestCase.description}</CardDescription>
            </CardHeader>
            <CardContent>
              {filteredRuns.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No previous runs. Destroy a test project to save a snapshot.
                </p>
              ) : (
                <div className="space-y-2">
                  {filteredRuns.map((run) => (
                    <RunHistoryRow
                      key={run.id}
                      run={run}
                      onDelete={handleDeleteRun}
                    />
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        );
      })()}
    </div>
  );
}
