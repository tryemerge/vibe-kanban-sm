import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { AlertTriangle, Plus, X, Rows3, LayoutList, Bot, GitFork } from 'lucide-react';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Loader } from '@/components/ui/loader';
import { tasksApi, attemptsApi, agentsApi, projectsApi, contextArtifactsApi, sessionsApi } from '@/lib/api';
import type { RepoBranchStatus, Workspace, ContextArtifact } from 'shared/types';
import { openTaskForm } from '@/lib/openTaskForm';
import { FeatureShowcaseDialog } from '@/components/dialogs/global/FeatureShowcaseDialog';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { showcases } from '@/config/showcases';
import { useUserSystem } from '@/components/ConfigProvider';
import { usePostHog } from 'posthog-js/react';

import { useSearch } from '@/contexts/SearchContext';
import { useProject } from '@/contexts/ProjectContext';
import { useTaskAttempts } from '@/hooks/useTaskAttempts';
import { useTaskAttemptWithSession } from '@/hooks/useTaskAttempt';
import { useMediaQuery } from '@/hooks/useMediaQuery';
import { useBranchStatus, useAttemptExecution, useProjectColumns, findColumnBySlug, useProjectRepos } from '@/hooks';
import { paths } from '@/lib/paths';
import { ExecutionProcessesProvider } from '@/contexts/ExecutionProcessesContext';
import { ClickedElementsProvider } from '@/contexts/ClickedElementsProvider';
import { ReviewProvider } from '@/contexts/ReviewProvider';
import {
  GitOperationsProvider,
  useGitOperationsError,
} from '@/contexts/GitOperationsContext';
import {
  useKeyCreate,
  useKeyExit,
  useKeyFocusSearch,
  useKeyNavUp,
  useKeyNavDown,
  useKeyNavLeft,
  useKeyNavRight,
  useKeyOpenDetails,
  Scope,
  useKeyDeleteTask,
  useKeyCycleViewBackward,
} from '@/keyboard';

import TaskKanbanBoard, {
  type KanbanColumnItem,
  type KanbanColumnDef,
  type KanbanColumnItems,
} from '@/components/tasks/TaskKanbanBoard';
import { TaskLabelsProvider } from '@/contexts/TaskLabelsContext';
import { TaskGroupsProvider } from '@/contexts/TaskGroupsContext';
import { TaskGroupBoard } from '@/components/tasks/TaskGroupBoard';
import { TaskGroupDagView } from '@/components/tasks/TaskGroupDagView';
import { SplitScreenLayout } from '@/components/layouts/SplitScreenLayout';
import { PlansSidebar } from '@/components/tasks/PlansSidebar';
import { useTaskGroups, useTaskGroupMutations, useTaskGroupDependencies } from '@/hooks/useTaskGroups';
import { useSwimLaneConfig } from '@/hooks/useSwimLaneConfig';
import type { DragEndEvent } from '@/components/ui/shadcn-io/kanban';
import {
  useProjectTasks,
  type SharedTaskRecord,
} from '@/hooks/useProjectTasks';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { useHotkeysContext } from 'react-hotkeys-hook';
import { TasksLayout, type LayoutMode } from '@/components/layout/TasksLayout';
import { PreviewPanel } from '@/components/panels/PreviewPanel';
import { DiffsPanel } from '@/components/panels/DiffsPanel';
import TaskAttemptPanel from '@/components/panels/TaskAttemptPanel';
import TaskPanel from '@/components/panels/TaskPanel';
import SharedTaskPanel from '@/components/panels/SharedTaskPanel';
import TodoPanel from '@/components/tasks/TodoPanel';
import { ProjectAgentPanel } from '@/components/tasks/ProjectAgentPanel';
import { useTaskCompletionToasts } from '@/hooks/useTaskCompletionToasts';
import { useAuth } from '@/hooks';
import { NewCard, NewCardHeader } from '@/components/ui/new-card';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbLink,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { AttemptHeaderActions } from '@/components/panels/AttemptHeaderActions';
import { TaskPanelHeaderActions } from '@/components/panels/TaskPanelHeaderActions';

import type { TaskWithAttemptStatus, TaskStatus, BaseCodingAgent } from 'shared/types';

type Task = TaskWithAttemptStatus;

const TASK_STATUSES = [
  'todo',
  'inprogress',
  'inreview',
  'done',
  'cancelled',
] as const;

// Default column definitions when no board is assigned to the project
const DEFAULT_COLUMN_DEFS: KanbanColumnDef[] = [
  { id: 'default-todo', name: 'To Do', slug: 'todo', color: null },
  { id: 'default-inprogress', name: 'In Progress', slug: 'inprogress', color: null },
  { id: 'default-inreview', name: 'In Review', slug: 'inreview', color: null },
  { id: 'default-done', name: 'Done', slug: 'done', color: null },
  { id: 'default-cancelled', name: 'Cancelled', slug: 'cancelled', color: null },
];

const normalizeStatus = (status: string): TaskStatus =>
  status.toLowerCase() as TaskStatus;

function GitErrorBanner() {
  const { error: gitError } = useGitOperationsError();

  if (!gitError) return null;

  return (
    <div className="mx-4 mt-4 p-3 border border-destructive rounded">
      <div className="text-destructive text-sm">{gitError}</div>
    </div>
  );
}

function DiffsPanelContainer({
  attempt,
  selectedTask,
  branchStatus,
}: {
  attempt: Workspace | null;
  selectedTask: TaskWithAttemptStatus | null;
  branchStatus: RepoBranchStatus[] | null;
}) {
  const { isAttemptRunning } = useAttemptExecution(attempt?.id);

  return (
    <DiffsPanel
      key={attempt?.id}
      selectedAttempt={attempt}
      gitOps={
        attempt && selectedTask
          ? {
              task: selectedTask,
              branchStatus: branchStatus ?? null,
              isAttemptRunning,
              selectedBranch: branchStatus?.[0]?.target_branch_name ?? null,
            }
          : undefined
      }
    />
  );
}

