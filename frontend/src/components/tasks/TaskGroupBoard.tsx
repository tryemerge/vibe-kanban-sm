import { memo, useMemo, useState } from 'react';
import type { TaskGroup } from 'shared/types';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { FlaskConical, ArrowDown, Layers, Loader2, Terminal, FileText } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

// TaskGroup lifecycle states (from ADR-015)
const GROUP_STATES = [
  { slug: 'backlog', label: 'Task Builder', description: 'Task Builder reads Plans and creates tasks for each group' },
  { slug: 'analyzing', label: 'Analyzing', description: 'Group Evaluator is building the execution DAG' },
  { slug: 'ready', label: 'Ready', description: 'Waiting for project DAG builder to assign prereq evaluation slot' },
  { slug: 'prereq_eval', label: 'PreReq Eval', description: 'PreReq Evaluator is validating prerequisites' },
  { slug: 'executing', label: 'Executing', description: 'Tasks are being worked on' },
  { slug: 'done', label: 'Done', description: 'All tasks completed' },
] as const;

export interface TaskGroupTask {
  id: string;
  title: string;
  task_group_id: string | null;
  status?: string;
}

interface TaskGroupBoardProps {
  groups: TaskGroup[];
  selectedGroupId?: string | null;
  onSelectGroup: (groupId: string | null) => void;
  projectId: string;
  tasks?: TaskGroupTask[];
  onTransitionGroup?: (groupId: string, from: string, to: string) => void;
  onViewColumnAgent?: (columnSlug: 'backlog' | 'analyzing' | 'prereq_eval') => void;
  activeColumnAgent?: string | null; // column slug currently being viewed
  onViewGrouperAgent?: () => void; // called when user clicks View Agent in Task Builder column
  onViewArtifact?: (artifactId: string) => void;
}

interface GroupsByState {
  [key: string]: TaskGroup[];
}

interface ExecutionDAG {
  parallel_sets: string[][];
}

const STATUS_COLORS: Record<string, string> = {
  todo: 'bg-gray-100 dark:bg-gray-800 border-gray-300 dark:border-gray-600',
  inprogress: 'bg-blue-50 dark:bg-blue-950 border-blue-300 dark:border-blue-700',
  inreview: 'bg-purple-50 dark:bg-purple-950 border-purple-300 dark:border-purple-700',
  done: 'bg-green-50 dark:bg-green-950 border-green-300 dark:border-green-700',
  cancelled: 'bg-red-50 dark:bg-red-950 border-red-300 dark:border-red-700',
};

const STATUS_DOT_COLORS: Record<string, string> = {
  todo: 'bg-gray-400',
  inprogress: 'bg-blue-500',
  inreview: 'bg-purple-500',
  done: 'bg-green-500',
  cancelled: 'bg-red-500',
};

function parseDag(dagStr: string | null): ExecutionDAG | null {
  if (!dagStr) return null;
  try {
    const parsed = JSON.parse(dagStr);
    if (parsed?.parallel_sets && Array.isArray(parsed.parallel_sets)) {
      return parsed as ExecutionDAG;
    }
    return null;
  } catch {
    return null;
  }
}

// Helper to convert hex color to RGB with opacity
function hexToRgba(hex: string | null, opacity: number = 0.15) {
  if (!hex) return undefined;
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return undefined;
  const r = parseInt(result[1], 16);
  const g = parseInt(result[2], 16);
  const b = parseInt(result[3], 16);
  return `rgba(${r}, ${g}, ${b}, ${opacity})`;
}

