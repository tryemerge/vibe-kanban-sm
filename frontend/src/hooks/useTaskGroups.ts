import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { taskGroupsApi } from '@/lib/api';
import { taskKeys } from '@/hooks/useTask';

// Query key factory for task groups
export const taskGroupsKeys = {
  all: ['taskGroups'] as const,
  byProject: (projectId: string) =>
    [...taskGroupsKeys.all, 'project', projectId] as const,
  dependencies: (projectId: string) =>
    [...taskGroupsKeys.all, 'dependencies', projectId] as const,
};

/**
 * Hook to fetch all task groups for a project
 */
export function useTaskGroups(projectId: string | undefined) {
  return useQuery({
    queryKey: taskGroupsKeys.byProject(projectId || ''),
    queryFn: () => taskGroupsApi.list(projectId!),
    enabled: !!projectId,
    staleTime: 30000,
  });
}

/**
 * Hook to fetch inter-group dependencies for a project
 */
export function useTaskGroupDependencies(projectId: string | undefined) {
  return useQuery({
    queryKey: taskGroupsKeys.dependencies(projectId || ''),
    queryFn: () => taskGroupsApi.listDependencies(projectId!),
    enabled: !!projectId,
    staleTime: 30000,
  });
}

/**
 * Hook for task group CRUD and assignment mutations
 */
export function useTaskGroupMutations(projectId: string) {
  const queryClient = useQueryClient();

  const invalidateGroups = () => {
    queryClient.invalidateQueries({
      queryKey: taskGroupsKeys.byProject(projectId),
    });
  };

  const invalidateGroupsAndTasks = () => {
    invalidateGroups();
    queryClient.invalidateQueries({
      queryKey: taskKeys.all,
    });
  };

  const createGroup = useMutation({
    mutationFn: (data: { name: string; color: string | null }) =>
      taskGroupsApi.create(projectId, data),
    onSuccess: invalidateGroups,
  });

  const updateGroup = useMutation({
    mutationFn: ({
      groupId,
      data,
    }: {
      groupId: string;
      data: { name: string | null; color: string | null };
    }) => taskGroupsApi.update(projectId, groupId, data),
    onSuccess: invalidateGroups,
  });

  const deleteGroup = useMutation({
    mutationFn: (groupId: string) => taskGroupsApi.delete(projectId, groupId),
    onSuccess: invalidateGroupsAndTasks,
  });

  const reorderGroups = useMutation({
    mutationFn: (groupIds: string[]) =>
      taskGroupsApi.reorder(projectId, groupIds),
    onSuccess: invalidateGroups,
  });

  const addTaskToGroup = useMutation({
    mutationFn: ({ taskId, groupId }: { taskId: string; groupId: string }) =>
      taskGroupsApi.addTask(taskId, groupId),
    onSuccess: invalidateGroupsAndTasks,
  });

  const removeTaskFromGroup = useMutation({
    mutationFn: (taskId: string) => taskGroupsApi.removeTask(taskId),
    onSuccess: invalidateGroupsAndTasks,
  });

  const addDependency = useMutation({
    mutationFn: ({
      groupId,
      dependsOnGroupId,
    }: {
      groupId: string;
      dependsOnGroupId: string;
    }) => taskGroupsApi.addDependency(projectId, groupId, dependsOnGroupId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: taskGroupsKeys.dependencies(projectId),
      });
    },
  });

  const removeDependency = useMutation({
    mutationFn: (depId: string) =>
      taskGroupsApi.removeDependency(projectId, depId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: taskGroupsKeys.dependencies(projectId),
      });
    },
  });

  return {
    createGroup,
    updateGroup,
    deleteGroup,
    reorderGroups,
    addTaskToGroup,
    removeTaskFromGroup,
    addDependency,
    removeDependency,
  };
}
