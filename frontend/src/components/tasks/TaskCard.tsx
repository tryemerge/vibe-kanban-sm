import { useCallback, useEffect, useRef, useState } from 'react';
import { KanbanCard } from '@/components/ui/shadcn-io/kanban';
import { Link, Loader2, Lock, XCircle } from 'lucide-react';
import type { TaskWithAttemptStatus } from 'shared/types';
import { ActionsDropdown } from '@/components/ui/actions-dropdown';
import { Button } from '@/components/ui/button';
import { useNavigateWithSearch, useTaskDependencies } from '@/hooks';
import { paths } from '@/lib/paths';
import { attemptsApi } from '@/lib/api';
import type { SharedTaskRecord } from '@/hooks/useProjectTasks';
import { TaskCardHeader } from './TaskCardHeader';
import { useTranslation } from 'react-i18next';
import { useAuth } from '@/hooks';
import { LabelBadges } from '@/components/ui/label-badge';
import { useTaskLabelsContextSafe } from '@/contexts/TaskLabelsContext';

type Task = TaskWithAttemptStatus;

interface TaskCardProps {
  task: Task;
  index: number;
  status: string;
  onViewDetails: (task: Task) => void;
  isOpen?: boolean;
  projectId: string;
  sharedTask?: SharedTaskRecord;
}

export function TaskCard({
  task,
  index,
  status,
  onViewDetails,
  isOpen,
  projectId,
  sharedTask,
}: TaskCardProps) {
  const { t } = useTranslation('tasks');
  const navigate = useNavigateWithSearch();
  const [isNavigatingToParent, setIsNavigatingToParent] = useState(false);
  const { isSignedIn } = useAuth();
  const { getLabelsForTask } = useTaskLabelsContextSafe();
  const taskLabels = getLabelsForTask(task.id);
  const { data: dependencies = [] } = useTaskDependencies(task.id);
  const isBlocked = dependencies.some((d) => d.satisfied_at === null);

  const handleClick = useCallback(() => {
    onViewDetails(task);
  }, [task, onViewDetails]);

  const handleParentClick = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      if (!task.parent_workspace_id || isNavigatingToParent) return;

      setIsNavigatingToParent(true);
      try {
        const parentAttempt = await attemptsApi.get(task.parent_workspace_id);
        navigate(
          paths.attempt(
            projectId,
            parentAttempt.task_id,
            task.parent_workspace_id
          )
        );
      } catch (error) {
        console.error('Failed to navigate to parent task attempt:', error);
        setIsNavigatingToParent(false);
      }
    },
    [task.parent_workspace_id, projectId, navigate, isNavigatingToParent]
  );

  const localRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen || !localRef.current) return;
    const el = localRef.current;
    requestAnimationFrame(() => {
      el.scrollIntoView({
        block: 'center',
        inline: 'nearest',
        behavior: 'smooth',
      });
    });
  }, [isOpen]);

  return (
    <KanbanCard
      key={task.id}
      id={task.id}
      name={task.title}
      index={index}
      parent={status}
      onClick={handleClick}
      isOpen={isOpen}
      forwardedRef={localRef}
      dragDisabled={(!!sharedTask || !!task.shared_task_id) && !isSignedIn}
      className={
        sharedTask || task.shared_task_id
          ? 'relative overflow-hidden pl-5 before:absolute before:left-0 before:top-0 before:bottom-0 before:w-[3px] before:bg-card-foreground before:content-[""]'
          : undefined
      }
    >
      <div className="flex flex-col gap-2">
        <TaskCardHeader
          title={task.title}
          avatar={
            sharedTask
              ? {
                  firstName: sharedTask.assignee_first_name ?? undefined,
                  lastName: sharedTask.assignee_last_name ?? undefined,
                  username: sharedTask.assignee_username ?? undefined,
                }
              : undefined
          }
          right={
            <>
              {isBlocked && (
                <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-600 border border-amber-500/20">
                  <Lock className="h-2.5 w-2.5" />
                  Blocked
                </span>
              )}
              {task.status !== 'done' && task.status !== 'cancelled' && (
                <>
                  {task.task_state === 'inprogress' && (
                    <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-600 border border-blue-500/20">
                      <span className="relative flex h-1.5 w-1.5">
                        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75" />
                        <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-blue-500" />
                      </span>
                      Agent Working...
                    </span>
                  )}
                  {task.task_state === 'awaitingresponse' && (
                    <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-600 border border-amber-500/20">
                      <span className="h-1.5 w-1.5 rounded-full bg-amber-500" />
                      Agent Waiting for Response
                    </span>
                  )}
                  {task.task_state === 'transitioning' && (
                    <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-purple-500/10 text-purple-600 border border-purple-500/20">
                      <Loader2 className="h-2.5 w-2.5 animate-spin" />
                      Transitioning...
                    </span>
                  )}
                  {task.task_state === 'queued' && !task.has_in_progress_attempt && (
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground border">
                      Queued
                    </span>
                  )}
                  {task.has_in_progress_attempt && task.task_state === 'queued' && (
                    <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
                  )}
                </>
              )}
              {task.last_attempt_failed && (
                <XCircle className="h-4 w-4 text-destructive" />
              )}
              {task.parent_workspace_id && (
                <Button
                  variant="icon"
                  onClick={handleParentClick}
                  onPointerDown={(e) => e.stopPropagation()}
                  onMouseDown={(e) => e.stopPropagation()}
                  disabled={isNavigatingToParent}
                  title={t('navigateToParent')}
                >
                  <Link className="h-4 w-4" />
                </Button>
              )}
              <ActionsDropdown task={task} sharedTask={sharedTask} />
            </>
          }
        />
        {task.description && (
          <p className="text-sm text-secondary-foreground break-words">
            {task.description.length > 130
              ? `${task.description.substring(0, 130)}...`
              : task.description}
          </p>
        )}
        {taskLabels.length > 0 && (
          <LabelBadges labels={taskLabels} size="sm" maxDisplay={3} />
        )}
      </div>
    </KanbanCard>
  );
}