/** DAG visualization — shows parallel sets as rows with connections */
function DagVisualization({
  dag,
  tasks,
  groupColor,
}: {
  dag: ExecutionDAG;
  tasks: TaskGroupTask[];
  groupColor: string | null;
}) {
  const taskMap = useMemo(() => {
    const map = new Map<string, TaskGroupTask>();
    for (const t of tasks) map.set(t.id, t);
    return map;
  }, [tasks]);

  return (
    <div className="flex flex-col items-center gap-0 py-2">
      {dag.parallel_sets.map((set, setIndex) => (
        <div key={setIndex} className="flex flex-col items-center w-full">
          {/* Arrow between sets */}
          {setIndex > 0 && (
            <div className="flex flex-col items-center py-1">
              <ArrowDown className="h-4 w-4 text-muted-foreground" />
            </div>
          )}

          {/* Set label */}
          <div className="flex items-center gap-1.5 mb-1.5">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              Set {setIndex + 1}
            </span>
            {set.length > 1 && (
              <Badge variant="outline" className="text-[9px] h-4 px-1">
                {set.length} parallel
              </Badge>
            )}
          </div>

          {/* Tasks in this set */}
          <div className="flex flex-wrap gap-2 justify-center w-full px-2">
            {set.map((taskId) => {
              const task = taskMap.get(taskId);
              const status = task?.status ?? 'todo';
              const colorClass = STATUS_COLORS[status] ?? STATUS_COLORS.todo;
              const dotClass = STATUS_DOT_COLORS[status] ?? STATUS_DOT_COLORS.todo;

              return (
                <div
                  key={taskId}
                  className={cn(
                    'flex items-center gap-2 px-3 py-2 rounded-md border text-sm max-w-[280px]',
                    colorClass
                  )}
                  style={{
                    borderLeftColor: groupColor || undefined,
                    borderLeftWidth: groupColor ? '3px' : undefined,
                  }}
                >
                  <div className={cn('h-2 w-2 rounded-full flex-shrink-0', dotClass)} />
                  <span className="truncate">
                    {task?.title ?? `Unknown task (${taskId.slice(0, 8)}...)`}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}

function TaskGroupBoardComponent({
  groups,
  selectedGroupId,
  tasks = [],
  onTransitionGroup,
  onViewColumnAgent,
  activeColumnAgent,
  onViewGrouperAgent,
  onViewArtifact,
}: TaskGroupBoardProps) {
  // undefined = modal closed, string = viewing specific group
  const [viewingGroupId, setViewingGroupId] = useState<string | undefined>(undefined);

  const viewingGroup = useMemo(
    () => groups.find(g => g.id === viewingGroupId),
    [groups, viewingGroupId]
  );

  const viewingDag = useMemo(
    () => parseDag(viewingGroup?.execution_dag ?? null),
    [viewingGroup?.execution_dag]
  );

  const groupTasks = useMemo(
    () => tasks.filter(t => t.task_group_id === viewingGroupId),
    [tasks, viewingGroupId]
  );

  // Group task groups by their state
  const groupsByState = useMemo((): GroupsByState => {
    const result: GroupsByState = {};

    for (const state of GROUP_STATES) {
      result[state.slug] = [];
    }

    for (const group of groups) {
      // Hide the backlog group — it's an internal container for the grouper agent
      if (group.is_backlog) continue;

      // Map 'draft' and 'pending' states to 'backlog' for display
      let state = group.state || 'backlog';
      if (state === 'draft' || state === 'pending') {
        state = 'backlog';
      }
      if (result[state]) {
        result[state].push(group);
      }
    }

    return result;
  }, [groups]);

  return (
    <div className="flex flex-col h-full bg-muted/20">
      {/* Kanban columns */}
      <div className="flex flex-1 gap-4 overflow-x-auto p-4">
        {GROUP_STATES.map((state) => (
        <div
          key={state.slug}
          className="flex flex-col min-w-[280px] max-w-[320px] flex-shrink-0 bg-muted/30 border rounded-lg overflow-hidden"
        >
          {/* Column Header */}
          <div className="bg-background border-b px-3 py-2 sticky top-0 z-10">
            <div className="flex items-center justify-between mb-1">
              <h3 className="font-semibold text-sm">{state.label}</h3>
              <Badge variant="secondary" className="text-xs">
                {groupsByState[state.slug].length}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground mb-2">
              {state.description}
            </p>
            {/* View Agent button — Task Builder column (always enabled; creates workspace on demand) */}
            {state.slug === 'backlog' && onViewGrouperAgent && (
              <Button
                size="sm"
                variant={activeColumnAgent === 'backlog' ? 'secondary' : 'outline'}
                className="w-full h-7 text-xs gap-1.5"
                onClick={onViewGrouperAgent}
                title="View Task Builder agent"
              >
                <Terminal className="h-3 w-3" />
                View Agent
              </Button>
            )}
            {/* View Agent button — Analyzing / PreReq Eval columns (always visible) */}
            {(state.slug === 'analyzing' || state.slug === 'prereq_eval') && onViewColumnAgent && (
              <Button
                size="sm"
                variant={activeColumnAgent === state.slug ? 'secondary' : 'outline'}
                className="w-full h-7 text-xs gap-1.5"
                onClick={() => onViewColumnAgent(state.slug)}
                title={`View ${state.label} agent`}
              >
                {activeColumnAgent === state.slug ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Terminal className="h-3 w-3" />
                )}
                View Agent
              </Button>
            )}
          </div>

          {/* Column Content */}
          <div className="flex-1 space-y-2 overflow-y-auto p-3">
            {/* Show task groups */}
            {groupsByState[state.slug].map((group) => {
              const dag = parseDag(group.execution_dag);
              const taskCount = tasks.filter(t => t.task_group_id === group.id).length;

              return (
                <Card
                  key={group.id}
                  className={cn(
                    'cursor-pointer hover:shadow-lg transition-all hover:scale-[1.02]',
                    selectedGroupId === group.id && 'ring-2 ring-primary'
                  )}
                  onClick={() => setViewingGroupId(group.id)}
                  style={{
                    backgroundColor: hexToRgba(group.color, 0.15),
                    borderColor: group.color || undefined,
                    borderWidth: group.color ? '2px' : '1px',
                  }}
                >
                  <CardHeader className="py-3 px-4">
                    <CardTitle className="text-sm font-medium line-clamp-2">
                      {group.name}
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="py-2 px-4">
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                      <span className="flex items-center gap-1">
                        {taskCount > 0 && (
                          <>{taskCount} task{taskCount !== 1 ? 's' : ''}</>
                        )}
                        {dag && (
                          <Layers className="h-3 w-3 ml-1 text-amber-500" />
                        )}
                        {group.artifact_id && (
                          <button
                            title="Has IMPL doc"
                            className="ml-1 hover:text-blue-500 transition-colors"
                            onClick={(e) => {
                              e.stopPropagation();
                              onViewArtifact?.(group.artifact_id!);
                            }}
                          >
                            <FileText className="h-3 w-3 text-blue-400" />
                          </button>
                        )}
                      </span>
                      {group.started_at && (
                        <span>
                          Started {new Date(group.started_at).toLocaleDateString()}
                        </span>
                      )}
                    </div>
                    {state.slug === 'backlog' && (group.state === 'draft' || !group.state) && onTransitionGroup && (
                      <Button
                        size="sm"
                        variant="outline"
                        className="w-full mt-2 text-xs h-7"
                        onClick={(e) => {
                          e.stopPropagation();
                          onTransitionGroup(group.id, 'draft', 'analyzing');
                        }}
                      >
                        <FlaskConical className="h-3 w-3 mr-1" />
                        Analyze
                      </Button>
                    )}
                  </CardContent>
                </Card>
              );
            })}

            {/* Empty state */}
            {groupsByState[state.slug].length === 0 && (
              <div className="text-center py-8 text-sm text-muted-foreground">
                No groups
              </div>
            )}
          </div>
        </div>
      ))}
      </div>

      {/* Task Group Modal */}
      <Dialog open={viewingGroupId !== undefined} onOpenChange={(open) => !open && setViewingGroupId(undefined)}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              {viewingGroup?.color && (
                <div
                  className="h-3 w-3 rounded-full flex-shrink-0"
                  style={{ backgroundColor: viewingGroup.color }}
                />
              )}
              {viewingGroup?.name || 'Group Tasks'}
              {viewingGroup?.state && (
                <Badge variant="outline" className="text-xs ml-1">
                  {viewingGroup.state}
                </Badge>
              )}
            </DialogTitle>
          </DialogHeader>

          {/* DAG Visualization */}
          {viewingDag && viewingDag.parallel_sets.length > 0 ? (
            <div className="mt-4">
              <div className="flex items-center gap-2 mb-3">
                <Layers className="h-4 w-4 text-amber-500" />
                <span className="text-sm font-medium">Execution Plan</span>
                <span className="text-xs text-muted-foreground">
                  {viewingDag.parallel_sets.length} set{viewingDag.parallel_sets.length !== 1 ? 's' : ''}
                </span>
              </div>
              <div className="rounded-lg border bg-muted/20 p-4">
                <DagVisualization
                  dag={viewingDag}
                  tasks={groupTasks}
                  groupColor={viewingGroup?.color ?? null}
                />
              </div>
            </div>
          ) : (
            /* Flat task list fallback */
            <div className="space-y-2 mt-4">
              {groupTasks.map(task => (
                <div
                  key={task.id}
                  className="p-3 rounded-lg border bg-card hover:bg-accent cursor-pointer transition-colors"
                >
                  <div className="flex items-center gap-2">
                    {task.status && (
                      <div className={cn(
                        'h-2 w-2 rounded-full flex-shrink-0',
                        STATUS_DOT_COLORS[task.status] ?? STATUS_DOT_COLORS.todo
                      )} />
                    )}
                    <div className="font-medium text-sm">{task.title}</div>
                  </div>
                </div>
              ))}
              {groupTasks.length === 0 && (
                <div className="text-center py-8 text-sm text-muted-foreground">
                  No tasks in this group
                </div>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

export const TaskGroupBoard = memo(TaskGroupBoardComponent);
