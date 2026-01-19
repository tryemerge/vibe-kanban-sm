import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQueryClient } from '@tanstack/react-query';
import { useProject } from '@/contexts/ProjectContext';
import { useTaskAttemptsWithSessions } from '@/hooks/useTaskAttempts';
import { useTaskAttemptWithSession } from '@/hooks/useTaskAttempt';
import { useNavigateWithSearch, useProjectColumns } from '@/hooks';
import { useTaskLabelAssignments, taskLabelsKeys } from '@/hooks/useTaskLabels';
import { paths } from '@/lib/paths';
import { tasksApi } from '@/lib/api';
import type { TaskWithAttemptStatus } from 'shared/types';
import type { WorkspaceWithSession } from '@/types/attempt';
import { NewCardContent } from '../ui/new-card';
import { Button } from '../ui/button';
import { PlusIcon, Play, XCircle, Loader2, CheckCircle2 } from 'lucide-react';
import { CreateAttemptDialog } from '@/components/dialogs/tasks/CreateAttemptDialog';
import WYSIWYGEditor from '@/components/ui/wysiwyg';
import { DataTable, type ColumnDef } from '@/components/ui/table';
import { LabelPicker } from '@/components/tasks/LabelPicker';

interface TaskPanelProps {
  task: TaskWithAttemptStatus | null;
}

