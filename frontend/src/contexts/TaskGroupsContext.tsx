import { createContext, useContext, useMemo, type ReactNode } from 'react';
import { useTaskGroups } from '@/hooks/useTaskGroups';
import type { TaskGroup } from 'shared/types';

interface TaskGroupsContextValue {
  groups: TaskGroup[];
  groupsLoading: boolean;
  getGroupForTask: (taskGroupId: string | null) => TaskGroup | undefined;
  getGroupById: (groupId: string) => TaskGroup | undefined;
}

const TaskGroupsContext = createContext<TaskGroupsContextValue | null>(null);

export function TaskGroupsProvider({
  projectId,
  children,
}: {
  projectId: string | undefined;
  children: ReactNode;
}) {
  const { data: groups = [], isLoading: groupsLoading } =
    useTaskGroups(projectId);

  const groupsById = useMemo(() => {
    const map = new Map<string, TaskGroup>();
    for (const group of groups) {
      map.set(group.id, group);
    }
    return map;
  }, [groups]);

  const getGroupForTask = (taskGroupId: string | null) => {
    if (!taskGroupId) return undefined;
    return groupsById.get(taskGroupId);
  };

  const getGroupById = (groupId: string) => groupsById.get(groupId);

  return (
    <TaskGroupsContext.Provider
      value={{
        groups,
        groupsLoading,
        getGroupForTask,
        getGroupById,
      }}
    >
      {children}
    </TaskGroupsContext.Provider>
  );
}

export function useTaskGroupsContext() {
  const context = useContext(TaskGroupsContext);
  if (!context) {
    throw new Error(
      'useTaskGroupsContext must be used within a TaskGroupsProvider'
    );
  }
  return context;
}

/**
 * Safe version that returns empty data when not in a provider
 */
export function useTaskGroupsContextSafe(): TaskGroupsContextValue {
  const context = useContext(TaskGroupsContext);
  return (
    context ?? {
      groups: [],
      groupsLoading: false,
      getGroupForTask: () => undefined,
      getGroupById: () => undefined,
    }
  );
}
