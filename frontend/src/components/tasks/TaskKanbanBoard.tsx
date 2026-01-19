import { memo, useMemo } from 'react';
import { useAuth } from '@/hooks';
import {
  type DragEndEvent,
  KanbanBoard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
} from '@/components/ui/shadcn-io/kanban';
import { TaskCard } from './TaskCard';
import type { TaskWithAttemptStatus, TaskLabel } from 'shared/types';
import { statusBoardColors } from '@/utils/statusLabels';
import type { SharedTaskRecord } from '@/hooks/useProjectTasks';
import { SharedTaskCard } from './SharedTaskCard';
import { SwimLane } from './SwimLane';
import type { SwimLaneConfig } from '@/hooks/useSwimLaneConfig';
import { useTaskLabelsContextSafe } from '@/contexts/TaskLabelsContext';

export type KanbanColumnItem =
  | {
      type: 'task';
      task: TaskWithAttemptStatus;
      sharedTask?: SharedTaskRecord;
    }
  | {
      type: 'shared';
      task: SharedTaskRecord;
    };

// Column items keyed by column ID (or slug for backwards compatibility)
export type KanbanColumnItems = Record<string, KanbanColumnItem[]>;

// Column definition for rendering
export interface KanbanColumnDef {
  id: string;
  name: string;
  slug: string;
  color: string | null;
}

interface TaskKanbanBoardProps {
  columnDefs: KanbanColumnDef[];
  columnItems: KanbanColumnItems;
  onDragEnd: (event: DragEndEvent) => void;
  onViewTaskDetails: (task: TaskWithAttemptStatus) => void;
  onViewSharedTask?: (task: SharedTaskRecord) => void;
  selectedTaskId?: string;
  selectedSharedTaskId?: string | null;
  onCreateTask?: () => void;
  projectId: string;
  swimLaneConfig?: SwimLaneConfig;
  onToggleLaneCollapse?: (laneId: string) => void;
}

interface SwimLaneGroup {
  id: string;
  title: string | null;
  color: string | null;
  items: KanbanColumnItem[];
}

/**
 * Group column items by label for swim lane display
 */
function groupItemsByLabel(
  items: KanbanColumnItem[],
  labels: TaskLabel[],
  getLabelsForTask: (taskId: string) => TaskLabel[],
  showUnlabeled: boolean
): SwimLaneGroup[] {
  // Create a map of label ID -> items
  const labelGroups = new Map<string, KanbanColumnItem[]>();
  const unlabeledItems: KanbanColumnItem[] = [];

  // Initialize groups for all labels (to preserve label order)
  for (const label of labels) {
    labelGroups.set(label.id, []);
  }

  // Assign items to groups
  for (const item of items) {
    const taskId = item.type === 'task' ? item.task.id : item.task.id;
    const taskLabels = getLabelsForTask(taskId);

    if (taskLabels.length === 0) {
      unlabeledItems.push(item);
    } else {
      // Add to each label group the task belongs to
      // (tasks appear in multiple lanes if they have multiple labels)
      for (const label of taskLabels) {
        const group = labelGroups.get(label.id);
        if (group) {
          group.push(item);
        }
      }
    }
  }

  // Build result array in label order
  const result: SwimLaneGroup[] = [];

  for (const label of labels) {
    const group = labelGroups.get(label.id);
    if (group && group.length > 0) {
      result.push({
        id: label.id,
        title: label.name,
        color: label.color,
        items: group,
      });
    }
  }

  // Add unlabeled group at the end if there are items and showUnlabeled is true
  if (showUnlabeled && unlabeledItems.length > 0) {
    result.push({
      id: '__unlabeled__',
      title: 'No Label',
      color: null,
      items: unlabeledItems,
    });
  }

  return result;
}

function TaskKanbanBoard({
  columnDefs,
  columnItems,
  onDragEnd,
  onViewTaskDetails,
  onViewSharedTask,
  selectedTaskId,
  selectedSharedTaskId,
  onCreateTask,
  projectId,
  swimLaneConfig,
  onToggleLaneCollapse,
}: TaskKanbanBoardProps) {
  const { userId } = useAuth();
  const { labels, getLabelsForTask } = useTaskLabelsContextSafe();

  const isSwimLanesEnabled = swimLaneConfig?.groupBy.type === 'label';
  const showUnlabeled = swimLaneConfig?.showUnlabeled ?? true;

  // Pre-compute swim lane groups for all columns
  const swimLaneGroupsByColumn = useMemo(() => {
    if (!isSwimLanesEnabled) {
      return null;
    }

    const result: Record<string, SwimLaneGroup[]> = {};
    for (const column of columnDefs) {
      const items = columnItems[column.id] ?? [];
      result[column.id] = groupItemsByLabel(
        items,
        labels,
        getLabelsForTask,
        showUnlabeled
      );
    }
    return result;
  }, [columnDefs, columnItems, labels, getLabelsForTask, isSwimLanesEnabled, showUnlabeled]);

  // Render a single item (task or shared task)
  const renderItem = (
    item: KanbanColumnItem,
    index: number,
    columnSlug: string
  ) => {
    const isOwnTask =
      item.type === 'task' &&
      (!item.sharedTask?.assignee_user_id ||
        !userId ||
        item.sharedTask?.assignee_user_id === userId);

    if (isOwnTask) {
      return (
        <TaskCard
          key={item.task.id}
          task={item.task}
          index={index}
          status={columnSlug}
          onViewDetails={onViewTaskDetails}
          isOpen={selectedTaskId === item.task.id}
          projectId={projectId}
          sharedTask={item.sharedTask}
        />
      );
    }

    const sharedTask =
      item.type === 'shared' ? item.task : item.sharedTask!;

    return (
      <SharedTaskCard
        key={`shared-${item.task.id}`}
        task={sharedTask}
        index={index}
        status={columnSlug}
        isSelected={selectedSharedTaskId === item.task.id}
        onViewDetails={onViewSharedTask}
      />
    );
  };

  return (
    <KanbanProvider onDragEnd={onDragEnd}>
      {columnDefs.map((column) => {
        const items = columnItems[column.id] ?? [];
        // Use column color if provided, otherwise fall back to status-based color
        const columnColor = column.color ?? statusBoardColors[column.slug as keyof typeof statusBoardColors] ?? 'gray';

        // Get pre-computed swim lane groups for this column
        const swimLaneGroups = swimLaneGroupsByColumn?.[column.id];

        return (
          <KanbanBoard key={column.id} id={column.slug}>
            <KanbanHeader
              name={column.name}
              color={columnColor}
              onAddTask={onCreateTask}
            />
            <KanbanCards>
              {isSwimLanesEnabled && swimLaneGroups ? (
                // Render with swim lanes
                swimLaneGroups.map((lane) => (
                  <SwimLane
                    key={lane.id}
                    title={lane.title}
                    taskCount={lane.items.length}
                    collapsed={swimLaneConfig?.collapsedLanes.includes(lane.id) ?? false}
                    onToggleCollapse={() => onToggleLaneCollapse?.(lane.id)}
                    color={lane.color}
                  >
                    {lane.items.map((item, index) =>
                      renderItem(item, index, column.slug)
                    )}
                  </SwimLane>
                ))
              ) : (
                // Render flat list (current behavior)
                items.map((item, index) =>
                  renderItem(item, index, column.slug)
                )
              )}
            </KanbanCards>
          </KanbanBoard>
        );
      })}
    </KanbanProvider>
  );
}

export default memo(TaskKanbanBoard);