const TaskPanel = ({ task }: TaskPanelProps) => {
  const { t } = useTranslation('tasks');
  const navigate = useNavigateWithSearch();
  const { projectId } = useProject();
  const queryClient = useQueryClient();
  const [isMoving, setIsMoving] = useState<'start' | 'cancel' | null>(null);

  const { data: columns = [] } = useProjectColumns(projectId);

  const {
    data: attempts = [],
    isLoading: isAttemptsLoading,
    isError: isAttemptsError,
  } = useTaskAttemptsWithSessions(task?.id);

  const { data: parentAttempt, isLoading: isParentLoading } =
    useTaskAttemptWithSession(task?.parent_workspace_id || undefined);

  const { getLabelsForTask } = useTaskLabelAssignments(projectId);
  const taskLabels = task ? getLabelsForTask(task.id) : [];

  // Check if task is in a backlog (is_initial) or terminal column
  const currentColumn = columns.find((col) => col.id === task?.column_id);
  const isInBacklog = currentColumn?.is_initial ?? false;
  const isInTerminal = currentColumn?.is_terminal ?? false;
  const isSuccess = isInTerminal && currentColumn?.status === 'done';
  const isCancelled = isInTerminal && currentColumn?.status === 'cancelled';

  // Find target columns for Start and Cancel actions
  const workflowColumn = columns.find((col) => col.starts_workflow);
  const cancelledColumn = columns.find(
    (col) => col.is_terminal && col.status === 'cancelled'
  );

  const handleStartTask = async () => {
    if (!task || !workflowColumn) return;
    setIsMoving('start');
    try {
      await tasksApi.update(task.id, {
        title: task.title,
        description: task.description,
        status: workflowColumn.status,
        column_id: workflowColumn.id,
        parent_workspace_id: task.parent_workspace_id,
        image_ids: null,
      });
      // Invalidate tasks query to refresh the kanban board
      await queryClient.invalidateQueries({ queryKey: ['tasks'] });
      // Navigate to the latest attempt (which will be created by backend)
      if (projectId) {
        navigate(paths.attempt(projectId, task.id, 'latest'));
      }
    } catch (err) {
      console.error('Failed to start task:', err);
    } finally {
      setIsMoving(null);
    }
  };

  const handleCancelTask = async () => {
    if (!task || !cancelledColumn) return;
    setIsMoving('cancel');
    try {
      await tasksApi.update(task.id, {
        title: task.title,
        description: task.description,
        status: cancelledColumn.status,
        column_id: cancelledColumn.id,
        parent_workspace_id: task.parent_workspace_id,
        image_ids: null,
      });
      // Invalidate tasks query to refresh the kanban board
      await queryClient.invalidateQueries({ queryKey: ['tasks'] });
      // Navigate back to the kanban board
      if (projectId) {
        navigate(paths.projectTasks(projectId));
      }
    } catch (err) {
      console.error('Failed to cancel task:', err);
    } finally {
      setIsMoving(null);
    }
  };

  const formatTimeAgo = (iso: string) => {
    const d = new Date(iso);
    const diffMs = Date.now() - d.getTime();
    const absSec = Math.round(Math.abs(diffMs) / 1000);

    const rtf =
      typeof Intl !== 'undefined' &&
      typeof Intl.RelativeTimeFormat === 'function'
        ? new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
        : null;

    const to = (value: number, unit: Intl.RelativeTimeFormatUnit) =>
      rtf
        ? rtf.format(-value, unit)
        : `${value} ${unit}${value !== 1 ? 's' : ''} ago`;

    if (absSec < 60) return to(Math.round(absSec), 'second');
    const mins = Math.round(absSec / 60);
    if (mins < 60) return to(mins, 'minute');
    const hours = Math.round(mins / 60);
    if (hours < 24) return to(hours, 'hour');
    const days = Math.round(hours / 24);
    if (days < 30) return to(days, 'day');
    const months = Math.round(days / 30);
    if (months < 12) return to(months, 'month');
    const years = Math.round(months / 12);
    return to(years, 'year');
  };

  const displayedAttempts = [...attempts].sort(
    (a, b) =>
      new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
  );

  if (!task) {
    return (
      <div className="text-muted-foreground">
        {t('taskPanel.noTaskSelected')}
      </div>
    );
  }

  const titleContent = `# ${task.title || 'Task'}`;
  const descriptionContent = task.description || '';

  const attemptColumns: ColumnDef<WorkspaceWithSession>[] = [
    {
      id: 'status',
      header: '',
      accessor: (attempt) =>
        attempt.cancelled_at ? (
          <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
            {t('taskPanel.cancelled')}
          </span>
        ) : null,
      className: 'pr-2 w-0',
    },
    {
      id: 'executor',
      header: '',
      accessor: (attempt) => attempt.session?.executor || 'Base Agent',
      className: 'pr-4',
    },
    {
      id: 'branch',
      header: '',
      accessor: (attempt) => attempt.branch || '—',
      className: 'pr-4',
    },
    {
      id: 'time',
      header: '',
      accessor: (attempt) => formatTimeAgo(attempt.created_at),
      className: 'pr-0 text-right',
    },
  ];

  return (
    <>
      <NewCardContent>
        <div className="p-6 flex flex-col h-full max-h-[calc(100vh-8rem)]">
          <div className="space-y-3 overflow-y-auto flex-shrink min-h-0">
            <WYSIWYGEditor value={titleContent} disabled />
            {descriptionContent && (
              <WYSIWYGEditor value={descriptionContent} disabled />
            )}
          </div>

          {/* Labels section */}
          {projectId && (
            <div className="mt-4">
              <LabelPicker
                projectId={projectId}
                taskId={task.id}
                currentLabels={taskLabels}
                onLabelsChange={() => {
                  queryClient.invalidateQueries({
                    queryKey: taskLabelsKeys.assignments(projectId),
                  });
                }}
              />
            </div>
          )}

          {/* Terminal status indicator - show Success or Cancelled */}
          {isSuccess && (
            <div className="mt-4 flex items-center gap-2 text-green-600 dark:text-green-400">
              <CheckCircle2 className="h-5 w-5" />
              <span className="font-medium">{t('taskPanel.success', 'Completed Successfully')}</span>
            </div>
          )}
          {isCancelled && (
            <div className="mt-4 flex items-center gap-2 text-muted-foreground">
              <XCircle className="h-5 w-5" />
              <span className="font-medium">{t('taskPanel.taskCancelled', 'Task Cancelled')}</span>
            </div>
          )}

          {/* Backlog action buttons - only show when task is in backlog column */}
          {isInBacklog && (
            <div className="mt-6 flex gap-3">
              {workflowColumn && (
                <Button
                  onClick={handleStartTask}
                  disabled={isMoving !== null}
                  className="flex-1"
                >
                  {isMoving === 'start' ? (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  ) : (
                    <Play className="mr-2 h-4 w-4" />
                  )}
                  {t('taskPanel.startTask', 'Start')}
                </Button>
              )}
              {cancelledColumn && (
                <Button
                  variant="outline"
                  onClick={handleCancelTask}
                  disabled={isMoving !== null}
                  className="flex-1"
                >
                  {isMoving === 'cancel' ? (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  ) : (
                    <XCircle className="mr-2 h-4 w-4" />
                  )}
                  {t('taskPanel.cancelTask', 'Cancel')}
                </Button>
              )}
            </div>
          )}

          <div className="mt-6 flex-shrink-0 space-y-4">
            {task.parent_workspace_id && (
              <DataTable
                data={parentAttempt ? [parentAttempt] : []}
                columns={attemptColumns}
                keyExtractor={(attempt) => attempt.id}
                onRowClick={(attempt) => {
                  if (projectId) {
                    navigate(
                      paths.attempt(projectId, attempt.task_id, attempt.id)
                    );
                  }
                }}
                isLoading={isParentLoading}
                headerContent="Parent Attempt"
              />
            )}

            {isAttemptsLoading ? (
              <div className="text-muted-foreground">
                {t('taskPanel.loadingAttempts')}
              </div>
            ) : isAttemptsError ? (
              <div className="text-destructive">
                {t('taskPanel.errorLoadingAttempts')}
              </div>
            ) : (
              <DataTable
                data={displayedAttempts}
                columns={attemptColumns}
                keyExtractor={(attempt) => attempt.id}
                onRowClick={(attempt) => {
                  if (projectId && task.id) {
                    navigate(paths.attempt(projectId, task.id, attempt.id));
                  }
                }}
                emptyState={t('taskPanel.noAttempts')}
                headerContent={
                  <div className="w-full flex text-left">
                    <span className="flex-1">
                      {t('taskPanel.attemptsCount', {
                        count: displayedAttempts.length,
                      })}
                    </span>
                    <span>
                      <Button
                        variant="icon"
                        onClick={() =>
                          CreateAttemptDialog.show({
                            taskId: task.id,
                          })
                        }
                      >
                        <PlusIcon size={16} />
                      </Button>
                    </span>
                  </div>
                }
              />
            )}
          </div>
        </div>
      </NewCardContent>
    </>
  );
};

export default TaskPanel;
