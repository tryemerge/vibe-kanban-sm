import { memo } from 'react';
import { useAuth } from '@/hooks';
import {
  type DragEndEvent,
  KanbanBoard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
} from '@/components/ui/shadcn-io/kanban';
import { TaskCard } from './TaskCard';
import type { TaskWithAttemptStatus } from 'shared/types';
import { statusBoardColors } from '@/utils/statusLabels';
import type { SharedTaskRecord } from '@/hooks/useProjectTasks';
import { SharedTaskCard } from './SharedTaskCard';

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
}: TaskKanbanBoardProps) {
  const { userId } = useAuth();

  return (
    <KanbanProvider onDragEnd={onDragEnd}>
      {columnDefs.map((column) => {
        const items = columnItems[column.id] ?? [];
        // Use column color if provided, otherwise fall back to status-based color
        const columnColor = column.color ?? statusBoardColors[column.slug as keyof typeof statusBoardColors] ?? 'gray';

        return (
          <KanbanBoard key={column.id} id={column.slug}>
            <KanbanHeader
              name={column.name}
              color={columnColor}
              onAddTask={onCreateTask}
            />
            <KanbanCards>
              {items.map((item, index) => {
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
                      status={column.slug}
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
                    status={column.slug}
                    isSelected={selectedSharedTaskId === item.task.id}
                    onViewDetails={onViewSharedTask}
                  />
                );
              })}
            </KanbanCards>
          </KanbanBoard>
        );
      })}
    </KanbanProvider>
  );
}

export default memo(TaskKanbanBoard);
