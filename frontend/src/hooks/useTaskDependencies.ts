import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { taskDependenciesApi } from '@/lib/api';

// Query key factory for task dependencies
export const taskDependenciesKeys = {
  all: ['taskDependencies'] as const,
  byTask: (taskId: string) =>
    [...taskDependenciesKeys.all, 'task', taskId] as const,
};

/**
 * Hook to fetch dependencies for a task (what this task is waiting for)
 */
export function useTaskDependencies(taskId: string | undefined) {
  return useQuery({
    queryKey: taskDependenciesKeys.byTask(taskId || ''),
    queryFn: () => taskDependenciesApi.list(taskId!),
    enabled: !!taskId,
    staleTime: 30000,
  });
}

/**
 * Hook for dependency mutations (create/delete)
 */
export function useTaskDependencyMutations(taskId: string) {
  const queryClient = useQueryClient();

  const createDependency = useMutation({
    mutationFn: (dependsOnTaskId: string) =>
      taskDependenciesApi.create(taskId, {
        depends_on_task_id: dependsOnTaskId,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: taskDependenciesKeys.byTask(taskId),
      });
    },
  });

  const deleteDependency = useMutation({
    mutationFn: (dependencyId: string) =>
      taskDependenciesApi.delete(taskId, dependencyId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: taskDependenciesKeys.byTask(taskId),
      });
    },
  });

  return {
    createDependency,
    deleteDependency,
  };
}
