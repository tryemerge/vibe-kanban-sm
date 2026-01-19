import { createContext, useContext, type ReactNode } from 'react';
import { useTaskLabelAssignments, useProjectLabels } from '@/hooks/useTaskLabels';
import type { TaskLabel } from 'shared/types';

interface TaskLabelsContextValue {
  labels: TaskLabel[];
  labelsLoading: boolean;
  getLabelsForTask: (taskId: string) => TaskLabel[];
}

const TaskLabelsContext = createContext<TaskLabelsContextValue | null>(null);

export function TaskLabelsProvider({
  projectId,
  children,
}: {
  projectId: string | undefined;
  children: ReactNode;
}) {
  const { data: labels = [], isLoading: labelsLoading } =
    useProjectLabels(projectId);
  const { getLabelsForTask } = useTaskLabelAssignments(projectId);

  return (
    <TaskLabelsContext.Provider
      value={{
        labels,
        labelsLoading,
        getLabelsForTask,
      }}
    >
      {children}
    </TaskLabelsContext.Provider>
  );
}

export function useTaskLabelsContext() {
  const context = useContext(TaskLabelsContext);
  if (!context) {
    throw new Error(
      'useTaskLabelsContext must be used within a TaskLabelsProvider'
    );
  }
  return context;
}

/**
 * Safe version that returns empty data when not in a provider
 * Useful for components that may render outside the kanban board
 */
export function useTaskLabelsContextSafe(): TaskLabelsContextValue {
  const context = useContext(TaskLabelsContext);
  return (
    context ?? {
      labels: [],
      labelsLoading: false,
      getLabelsForTask: () => [],
    }
  );
}
