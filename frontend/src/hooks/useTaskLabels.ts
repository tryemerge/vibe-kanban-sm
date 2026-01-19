import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { labelsApi } from '@/lib/api';
import type { TaskLabel } from 'shared/types';
import { useMemo } from 'react';

// Query key factory for task labels
export const taskLabelsKeys = {
  all: ['taskLabels'] as const,
  byProject: (projectId: string) =>
    [...taskLabelsKeys.all, 'project', projectId] as const,
  assignments: (projectId: string) =>
    [...taskLabelsKeys.all, 'assignments', projectId] as const,
};

/**
 * Hook to fetch all labels for a project
 */
export function useProjectLabels(projectId: string | undefined) {
  return useQuery({
    queryKey: taskLabelsKeys.byProject(projectId || ''),
    queryFn: () => labelsApi.listByProject(projectId!),
    enabled: !!projectId,
    staleTime: 30000, // Cache for 30 seconds
  });
}

/**
 * Hook to fetch all task-label assignments for a project
 * Returns a map of taskId -> labels[] for efficient lookup
 */
export function useTaskLabelAssignments(projectId: string | undefined) {
  const query = useQuery({
    queryKey: taskLabelsKeys.assignments(projectId || ''),
    queryFn: () => labelsApi.getProjectAssignments(projectId!),
    enabled: !!projectId,
    staleTime: 30000, // Cache for 30 seconds
  });

  // Convert to a map for efficient lookup
  const labelsByTaskId = useMemo(() => {
    const map = new Map<string, TaskLabel[]>();
    if (query.data) {
      for (const assignment of query.data) {
        const existing = map.get(assignment.task_id) || [];
        existing.push(assignment.label);
        map.set(assignment.task_id, existing);
      }
    }
    return map;
  }, [query.data]);

  return {
    ...query,
    labelsByTaskId,
    getLabelsForTask: (taskId: string): TaskLabel[] =>
      labelsByTaskId.get(taskId) || [],
  };
}

/**
 * Hook for label assignment mutations
 */
export function useTaskLabelMutations(projectId: string) {
  const queryClient = useQueryClient();

  const assignLabel = useMutation({
    mutationFn: ({ taskId, labelId }: { taskId: string; labelId: string }) =>
      labelsApi.assignToTask(taskId, labelId),
    onSuccess: () => {
      // Invalidate assignments to refetch
      queryClient.invalidateQueries({
        queryKey: taskLabelsKeys.assignments(projectId),
      });
    },
  });

  const removeLabel = useMutation({
    mutationFn: ({ taskId, labelId }: { taskId: string; labelId: string }) =>
      labelsApi.removeFromTask(taskId, labelId),
    onSuccess: () => {
      // Invalidate assignments to refetch
      queryClient.invalidateQueries({
        queryKey: taskLabelsKeys.assignments(projectId),
      });
    },
  });

  return {
    assignLabel,
    removeLabel,
  };
}
