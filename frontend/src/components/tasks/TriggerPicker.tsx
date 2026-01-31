import { useState } from 'react';
import { X, PlayCircle, Loader2, CheckCircle2, Clock } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useTaskTriggers, useTaskTriggerMutations, taskTriggersKeys } from '@/hooks/useTaskTriggers';
import { tasksApi } from '@/lib/api';
import type { TaskTrigger, TaskWithAttemptStatus } from 'shared/types';

interface TriggerPickerProps {
  projectId: string;
  taskId: string;
  size?: 'sm' | 'default';
}

/**
 * Displays a trigger as a task reference with the condition and fired status
 */
function TriggerBadge({
  trigger,
  task,
  onRemove,
}: {
  trigger: TaskTrigger;
  task?: TaskWithAttemptStatus;
  onRemove: () => void;
}) {
  const title = task?.title || 'Unknown Task';
  const isFired = trigger.fired_at !== null;
  const conditionLabel =
    trigger.trigger_on === 'completed'
      ? 'completes'
      : trigger.trigger_on === 'merged'
        ? 'is merged'
        : `completes with ${trigger.trigger_on}`;

  return (
    <div
      className={cn(
        'flex items-center gap-1 px-2 py-1 rounded-md text-xs group',
        isFired ? 'bg-green-500/10' : 'bg-muted'
      )}
    >
      {isFired ? (
        <CheckCircle2 className="h-3 w-3 text-green-500 flex-shrink-0" />
      ) : (
        <Clock className="h-3 w-3 text-muted-foreground flex-shrink-0" />
      )}
      <span className="truncate max-w-[180px]" title={title}>
        When <strong>{title}</strong> {conditionLabel}
      </span>
      {isFired && (
        <span className="text-green-600 text-[10px] ml-1">(ready)</span>
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

export function TriggerPicker({ projectId, taskId, size = 'default' }: TriggerPickerProps) {
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();

  // Fetch current triggers for this task
  const { data: triggers = [], isLoading: triggersLoading } = useTaskTriggers(taskId);
  const { createTrigger, deleteTrigger } = useTaskTriggerMutations(taskId);

  // Fetch all tasks for this project to show as options
  const { data: allTasks = [], isLoading: tasksLoading } = useQuery({
    queryKey: ['tasks', 'list', projectId],
    queryFn: () => tasksApi.listByProject(projectId),
    enabled: !!projectId,
    staleTime: 30000,
  });

  // Build a map of task IDs that are already triggers
  const triggerTaskIds = new Set(triggers.map((t) => t.trigger_task_id));

  // Build a map of all tasks by ID for quick lookup
  const tasksById = new Map(allTasks.map((t) => [t.id, t]));

  // Filter out the current task and tasks that are already triggers
  const availableTasks = allTasks.filter(
    (t) => t.id !== taskId && !triggerTaskIds.has(t.id)
  );

  const handleAddTrigger = async (triggerTaskId: string) => {
    await createTrigger.mutateAsync({
      triggerTaskId,
      triggerOn: 'completed',
      isPersistent: false,
    });
    queryClient.invalidateQueries({
      queryKey: taskTriggersKeys.byTask(taskId),
    });
  };

  const handleRemoveTrigger = async (triggerId: string) => {
    await deleteTrigger.mutateAsync(triggerId);
    queryClient.invalidateQueries({
      queryKey: taskTriggersKeys.byTask(taskId),
    });
  };

  const isLoading = triggersLoading || tasksLoading;

  return (
    <div className="flex flex-col gap-2">
      {/* Display current triggers */}
      {triggers.length > 0 && (
        <div className="flex flex-col gap-1">
          {triggers.map((trigger) => (
            <TriggerBadge
              key={trigger.id}
              trigger={trigger}
              task={tasksById.get(trigger.trigger_task_id)}
              onRemove={() => handleRemoveTrigger(trigger.id)}
            />
          ))}
        </div>
      )}

      {/* Trigger picker popover */}
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
            <PlayCircle className={cn('mr-1', size === 'sm' ? 'h-3 w-3' : 'h-4 w-4')} />
            {triggers.length === 0 ? 'Add trigger' : 'Add another trigger'}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-72 p-2" align="start">
          <div className="space-y-2">
            <div className="text-sm font-medium text-muted-foreground px-2 py-1">
              Start after task completes
            </div>
            {isLoading ? (
              <div className="flex items-center justify-center py-4">
                <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
              </div>
            ) : availableTasks.length === 0 ? (
              <div className="text-sm text-muted-foreground px-2 py-4 text-center">
                {allTasks.length <= 1
                  ? 'No other tasks in this project.'
                  : 'All tasks are already triggers.'}
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
                    onClick={() => handleAddTrigger(task.id)}
                    disabled={createTrigger.isPending}
                  >
                    <PlayCircle className="h-4 w-4 text-muted-foreground flex-shrink-0" />
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
