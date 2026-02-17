import { useState } from 'react';
import { X, Lock, Loader2, CheckCircle2, LockOpen } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import {
  useTaskDependencies,
  useTaskDependencyMutations,
  taskDependenciesKeys,
} from '@/hooks/useTaskDependencies';
import { tasksApi } from '@/lib/api';
import type { TaskDependency, TaskWithAttemptStatus } from 'shared/types';

interface DependencyPickerProps {
  projectId: string;
  taskId: string;
  size?: 'sm' | 'default';
}

function DependencyBadge({
  dependency,
  task,
  onRemove,
}: {
  dependency: TaskDependency;
  task?: TaskWithAttemptStatus;
  onRemove: () => void;
}) {
  const title = task?.title || 'Unknown Task';
  const isSatisfied = dependency.satisfied_at !== null;

  return (
    <div
      className={cn(
        'flex items-center gap-1 px-2 py-1 rounded-md text-xs group',
        isSatisfied ? 'bg-green-500/10' : 'bg-amber-500/10'
      )}
    >
      {isSatisfied ? (
        <CheckCircle2 className="h-3 w-3 text-green-500 flex-shrink-0" />
      ) : (
        <Lock className="h-3 w-3 text-amber-500 flex-shrink-0" />
      )}
      <span className="truncate max-w-[180px]" title={title}>
        Depends on <strong>{title}</strong>
      </span>
      {isSatisfied ? (
        <span className="text-green-600 text-[10px] ml-1">(done)</span>
      ) : (
        <span className="text-amber-600 text-[10px] ml-1">(blocked)</span>
      )}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        className="ml-1 opacity-0 group-hover:opacity-100 transition-opacity"
      >
        <X className="h-3 w-3 text-muted-foreground hover:text-foreground" />
      </button>
    </div>
  );
}

export function DependencyPicker({
  projectId,
  taskId,
  size = 'default',
}: DependencyPickerProps) {
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();

  const { data: dependencies = [], isLoading: depsLoading } =
    useTaskDependencies(taskId);
  const { createDependency, deleteDependency } =
    useTaskDependencyMutations(taskId);

  const { data: allTasks = [], isLoading: tasksLoading } = useQuery({
    queryKey: ['tasks', 'list', projectId],
    queryFn: () => tasksApi.listByProject(projectId),
    enabled: !!projectId,
    staleTime: 30000,
  });

  const depTaskIds = new Set(dependencies.map((d) => d.depends_on_task_id));
  const tasksById = new Map(allTasks.map((t) => [t.id, t]));

  const availableTasks = allTasks.filter(
    (t) => t.id !== taskId && !depTaskIds.has(t.id)
  );

  const handleAdd = async (dependsOnTaskId: string) => {
    await createDependency.mutateAsync(dependsOnTaskId);
    queryClient.invalidateQueries({
      queryKey: taskDependenciesKeys.byTask(taskId),
    });
  };

  const handleRemove = async (dependencyId: string) => {
    await deleteDependency.mutateAsync(dependencyId);
    queryClient.invalidateQueries({
      queryKey: taskDependenciesKeys.byTask(taskId),
    });
  };

  const isLoading = depsLoading || tasksLoading;
  const hasUnsatisfied = dependencies.some((d) => d.satisfied_at === null);

  return (
    <div className="flex flex-col gap-2">
      {dependencies.length > 0 && (
        <div className="flex flex-col gap-1">
          {dependencies.map((dep) => (
            <DependencyBadge
              key={dep.id}
              dependency={dep}
              task={tasksById.get(dep.depends_on_task_id)}
              onRemove={() => handleRemove(dep.id)}
            />
          ))}
        </div>
      )}

      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="ghost"
            size={size === 'sm' ? 'sm' : 'default'}
            className={cn(
              'justify-start text-muted-foreground hover:text-foreground',
              size === 'sm' && 'h-7 px-2 text-xs'
            )}
          >
            {hasUnsatisfied ? (
              <Lock
                className={cn(
                  'mr-1 text-amber-500',
                  size === 'sm' ? 'h-3 w-3' : 'h-4 w-4'
                )}
              />
            ) : (
              <LockOpen
                className={cn('mr-1', size === 'sm' ? 'h-3 w-3' : 'h-4 w-4')}
              />
            )}
            {dependencies.length === 0
              ? 'Add dependency'
              : 'Add another dependency'}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-72 p-2" align="start">
          <div className="space-y-2">
            <div className="text-sm font-medium text-muted-foreground px-2 py-1">
              Must complete before this task
            </div>
            {isLoading ? (
              <div className="flex items-center justify-center py-4">
                <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
              </div>
            ) : availableTasks.length === 0 ? (
              <div className="text-sm text-muted-foreground px-2 py-4 text-center">
                {allTasks.length <= 1
                  ? 'No other tasks in this project.'
                  : 'All tasks are already dependencies.'}
              </div>
            ) : (
              <div className="space-y-1 max-h-64 overflow-y-auto">
                {availableTasks.map((task) => (
                  <button
                    key={task.id}
                    className={cn(
                      'w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors',
                      'hover:bg-muted/50 text-left'
                    )}
                    onClick={() => handleAdd(task.id)}
                    disabled={createDependency.isPending}
                  >
                    <Lock className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                    <span className="flex-1 truncate">{task.title}</span>
                    <span className="text-xs text-muted-foreground capitalize">
                      {task.status}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
}