export function ProjectTasks() {
  const { t } = useTranslation(['tasks', 'common']);
  const { taskId, attemptId } = useParams<{
    projectId: string;
    taskId?: string;
    attemptId?: string;
  }>();
  const navigate = useNavigate();
  const { enableScope, disableScope, activeScopes } = useHotkeysContext();
  const [searchParams, setSearchParams] = useSearchParams();
  const isXL = useMediaQuery('(min-width: 1280px)');
  const isMobile = !isXL;
  const posthog = usePostHog();
  const [selectedSharedTaskId, setSelectedSharedTaskId] = useState<
    string | null
  >(null);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);

  const { userId } = useAuth();

  const {
    projectId,
    project,
    isLoading: projectLoading,
    error: projectError,
  } = useProject();

  useEffect(() => {
    enableScope(Scope.KANBAN);

    return () => {
      disableScope(Scope.KANBAN);
    };
  }, [enableScope, disableScope]);

  const handleCreateTask = useCallback(() => {
    if (projectId) {
      openTaskForm({ mode: 'create', projectId });
    }
  }, [projectId]);
  const { query: searchQuery, focusInput } = useSearch();

  const {
    tasks,
    tasksById,
    sharedTasksById,
    sharedOnlyByStatus,
    isLoading,
    error: streamError,
  } = useProjectTasks(projectId || '');

  // Show in-app toasts when tasks complete or fail
  useTaskCompletionToasts(tasks);

  // Fetch task groups for split-screen display
  const { data: taskGroups = [] } = useTaskGroups(projectId);
  const { data: groupDependencies = [] } = useTaskGroupDependencies(projectId);
  const { transitionGroup, createGroup } = useTaskGroupMutations(projectId || '');

  // Fetch columns for the project to check for agent assignments
  const { data: projectColumns } = useProjectColumns(projectId);

  // Fetch repos for the project (needed for creating task attempts)
  const { data: projectRepos = [] } = useProjectRepos(projectId);

  // Swim lane configuration (persisted in localStorage)
  const {
    config: swimLaneConfig,
    toggleLaneCollapse,
    setGroupBy,
    isEnabled: swimLanesEnabled,
  } = useSwimLaneConfig(projectId);

  // Create column definitions from project columns or use defaults
  const columnDefs = useMemo<KanbanColumnDef[]>(() => {
    if (projectColumns && projectColumns.length > 0) {
      return projectColumns.map((col) => ({
        id: col.id,
        name: col.name,
        slug: col.slug,
        color: col.color,
      }));
    }
    return DEFAULT_COLUMN_DEFS;
  }, [projectColumns]);

  // Build a map from column ID to column definition for quick lookups
  const columnById = useMemo(() => {
    const map = new Map<string, KanbanColumnDef>();
    for (const col of columnDefs) {
      map.set(col.id, col);
    }
    return map;
  }, [columnDefs]);

  // Build a map from slug to column ID for fallback matching
  const columnIdBySlug = useMemo(() => {
    const map = new Map<string, string>();
    for (const col of columnDefs) {
      map.set(col.slug, col.id);
    }
    return map;
  }, [columnDefs]);

  const selectedTask = useMemo(
    () => (taskId ? (tasksById[taskId] ?? null) : null),
    [taskId, tasksById]
  );

  const selectedSharedTask = useMemo(() => {
    if (!selectedSharedTaskId) return null;
    return sharedTasksById[selectedSharedTaskId] ?? null;
  }, [selectedSharedTaskId, sharedTasksById]);

  useEffect(() => {
    if (taskId) {
      setSelectedSharedTaskId(null);
    }
  }, [taskId]);

  // Column-level agent panel — one persistent workspace per agent type, stored on project
  const [viewingColumnAgent, setViewingColumnAgent] = useState<'backlog' | 'analyzing' | 'prereq_eval' | null>(null);
  const [columnAgentWorkspaceId, setColumnAgentWorkspaceId] = useState<string | null>(null);
  // Workspace IDs come from the project directly (set by backend when agent first runs)
  const grouperWorkspaceId = project?.grouper_workspace_id ?? null;
  const groupEvaluatorWorkspaceId = project?.group_evaluator_workspace_id ?? null;
  const prereqEvalWorkspaceId = project?.prereq_eval_workspace_id ?? null;

  const handleViewColumnAgent = useCallback(async (slug: 'backlog' | 'analyzing' | 'prereq_eval') => {
    if (!projectId) return;
    // Toggle off if already viewing this agent
    if (viewingColumnAgent === slug) {
      setViewingColumnAgent(null);
      setColumnAgentWorkspaceId(null);
      return;
    }
    // Navigate away from selected task
    navigate(paths.projectTasks(projectId));

    let workspaceId: string | null =
      slug === 'backlog' ? grouperWorkspaceId :
      slug === 'analyzing' ? groupEvaluatorWorkspaceId :
      prereqEvalWorkspaceId;

    if (!workspaceId) {
      // Create workspace on demand (idempotent on server)
      try {
        const starter =
          slug === 'backlog' ? projectsApi.startGrouperAgent :
          slug === 'analyzing' ? projectsApi.startGroupEvaluatorAgent :
          projectsApi.startPrereqEvalAgent;
        const result = await starter(projectId);
        workspaceId = result.workspace_id;
      } catch (err) {
        console.error('Failed to start column agent:', err);
        return;
      }
    }

    setViewingColumnAgent(slug);
    setColumnAgentWorkspaceId(workspaceId);
  }, [projectId, viewingColumnAgent, grouperWorkspaceId, groupEvaluatorWorkspaceId, prereqEvalWorkspaceId, navigate]);

  // DAG viewer state
  const [viewingDag, setViewingDag] = useState(false);

  // Project agent panel state
  const [viewingProjectAgent, setViewingProjectAgent] = useState(false);
  const [projectAgentWorkspaceId, setProjectAgentWorkspaceId] = useState<string | null>(
    project?.agent_workspace_id ?? null
  );
  const [isStartingAgent, setIsStartingAgent] = useState(false);
  const [pendingBriefMessage, setPendingBriefMessage] = useState<string | null>(null);

  // Artifact viewing state (IMPL doc modal)
  const [viewingArtifact, setViewingArtifact] = useState<ContextArtifact | null>(null);

  const handleViewArtifact = useCallback(async (artifactId: string) => {
    try {
      const artifact = await contextArtifactsApi.get(artifactId);
      setViewingArtifact(artifact);
    } catch (err) {
      console.error('Failed to fetch artifact', err);
    }
  }, []);

  const handleCreateGroupFromPlan = useCallback((plan: ContextArtifact) => {
    createGroup.mutate({
      name: plan.title,
      color: null,
      is_backlog: null,
      artifact_id: plan.id,
    });
  }, [createGroup]);

  // Sync project.agent_workspace_id into state when project loads
  useEffect(() => {
    if (project?.agent_workspace_id && !projectAgentWorkspaceId) {
      setProjectAgentWorkspaceId(project.agent_workspace_id);
    }
  }, [project?.agent_workspace_id, projectAgentWorkspaceId]);

  const handleToggleProjectAgent = useCallback(async () => {
    if (viewingProjectAgent) {
      setViewingProjectAgent(false);
      return;
    }
    if (projectAgentWorkspaceId) {
      setViewingProjectAgent(true);
      return;
    }
    if (!projectId) return;
    setIsStartingAgent(true);
    try {
      const result = await projectsApi.startAgent(projectId);
      setProjectAgentWorkspaceId(result.workspace_id);
      setViewingProjectAgent(true);
    } catch (err) {
      console.error('Failed to start project agent:', err);
    } finally {
      setIsStartingAgent(false);
    }
  }, [viewingProjectAgent, projectAgentWorkspaceId, projectId]);

  const { data: groupAttempt } = useTaskAttemptWithSession(columnAgentWorkspaceId ?? undefined);
  const { data: projectAgentAttempt } = useTaskAttemptWithSession(
    viewingProjectAgent ? (projectAgentWorkspaceId ?? undefined) : undefined
  );

  // When a pending brief message is set and the project agent session becomes available, send it
  useEffect(() => {
    const sessionId = projectAgentAttempt?.session?.id;
    if (!pendingBriefMessage || !sessionId) return;
    const message = pendingBriefMessage;
    setPendingBriefMessage(null);
    sessionsApi.followUp(sessionId, { prompt: message, variant: null, retry_process_id: null, force_when_dirty: null, perform_git_reset: null }).catch((err) => {
      console.error('Failed to send brief to project agent:', err);
    });
  }, [pendingBriefMessage, projectAgentAttempt?.session?.id]);

  const handleApproveBrief = useCallback((brief: ContextArtifact) => {
    const message = `Please convert this brief into an ADR and implementation plan. Use chain_id = '${brief.id}' for BOTH the ADR and the iplan so they appear linked to this brief in the Plans panel.\n\nBrief title: ${brief.title}\n\n${brief.content}`;
    const sessionId = projectAgentAttempt?.session?.id;
    if (sessionId) {
      sessionsApi.followUp(sessionId, { prompt: message, variant: null, retry_process_id: null, force_when_dirty: null, perform_git_reset: null }).catch((err) => {
        console.error('Failed to send brief to project agent:', err);
      });
      if (!viewingProjectAgent) setViewingProjectAgent(true);
    } else {
      setPendingBriefMessage(message);
      if (!viewingProjectAgent) handleToggleProjectAgent();
    }
  }, [viewingProjectAgent, projectAgentAttempt?.session?.id, handleToggleProjectAgent]);

  // Close agent panels when a task is selected
  useEffect(() => {
    if (taskId) {
      setViewingColumnAgent(null);
      setColumnAgentWorkspaceId(null);
      setViewingProjectAgent(false);
    }
  }, [taskId]);

  const isTaskPanelOpen = Boolean(taskId && selectedTask);
  const isSharedPanelOpen = Boolean(selectedSharedTask);
  const isGroupPanelOpen = Boolean(columnAgentWorkspaceId && groupAttempt);
  const isProjectAgentPanelOpen = Boolean(viewingProjectAgent && projectAgentAttempt);
  const isPanelOpen = isTaskPanelOpen || isSharedPanelOpen || isGroupPanelOpen || isProjectAgentPanelOpen;

  const { config, updateAndSaveConfig, loading } = useUserSystem();

  const isLoaded = !loading;
  const showcaseId = showcases.taskPanel.id;
  const seenFeatures = useMemo(
    () => config?.showcases?.seen_features ?? [],
    [config?.showcases?.seen_features]
  );
  const seen = isLoaded && seenFeatures.includes(showcaseId);

  useEffect(() => {
    if (!isLoaded || !isPanelOpen || seen) return;

    FeatureShowcaseDialog.show({ config: showcases.taskPanel }).finally(() => {
      FeatureShowcaseDialog.hide();
      if (seenFeatures.includes(showcaseId)) return;
      void updateAndSaveConfig({
        showcases: { seen_features: [...seenFeatures, showcaseId] },
      });
    });
  }, [
    isLoaded,
    isPanelOpen,
    seen,
    showcaseId,
    updateAndSaveConfig,
    seenFeatures,
  ]);

  const isLatest = attemptId === 'latest';
  const { data: attempts = [], isLoading: isAttemptsLoading } = useTaskAttempts(
    taskId,
    {
      enabled: !!taskId && isLatest,
    }
  );

  const latestAttemptId = useMemo(() => {
    if (!attempts?.length) return undefined;
    return [...attempts].sort((a, b) => {
      const diff =
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
      if (diff !== 0) return diff;
      return a.id.localeCompare(b.id);
    })[0].id;
  }, [attempts]);

  const navigateWithSearch = useCallback(
    (pathname: string, options?: { replace?: boolean }) => {
      const search = searchParams.toString();
      navigate({ pathname, search: search ? `?${search}` : '' }, options);
    },
    [navigate, searchParams]
  );

  useEffect(() => {
    if (!projectId || !taskId) return;
    if (!isLatest) return;
    if (isAttemptsLoading) return;

    if (!latestAttemptId) {
      navigateWithSearch(paths.task(projectId, taskId), { replace: true });
      return;
    }

    navigateWithSearch(paths.attempt(projectId, taskId, latestAttemptId), {
      replace: true,
    });
  }, [
    projectId,
    taskId,
    isLatest,
    isAttemptsLoading,
    latestAttemptId,
    navigate,
    navigateWithSearch,
  ]);

  useEffect(() => {
    if (!projectId || !taskId || isLoading) return;
    if (selectedTask === null) {
      navigate(`/projects/${projectId}/tasks`, { replace: true });
    }
  }, [projectId, taskId, isLoading, selectedTask, navigate]);

  const effectiveAttemptId = attemptId === 'latest' ? undefined : attemptId;
  const isTaskView = !!taskId && !effectiveAttemptId;
  const { data: attempt } = useTaskAttemptWithSession(effectiveAttemptId);

  const { data: branchStatus } = useBranchStatus(attempt?.id);

  const rawMode = searchParams.get('view') as LayoutMode;
  const mode: LayoutMode =
    rawMode === 'preview' || rawMode === 'diffs' ? rawMode : null;

  // TODO: Remove this redirect after v0.1.0 (legacy URL support for bookmarked links)
  // Migrates old `view=logs` to `view=diffs`
  useEffect(() => {
    const view = searchParams.get('view');
    if (view === 'logs') {
      const params = new URLSearchParams(searchParams);
      params.set('view', 'diffs');
      setSearchParams(params, { replace: true });
    }
  }, [searchParams, setSearchParams]);

  const setMode = useCallback(
    (newMode: LayoutMode) => {
      const params = new URLSearchParams(searchParams);
      if (newMode === null) {
        params.delete('view');
      } else {
        params.set('view', newMode);
      }
      setSearchParams(params, { replace: true });
    },
    [searchParams, setSearchParams]
  );

  const handleCreateNewTask = useCallback(() => {
    handleCreateTask();
  }, [handleCreateTask]);

  useKeyCreate(handleCreateNewTask, {
    scope: Scope.KANBAN,
    preventDefault: true,
  });

  useKeyFocusSearch(
    () => {
      focusInput();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  useKeyExit(
    () => {
      if (isPanelOpen) {
        handleClosePanel();
      } else {
        navigate('/projects');
      }
    },
    { scope: Scope.KANBAN }
  );

  const hasSearch = Boolean(searchQuery.trim());
  const normalizedSearch = searchQuery.trim().toLowerCase();
  const showSharedTasks = searchParams.get('shared') !== 'off';

  useEffect(() => {
    if (showSharedTasks) return;
    if (!selectedSharedTaskId) return;
    const sharedTask = sharedTasksById[selectedSharedTaskId];
    if (sharedTask && sharedTask.assignee_user_id === userId) {
      return;
    }
    setSelectedSharedTaskId(null);
  }, [selectedSharedTaskId, sharedTasksById, showSharedTasks, userId]);

  // Find the initial/backlog column for fallback
  const initialColumnId = useMemo(() => {
    return projectColumns?.find((col) => col.is_initial)?.id ?? null;
  }, [projectColumns]);

  // Helper to determine which column a task belongs to
  const getTaskColumnId = useCallback((task: Task): string | null => {
    // If task has a column_id, use it if it exists in our columns
    if (task.column_id && columnById.has(task.column_id)) {
      return task.column_id;
    }
    // Fall back to matching by status slug
    const statusSlug = normalizeStatus(task.status);
    const bySlug = columnIdBySlug.get(statusSlug);
    if (bySlug) {
      return bySlug;
    }
    // Final fallback: use the initial/backlog column
    return initialColumnId;
  }, [columnById, columnIdBySlug, initialColumnId]);

  const kanbanColumnItems = useMemo<KanbanColumnItems>(() => {
    // Initialize columns from column definitions
    const columns: KanbanColumnItems = {};
    for (const col of columnDefs) {
      columns[col.id] = [];
    }

    // Build task group lookup for checking state
    const taskGroupById = new Map(taskGroups.map(g => [g.id, g]));

    const matchesSearch = (
      title: string,
      description?: string | null
    ): boolean => {
      if (!hasSearch) return true;
      const lowerTitle = title.toLowerCase();
      const lowerDescription = description?.toLowerCase() ?? '';
      return (
        lowerTitle.includes(normalizedSearch) ||
        lowerDescription.includes(normalizedSearch)
      );
    };

    tasks.forEach((task) => {
      // Skip system/meta tasks created by the Task Grouper agent
      if (/^Group \d+ ungrouped tasks$/i.test(task.title)) {
        return;
      }

      // Skip ungrouped tasks - they appear in the Task Group board at the top
      if (!task.task_group_id) {
        return;
      }

      // Only show tasks in the kanban once their group is executing or done
      // Before that, tasks are managed at the group level (not yet in the kanban backlog)
      const taskGroup = taskGroupById.get(task.task_group_id);
      if (taskGroup && taskGroup.state !== 'executing' && taskGroup.state !== 'done') {
        return;
      }

      const columnId = getTaskColumnId(task);
      if (!columnId || !columns[columnId]) {
        // Task doesn't belong to any known column, skip it
        return;
      }

      const sharedTask = task.shared_task_id
        ? sharedTasksById[task.shared_task_id]
        : sharedTasksById[task.id];

      if (!matchesSearch(task.title, task.description)) {
        return;
      }

      const isSharedAssignedElsewhere =
        !showSharedTasks &&
        !!sharedTask &&
        !!sharedTask.assignee_user_id &&
        sharedTask.assignee_user_id !== userId;

      if (isSharedAssignedElsewhere) {
        return;
      }

      // Apply group color to tasks in executing groups for visual correlation
      const backgroundColor =
        taskGroup && taskGroup.state === 'executing' ? taskGroup.color ?? undefined : undefined;

      columns[columnId].push({
        type: 'task',
        task,
        sharedTask,
        backgroundColor,
      });
    });

    // Add shared-only tasks by matching their status to column slugs
    (
      Object.entries(sharedOnlyByStatus) as [TaskStatus, SharedTaskRecord[]][]
    ).forEach(([status, items]) => {
      const columnId = columnIdBySlug.get(status);
      if (!columnId || !columns[columnId]) {
        return;
      }
      items.forEach((sharedTask) => {
        if (!matchesSearch(sharedTask.title, sharedTask.description)) {
          return;
        }
        const shouldIncludeShared =
          showSharedTasks || sharedTask.assignee_user_id === userId;
        if (!shouldIncludeShared) {
          return;
        }
        columns[columnId].push({
          type: 'shared',
          task: sharedTask,
        });
      });
    });

    const getTimestamp = (item: KanbanColumnItem) => {
      const createdAt =
        item.type === 'task' ? item.task.created_at : item.task.created_at;
      return new Date(createdAt).getTime();
    };

    // Sort items within each column by timestamp (newest first)
    for (const columnId of Object.keys(columns)) {
      columns[columnId].sort((a, b) => getTimestamp(b) - getTimestamp(a));
    }

    return columns;
  }, [
    columnDefs,
    columnIdBySlug,
    hasSearch,
    normalizedSearch,
    tasks,
    taskGroups,
    sharedOnlyByStatus,
    sharedTasksById,
    showSharedTasks,
    userId,
    getTaskColumnId,
  ]);

  // Build a map from column slug to visible tasks for keyboard navigation
  const visibleTasksBySlug = useMemo(() => {
    const map: Record<string, Task[]> = {};

    for (const col of columnDefs) {
      const items = kanbanColumnItems[col.id] ?? [];
      map[col.slug] = items
        .filter((item) => item.type === 'task')
        .map((item) => item.task);
    }

    return map;
  }, [columnDefs, kanbanColumnItems]);

  // Legacy visibleTasksByStatus for keyboard nav (uses slug keys that match TaskStatus)
  const visibleTasksByStatus = useMemo(() => {
    const map: Record<TaskStatus, Task[]> = {
      todo: [],
      inprogress: [],
      inreview: [],
      done: [],
      cancelled: [],
    };

    // Map from slug-keyed results to status-keyed results
    for (const status of TASK_STATUSES) {
      map[status] = visibleTasksBySlug[status] ?? [];
    }

    return map;
  }, [visibleTasksBySlug]);

  const hasVisibleLocalTasks = useMemo(
    () =>
      Object.values(kanbanColumnItems).some(
        (items) => items && items.some((item) => item.type === 'task')
      ),
    [kanbanColumnItems]
  );

  const hasVisibleSharedTasks = useMemo(
    () =>
      Object.values(kanbanColumnItems).some((items) =>
        items.some((item) => item.type === 'shared')
      ),
    [kanbanColumnItems]
  );

  useKeyNavUp(
    () => {
      selectPreviousTask();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  useKeyNavDown(
    () => {
      selectNextTask();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  useKeyNavLeft(
    () => {
      selectPreviousColumn();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  useKeyNavRight(
    () => {
      selectNextColumn();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  /**
   * Cycle the attempt area view.
   * - When panel is closed: opens task details (if a task is selected)
   * - When panel is open: cycles among [attempt, preview, diffs]
   */
  const cycleView = useCallback(
    (direction: 'forward' | 'backward' = 'forward') => {
      const order: LayoutMode[] = [null, 'preview', 'diffs'];
      const idx = order.indexOf(mode);
      const next =
        direction === 'forward'
          ? order[(idx + 1) % order.length]
          : order[(idx - 1 + order.length) % order.length];
      setMode(next);
    },
    [mode, setMode]
  );

  const cycleViewForward = useCallback(() => cycleView('forward'), [cycleView]);
  const cycleViewBackward = useCallback(
    () => cycleView('backward'),
    [cycleView]
  );

  // meta/ctrl+enter → open details or cycle forward
  const isFollowUpReadyActive = activeScopes.includes(Scope.FOLLOW_UP_READY);

  useKeyOpenDetails(
    () => {
      if (isPanelOpen) {
        // Track keyboard shortcut before cycling view
        const order: LayoutMode[] = [null, 'preview', 'diffs'];
        const idx = order.indexOf(mode);
        const next = order[(idx + 1) % order.length];

        if (next === 'preview') {
          posthog?.capture('preview_navigated', {
            trigger: 'keyboard',
            direction: 'forward',
            timestamp: new Date().toISOString(),
            source: 'frontend',
          });
        } else if (next === 'diffs') {
          posthog?.capture('diffs_navigated', {
            trigger: 'keyboard',
            direction: 'forward',
            timestamp: new Date().toISOString(),
            source: 'frontend',
          });
        }

        cycleViewForward();
      } else if (selectedTask) {
        handleViewTaskDetails(selectedTask);
      }
    },
    { scope: Scope.KANBAN, when: () => !isFollowUpReadyActive }
  );

  // meta/ctrl+shift+enter → cycle backward
  useKeyCycleViewBackward(
    () => {
      if (isPanelOpen) {
        // Track keyboard shortcut before cycling view
        const order: LayoutMode[] = [null, 'preview', 'diffs'];
        const idx = order.indexOf(mode);
        const next = order[(idx - 1 + order.length) % order.length];

        if (next === 'preview') {
          posthog?.capture('preview_navigated', {
            trigger: 'keyboard',
            direction: 'backward',
            timestamp: new Date().toISOString(),
            source: 'frontend',
          });
        } else if (next === 'diffs') {
          posthog?.capture('diffs_navigated', {
            trigger: 'keyboard',
            direction: 'backward',
            timestamp: new Date().toISOString(),
            source: 'frontend',
          });
        }

        cycleViewBackward();
      }
    },
    { scope: Scope.KANBAN, preventDefault: true }
  );

  useKeyDeleteTask(
    () => {
      // Note: Delete is now handled by TaskActionsDropdown
      // This keyboard shortcut could trigger the dropdown action if needed
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  const handleClosePanel = useCallback(() => {
    if (projectId) {
      navigate(`/projects/${projectId}/tasks`, { replace: true });
    }
  }, [projectId, navigate]);


  const handleViewTaskDetails = useCallback(
    (task: Task, attemptIdToShow?: string) => {
      if (!projectId) return;
      setSelectedSharedTaskId(null);

      // Check if task is in a backlog (is_initial) or terminal (done/cancelled) column
      // These columns should show the task panel with attempts list
      const taskColumn = projectColumns?.find((col) => col.id === task.column_id);
      const isInBacklog = taskColumn?.is_initial ?? false;
      const isInTerminal = taskColumn?.is_terminal ?? false;
      const showSummaryView = isInBacklog || isInTerminal;

      if (showSummaryView) {
        // Show task panel with summary and attempts list
        navigateWithSearch(paths.task(projectId, task.id));
      } else if (attemptIdToShow) {
        navigateWithSearch(paths.attempt(projectId, task.id, attemptIdToShow));
      } else if (task.latest_attempt_id) {
        // Navigate directly to the latest attempt (active execution view)
        navigateWithSearch(paths.attempt(projectId, task.id, task.latest_attempt_id));
      } else {
        // No attempts yet, show task details
        navigateWithSearch(paths.task(projectId, task.id));
      }
    },
    [projectId, navigateWithSearch, projectColumns]
  );

  const handleViewSharedTask = useCallback(
    (sharedTask: SharedTaskRecord) => {
      setSelectedSharedTaskId(sharedTask.id);
      setMode(null);
      if (projectId) {
        navigateWithSearch(paths.projectTasks(projectId), { replace: true });
      }
    },
    [navigateWithSearch, projectId, setMode]
  );

  const selectNextTask = useCallback(() => {
    if (selectedTask) {
      const statusKey = normalizeStatus(selectedTask.status);
      const tasksInStatus = visibleTasksByStatus[statusKey] || [];
      const currentIndex = tasksInStatus.findIndex(
        (task) => task.id === selectedTask.id
      );
      if (currentIndex >= 0 && currentIndex < tasksInStatus.length - 1) {
        handleViewTaskDetails(tasksInStatus[currentIndex + 1]);
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = visibleTasksByStatus[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, visibleTasksByStatus, handleViewTaskDetails]);

  const selectPreviousTask = useCallback(() => {
    if (selectedTask) {
      const statusKey = normalizeStatus(selectedTask.status);
      const tasksInStatus = visibleTasksByStatus[statusKey] || [];
      const currentIndex = tasksInStatus.findIndex(
        (task) => task.id === selectedTask.id
      );
      if (currentIndex > 0) {
        handleViewTaskDetails(tasksInStatus[currentIndex - 1]);
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = visibleTasksByStatus[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, visibleTasksByStatus, handleViewTaskDetails]);

  const selectNextColumn = useCallback(() => {
    if (selectedTask) {
      const currentStatus = normalizeStatus(selectedTask.status);
      const currentIndex = TASK_STATUSES.findIndex(
        (status) => status === currentStatus
      );
      for (let i = currentIndex + 1; i < TASK_STATUSES.length; i++) {
        const tasks = visibleTasksByStatus[TASK_STATUSES[i]];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          return;
        }
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = visibleTasksByStatus[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, visibleTasksByStatus, handleViewTaskDetails]);

  const selectPreviousColumn = useCallback(() => {
    if (selectedTask) {
      const currentStatus = normalizeStatus(selectedTask.status);
      const currentIndex = TASK_STATUSES.findIndex(
        (status) => status === currentStatus
      );
      for (let i = currentIndex - 1; i >= 0; i--) {
        const tasks = visibleTasksByStatus[TASK_STATUSES[i]];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          return;
        }
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = visibleTasksByStatus[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, visibleTasksByStatus, handleViewTaskDetails]);

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || !active.data.current) return;

      const draggedTaskId = active.id as string;
      const newStatus = over.id as Task['status'];
      const task = tasksById[draggedTaskId];
      if (!task || task.status === newStatus) return;

      // Find the column for the target status (by slug)
      const targetColumn = findColumnBySlug(projectColumns, newStatus);
      const newColumnId = targetColumn?.id ?? null;

      try {
        // Update task status and column_id
        await tasksApi.update(draggedTaskId, {
          title: task.title,
          description: task.description,
          status: newStatus,
          column_id: newColumnId,
          parent_workspace_id: task.parent_workspace_id,
          image_ids: null,
          task_group_id: null,
        });

        // If the target column has an agent assigned, trigger execution
        if (targetColumn?.agent_id && projectRepos.length > 0) {
          try {
            // Fetch agent details to get the executor type
            const agent = await agentsApi.getById(targetColumn.agent_id);

            // Create repos input for the task attempt
            const repos = projectRepos.map((repo) => ({
              repo_id: repo.id,
              target_branch: 'main', // TODO: Get default branch from repo
            }));

            // Create and start a task attempt with the column's agent
            await attemptsApi.create({
              task_id: draggedTaskId,
              executor_profile_id: {
                executor: agent.executor as BaseCodingAgent,
                variant: null,
              },
              repos,
            });

            console.log(`Started agent execution for task ${draggedTaskId} with agent ${agent.name}`);
          } catch (agentErr) {
            console.error('Failed to trigger agent execution:', agentErr);
          }
        }
      } catch (err) {
        console.error('Failed to update task status:', err);
      }
    },
    [tasksById, projectColumns, projectRepos]
  );

  const getSharedTask = useCallback(
    (task: Task | null | undefined) => {
      if (!task) return undefined;
      if (task.shared_task_id) {
        return sharedTasksById[task.shared_task_id];
      }
      return sharedTasksById[task.id];
    },
    [sharedTasksById]
  );

  const hasSharedTasks = useMemo(() => {
    return Object.values(kanbanColumnItems).some((items) =>
      items.some((item) => {
        if (item.type === 'shared') return true;
        return Boolean(item.sharedTask);
      })
    );
  }, [kanbanColumnItems]);

  const isInitialTasksLoad = isLoading && tasks.length === 0;

  if (projectError) {
    return (
      <div className="p-4">
        <Alert>
          <AlertTitle className="flex items-center gap-2">
            <AlertTriangle size="16" />
            {t('common:states.error')}
          </AlertTitle>
          <AlertDescription>
            {projectError.message || 'Failed to load project'}
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  if (projectLoading && isInitialTasksLoad) {
    return <Loader message={t('loading')} size={32} className="py-8" />;
  }

  const truncateTitle = (title: string | undefined, maxLength = 20) => {
    if (!title) return 'Task';
    if (title.length <= maxLength) return title;

    const truncated = title.substring(0, maxLength);
    const lastSpace = truncated.lastIndexOf(' ');

    return lastSpace > 0
      ? `${truncated.substring(0, lastSpace)}...`
      : `${truncated}...`;
  };

  // Check if there are tasks in the Task Group board (ungrouped or in draft/pending groups)
  const hasTasksInGroupBoard = tasks.length > 0 || taskGroups.length > 0;

  const dagContent = (
    <TaskGroupDagView groups={taskGroups} dependencies={groupDependencies} />
  );

  const kanbanContent =
    viewingDag ? dagContent :
    tasks.length === 0 && !hasSharedTasks ? (
      <div className="max-w-7xl mx-auto mt-8">
        <Card>
          <CardContent className="text-center py-8">
            <p className="text-muted-foreground">{t('empty.noTasks')}</p>
            <Button className="mt-4" onClick={handleCreateNewTask}>
              <Plus className="h-4 w-4 mr-2" />
              {t('empty.createFirst')}
            </Button>
          </CardContent>
        </Card>
      </div>
    ) : !hasVisibleLocalTasks && !hasVisibleSharedTasks && !hasTasksInGroupBoard ? (
      <div className="max-w-7xl mx-auto mt-8">
        <Card>
          <CardContent className="text-center py-8">
            <p className="text-muted-foreground">
              {t('empty.noSearchResults')}
            </p>
          </CardContent>
        </Card>
      </div>
    ) : (
      <TaskGroupsProvider projectId={projectId}>
      <TaskLabelsProvider projectId={projectId}>
        <div className="flex h-full">
          {/* Left Sidebar - Plans + Ungrouped Tasks */}
          <div className="w-64 flex-shrink-0">
            <PlansSidebar
              projectId={projectId || ''}
              taskGroups={taskGroups}
              onViewArtifact={handleViewArtifact}
              onCreateGroupFromPlan={handleCreateGroupFromPlan}
              onApproveBrief={handleApproveBrief}
            />
          </div>

          {/* Right Side - Split Screen */}
          <div className="flex-1 flex flex-col min-w-0">
            <SplitScreenLayout
              topPanel={
                <TaskGroupBoard
                  groups={taskGroups}
                  selectedGroupId={selectedGroupId}
                  onSelectGroup={setSelectedGroupId}
                  projectId={projectId || ''}
                  tasks={tasks.filter(t => !/^Group \d+ ungrouped tasks$/i.test(t.title)).map(t => ({ id: t.id, title: t.title, task_group_id: t.task_group_id, status: t.status }))}
                  onTransitionGroup={(groupId, from, to) => transitionGroup.mutate({ groupId, from, to })}
                  onViewColumnAgent={(slug) => handleViewColumnAgent(slug)}
                  activeColumnAgent={viewingColumnAgent}
                  onViewGrouperAgent={() => handleViewColumnAgent('backlog')}
                  onViewArtifact={handleViewArtifact}
                />
              }
              bottomPanel={
            <div className="w-full h-full flex flex-col min-h-0">
              {/* Kanban Toolbar */}
              <div className="shrink-0 flex items-center justify-end gap-2 px-4 py-2 border-b bg-background/50">
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant={swimLanesEnabled ? 'secondary' : 'ghost'}
                        size="sm"
                        onClick={() => {
                          if (swimLanesEnabled) {
                            setGroupBy({ type: 'none' });
                          } else {
                            setGroupBy({ type: 'label' });
                          }
                        }}
                        className="gap-2"
                      >
                        {swimLanesEnabled ? (
                          <Rows3 className="h-4 w-4" />
                        ) : (
                          <LayoutList className="h-4 w-4" />
                        )}
                        <span className="hidden sm:inline">
                          {swimLanesEnabled ? 'Swim Lanes' : 'Flat View'}
                        </span>
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>
                      {swimLanesEnabled
                        ? 'Switch to flat view'
                        : 'Group tasks by label (swim lanes)'}
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>

              {/* Kanban Board */}
              <div className="flex-1 min-h-0 overflow-x-auto overflow-y-auto overscroll-x-contain">
                <TaskKanbanBoard
                  columnDefs={columnDefs}
                  columnItems={kanbanColumnItems}
                  onDragEnd={handleDragEnd}
                  onViewTaskDetails={handleViewTaskDetails}
                  onViewSharedTask={handleViewSharedTask}
                  selectedTaskId={selectedTask?.id}
                  selectedSharedTaskId={selectedSharedTaskId}
                  onCreateTask={handleCreateNewTask}
                  projectId={projectId!}
                  swimLaneConfig={swimLaneConfig}
                  onToggleLaneCollapse={toggleLaneCollapse}
                />
              </div>
            </div>
          }
        />
          </div>
        </div>
      </TaskLabelsProvider>
      </TaskGroupsProvider>
    );

  const rightHeader = selectedTask ? (
    <NewCardHeader
      className="shrink-0"
      actions={
        isTaskView ? (
          <TaskPanelHeaderActions
            task={selectedTask}
            sharedTask={getSharedTask(selectedTask)}
            onClose={() =>
              navigate(`/projects/${projectId}/tasks`, { replace: true })
            }
          />
        ) : (
          <AttemptHeaderActions
            mode={mode}
            onModeChange={setMode}
            task={selectedTask}
            sharedTask={getSharedTask(selectedTask)}
            attempt={attempt ?? null}
            onClose={() =>
              navigate(`/projects/${projectId}/tasks`, { replace: true })
            }
          />
        )
      }
    >
      <div className="mx-auto w-full">
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              {isTaskView ? (
                <BreadcrumbPage>
                  {truncateTitle(selectedTask?.title)}
                </BreadcrumbPage>
              ) : (
                <BreadcrumbLink
                  className="cursor-pointer hover:underline"
                  onClick={() =>
                    navigateWithSearch(paths.task(projectId!, taskId!))
                  }
                >
                  {truncateTitle(selectedTask?.title)}
                </BreadcrumbLink>
              )}
            </BreadcrumbItem>
            {!isTaskView && (
              <>
                <BreadcrumbSeparator />
                <BreadcrumbItem>
                  <BreadcrumbPage>
                    {attempt?.branch || 'Task Attempt'}
                  </BreadcrumbPage>
                </BreadcrumbItem>
              </>
            )}
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </NewCardHeader>
  ) : selectedSharedTask ? (
    <NewCardHeader
      className="shrink-0"
      actions={
        <Button
          variant="icon"
          aria-label={t('common:buttons.close', { defaultValue: 'Close' })}
          onClick={() => {
            setSelectedSharedTaskId(null);
            if (projectId) {
              navigateWithSearch(paths.projectTasks(projectId), {
                replace: true,
              });
            }
          }}
        >
          <X size={16} />
        </Button>
      }
    >
      <div className="mx-auto w-full">
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbPage>
                {truncateTitle(selectedSharedTask?.title)}
              </BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </NewCardHeader>
  ) : groupAttempt ? (
    <NewCardHeader
      className="shrink-0"
      actions={
        <Button variant="icon" aria-label="Close" onClick={() => {
          setViewingColumnAgent(null);
          setColumnAgentWorkspaceId(null);
        }}>
          <X size={16} />
        </Button>
      }
    >
      <div className="mx-auto w-full">
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbPage>
                {viewingColumnAgent === 'backlog' ? 'Task Grouper' :
                  viewingColumnAgent === 'prereq_eval' ? 'PreReq Evaluator' : 'Group Evaluator'}
              </BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </NewCardHeader>
  ) : isProjectAgentPanelOpen ? (
    <NewCardHeader
      className="shrink-0"
      actions={
        <Button variant="icon" aria-label="Close" onClick={() => setViewingProjectAgent(false)}>
          <X size={16} />
        </Button>
      }
    >
      <div className="mx-auto w-full">
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbPage>Project Agent</BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </NewCardHeader>
  ) : null;

  const attemptContent = selectedTask ? (
    <NewCard className="h-full min-h-0 flex flex-col bg-diagonal-lines bg-muted border-0">
      {isTaskView ? (
        <TaskPanel task={selectedTask} />
      ) : (
        <TaskAttemptPanel attempt={attempt} task={selectedTask} projectId={projectId}>
          {({ logs, followUp }) => (
            <>
              <GitErrorBanner />
              <div className="flex-1 min-h-0 flex flex-col">
                <div className="flex-1 min-h-0 flex flex-col">{logs}</div>

                <div className="shrink-0 border-t">
                  <div className="mx-auto w-full max-w-[50rem]">
                    <TodoPanel />
                  </div>
                </div>

                <div className="min-h-0 max-h-[50%] border-t overflow-hidden bg-background">
                  <div className="mx-auto w-full max-w-[50rem] h-full min-h-0">
                    {followUp}
                  </div>
                </div>
              </div>
            </>
          )}
        </TaskAttemptPanel>
      )}
    </NewCard>
  ) : selectedSharedTask ? (
    <NewCard className="h-full min-h-0 flex flex-col bg-diagonal-lines bg-muted border-0">
      <SharedTaskPanel task={selectedSharedTask} />
    </NewCard>
  ) : groupAttempt ? (
    <NewCard className="h-full min-h-0 flex flex-col bg-diagonal-lines bg-muted border-0">
      <ProjectAgentPanel attempt={groupAttempt} />
    </NewCard>
  ) : isProjectAgentPanelOpen && projectAgentAttempt ? (
    <NewCard className="h-full min-h-0 flex flex-col bg-diagonal-lines bg-muted border-0">
      <ProjectAgentPanel attempt={projectAgentAttempt} />
    </NewCard>
  ) : null;

  const auxContent =
    selectedTask && attempt ? (
      <div className="relative h-full w-full">
        {mode === 'preview' && <PreviewPanel />}
        {mode === 'diffs' && (
          <DiffsPanelContainer
            attempt={attempt}
            selectedTask={selectedTask}
            branchStatus={branchStatus ?? null}
          />
        )}
      </div>
    ) : (
      <div className="relative h-full w-full" />
    );

  const effectiveMode: LayoutMode = selectedSharedTask ? null : mode;

  // Prioritize the workspace that's actively being viewed
  const activeAttemptId = viewingProjectAgent
    ? projectAgentAttempt?.id
    : attempt?.id ?? groupAttempt?.id ?? projectAgentAttempt?.id;

  const attemptArea = (
    <GitOperationsProvider attemptId={activeAttemptId}>
      <ClickedElementsProvider attempt={attempt ?? groupAttempt}>
        <ReviewProvider attemptId={activeAttemptId}>
          <ExecutionProcessesProvider attemptId={activeAttemptId}>
            <TasksLayout
              kanban={kanbanContent}
              attempt={attemptContent}
              aux={auxContent}
              isPanelOpen={isPanelOpen}
              mode={effectiveMode}
              isMobile={isMobile}
              rightHeader={rightHeader}
            />
          </ExecutionProcessesProvider>
        </ReviewProvider>
      </ClickedElementsProvider>
    </GitOperationsProvider>
  );

  return (
    <div className="min-h-full h-full flex flex-col">
      {streamError && (
        <Alert className="w-full z-30 xl:sticky xl:top-0">
          <AlertTitle className="flex items-center gap-2">
            <AlertTriangle size="16" />
            {t('common:states.reconnecting')}
          </AlertTitle>
          <AlertDescription>{streamError}</AlertDescription>
        </Alert>
      )}

      {/* Persistent top toolbar — always visible */}
      <div className="shrink-0 flex items-center justify-end gap-2 px-4 py-1.5 border-b bg-background/80">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant={viewingDag ? 'secondary' : 'ghost'}
                size="sm"
                onClick={() => setViewingDag((v) => !v)}
                className="gap-1.5 text-xs h-7"
              >
                <GitFork className="h-3.5 w-3.5" />
                DAG
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              {viewingDag ? 'Close DAG view' : 'View group dependency graph'}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant={viewingProjectAgent ? 'secondary' : 'ghost'}
                size="sm"
                onClick={handleToggleProjectAgent}
                disabled={isStartingAgent}
                className="gap-1.5 text-xs h-7"
              >
                <Bot className="h-3.5 w-3.5" />
                {isStartingAgent ? 'Starting...' : 'Project Agent'}
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              {viewingProjectAgent
                ? 'Close Project Agent'
                : 'Chat with the Project Agent to manage tasks'}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>

      <div className="flex-1 min-h-0">{attemptArea}</div>

      {/* Artifact viewer modal */}
      <Dialog open={viewingArtifact !== null} onOpenChange={(open) => !open && setViewingArtifact(null)}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <span className="text-xs font-mono text-muted-foreground uppercase tracking-wider">
                {viewingArtifact?.artifact_type}
              </span>
              {viewingArtifact?.title}
            </DialogTitle>
          </DialogHeader>
          {viewingArtifact?.content && (
            <div className="mt-2 text-sm whitespace-pre-wrap font-mono bg-muted/30 rounded-lg p-4 border">
              {viewingArtifact.content}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
