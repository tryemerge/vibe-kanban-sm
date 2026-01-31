import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { taskTriggersApi } from '@/lib/api';

// Query key factory for task triggers
export const taskTriggersKeys = {
  all: ['taskTriggers'] as const,
  byTask: (taskId: string) => [...taskTriggersKeys.all, 'task', taskId] as const,
};

/**
 * Hook to fetch triggers for a task (what this task is waiting for)
 */
export function useTaskTriggers(taskId: string | undefined) {
  return useQuery({
    queryKey: taskTriggersKeys.byTask(taskId || ''),
    queryFn: () => taskTriggersApi.list(taskId!),
    enabled: !!taskId,
    staleTime: 30000,
  });
}

/**
 * Hook for trigger mutations (create/delete)
 */
export function useTaskTriggerMutations(taskId: string) {
  const queryClient = useQueryClient();

  const createTrigger = useMutation({
    mutationFn: ({
      triggerTaskId,
      triggerOn = 'completed',
      isPersistent = false,
    }: {
      triggerTaskId: string;
      triggerOn?: string;
      isPersistent?: boolean;
    }) =>
      taskTriggersApi.create(taskId, {
        trigger_task_id: triggerTaskId,
        trigger_on: triggerOn,
        is_persistent: isPersistent,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: taskTriggersKeys.byTask(taskId),
      });
    },
  });

  const deleteTrigger = useMutation({
    mutationFn: (triggerId: string) =>
      taskTriggersApi.delete(taskId, triggerId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: taskTriggersKeys.byTask(taskId),
      });
    },
  });

  return {
    createTrigger,
    deleteTrigger,
  };
}
