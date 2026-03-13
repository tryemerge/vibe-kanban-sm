import { useCallback, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { taskGroupsApi } from '@/lib/api';
import { taskKeys } from '@/hooks/useTask';
import { useJsonPatchWsStream } from './useJsonPatchWsStream';
import type { TaskGroup } from 'shared/types';

// Query key factory for task groups
export const taskGroupsKeys = {
  all: ['taskGroups'] as const,
  byProject: (projectId: string) =>
    [...taskGroupsKeys.all, 'project', projectId] as const,
  dependencies: (projectId: string) =>
    [...taskGroupsKeys.all, 'dependencies', projectId] as const,
};

type TaskGroupsState = {
  task_groups: Record<string, TaskGroup>;
};

/**
 * Hook to stream all task groups for a project via WebSocket
 */
export function useTaskGroups(projectId: string | undefined) {
  const endpoint = projectId
    ? `/api/projects/${encodeURIComponent(projectId)}/task-groups/stream/ws`
    : undefined;

  const initialData = useCallback((): TaskGroupsState => ({ task_groups: {} }), []);

  const { data, isConnected, error } = useJsonPatchWsStream(
    endpoint,
    !!projectId,
    initialData
  );

  const groups = useMemo(() => {
    if (!data?.task_groups) return [];
    return Object.values(data.task_groups);
  }, [data]);

  return {
    data: groups,
    isLoading: !isConnected && groups.length === 0,
    isConnected,
    error,
  };
}

/**
 * Hook to fetch inter-group dependencies for a project
 */
export function useTaskGroupDependencies(projectId: string | undefined) {
  return useQuery({
    queryKey: taskGroupsKeys.dependencies(projectId || ''),
    queryFn: () => taskGroupsApi.listDependencies(projectId!),
    enabled: !!projectId,
    staleTime: 5000,
    refetchInterval: 5000,
  });
}

/**
 * Hook for task group CRUD and assignment mutations
 */
export function useTaskGroupMutations(projectId: string) {
  const queryClient = useQueryClient();

  // Dependencies still use React Query, so we invalidate those
  const invalidateDeps = () => {
    queryClient.invalidateQueries({
      queryKey: taskGroupsKeys.dependencies(projectId),
    });
  };

  const invalidateTasks = () => {
    queryClient.invalidateQueries({
      queryKey: taskKeys.all,
    });
  };

  const createGroup = useMutation({
    mutationFn: (data: { name: string; color: string | null; is_backlog: boolean | null; artifact_id?: string | null }) =>
      taskGroupsApi.create(projectId, { ...data, artifact_id: data.artifact_id ?? null }),
    // WS stream handles the read update
  });

  const updateGroup = useMutation({
    mutationFn: ({
      groupId,
      data,
    }: {
      groupId: string;
      data: { name: string | null; color: string | null };
    }) => taskGroupsApi.update(projectId, groupId, data),
  });

  const deleteGroup = useMutation({
    mutationFn: (groupId: string) => taskGroupsApi.delete(projectId, groupId),
    onSuccess: invalidateTasks,
  });

  const reorderGroups = useMutation({
    mutationFn: (groupIds: string[]) =>
      taskGroupsApi.reorder(projectId, groupIds),
  });

  const addTaskToGroup = useMutation({
    mutationFn: ({ taskId, groupId }: { taskId: string; groupId: string }) =>
      taskGroupsApi.addTask(taskId, groupId),
    onSuccess: invalidateTasks,
  });

  const removeTaskFromGroup = useMutation({
    mutationFn: (taskId: string) => taskGroupsApi.removeTask(taskId),
    onSuccess: invalidateTasks,
  });

  const addDependency = useMutation({
    mutationFn: ({
      groupId,
      dependsOnGroupId,
    }: {
      groupId: string;
      dependsOnGroupId: string;
    }) => taskGroupsApi.addDependency(projectId, groupId, dependsOnGroupId),
    onSuccess: invalidateDeps,
  });

  const removeDependency = useMutation({
    mutationFn: (depId: string) =>
      taskGroupsApi.removeDependency(projectId, depId),
    onSuccess: invalidateDeps,
  });

  const transitionGroup = useMutation({
    mutationFn: ({
      groupId,
      from,
      to,
    }: {
      groupId: string;
      from: string;
      to: string;
    }) => taskGroupsApi.transition(groupId, from, to),
    // WS stream handles the read update
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
    transitionGroup,
  };
}
