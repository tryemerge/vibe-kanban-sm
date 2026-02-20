import { memo, useMemo, useState } from 'react';
import { TaskGroup } from 'shared/types';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Sparkles, Loader2 } from 'lucide-react';
import { taskGroupsApi } from '@/lib/api';

// TaskGroup lifecycle states (from ADR-015)
const GROUP_STATES = [
  { slug: 'backlog', label: 'Backlog', description: 'Ungrouped tasks awaiting analysis' },
  { slug: 'analyzing', label: 'Analyzing', description: 'Agent is analyzing and organizing' },
  { slug: 'ready', label: 'Ready', description: 'Analyzed, waiting for dependencies' },
  { slug: 'executing', label: 'Executing', description: 'Tasks are being worked on' },
  { slug: 'done', label: 'Done', description: 'All tasks completed' },
] as const;

interface TaskGroupBoardProps {
  groups: TaskGroup[];
  selectedGroupId?: string | null;
  onSelectGroup: (groupId: string | null) => void;
  ungroupedTaskCount?: number;
  projectId: string;
}

interface GroupsByState {
  [key: string]: TaskGroup[];
}

function TaskGroupBoardComponent({
  groups,
  selectedGroupId,
  onSelectGroup,
  ungroupedTaskCount = 0,
  projectId,
}: TaskGroupBoardProps) {
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Group task groups by their state
  const groupsByState = useMemo((): GroupsByState => {
    const result: GroupsByState = {};

    for (const state of GROUP_STATES) {
      result[state.slug] = [];
    }

    for (const group of groups) {
      const state = group.state || 'backlog';
      if (result[state]) {
        result[state].push(group);
      }
    }

    return result;
  }, [groups]);

  const handleAnalyzeBacklog = async () => {
    setIsAnalyzing(true);
    setStatusMessage(null);
    try {
      const result = await taskGroupsApi.analyzeBacklog(projectId);
      setStatusMessage(result.message);
      console.log('Grouping analysis requested:', result);

      // Clear message after 5 seconds
      setTimeout(() => setStatusMessage(null), 5000);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : 'Failed to analyze backlog';
      setStatusMessage(`Error: ${errorMsg}`);
      console.error('Grouping analysis failed:', error);

      // Clear error after 5 seconds
      setTimeout(() => setStatusMessage(null), 5000);
    } finally {
      setIsAnalyzing(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-muted/20">
      {/* Header with Group Tasks button */}
      {ungroupedTaskCount >= 2 && (
        <div className="shrink-0 flex items-center justify-between px-4 py-2 border-b bg-background/50">
          {statusMessage && (
            <span className="text-sm text-muted-foreground">
              {statusMessage}
            </span>
          )}
          <div className="flex-1" />
          <Button
            variant="default"
            size="sm"
            onClick={handleAnalyzeBacklog}
            disabled={isAnalyzing}
            className="gap-2"
          >
            {isAnalyzing ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Sparkles className="h-4 w-4" />
            )}
            Group Tasks
          </Button>
        </div>
      )}

      {/* Kanban columns */}
      <div className="flex flex-1 gap-4 overflow-x-auto p-4">
        {GROUP_STATES.map((state) => (
        <div
          key={state.slug}
          className="flex flex-col min-w-[280px] max-w-[320px] flex-shrink-0"
        >
          {/* Column Header */}
          <div className="mb-3 sticky top-0 z-10">
            <div className="bg-card rounded-lg border px-4 py-3 shadow-sm">
              <h3 className="font-semibold text-sm flex items-center justify-between">
                <span>{state.label}</span>
                <Badge variant="secondary" className="ml-2 text-xs">
                  {state.slug === 'backlog'
                    ? ungroupedTaskCount + groupsByState[state.slug].length
                    : groupsByState[state.slug].length}
                </Badge>
              </h3>
              <p className="text-xs text-muted-foreground mt-1">
                {state.description}
              </p>
            </div>
          </div>

          {/* Column Content */}
          <div className="flex-1 space-y-2 overflow-y-auto">
            {/* Show ungrouped tasks indicator in backlog */}
            {state.slug === 'backlog' && ungroupedTaskCount > 0 && (
              <Card
                className={cn(
                  'cursor-pointer hover:shadow-md transition-shadow border-dashed',
                  selectedGroupId === null && 'ring-2 ring-primary'
                )}
                onClick={() => onSelectGroup(null)}
              >
                <CardHeader className="py-3 px-4">
                  <CardTitle className="text-sm font-medium">
                    Ungrouped Tasks
                  </CardTitle>
                </CardHeader>
                <CardContent className="py-2 px-4">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span>{ungroupedTaskCount} tasks</span>
                    <span>Awaiting grouping</span>
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Show task groups */}
            {groupsByState[state.slug].map((group) => (
              <Card
                key={group.id}
                className={cn(
                  'cursor-pointer hover:shadow-md transition-shadow',
                  selectedGroupId === group.id && 'ring-2 ring-primary'
                )}
                onClick={() => onSelectGroup(group.id)}
                style={{
                  borderLeftColor: group.color || undefined,
                  borderLeftWidth: group.color ? '4px' : undefined,
                }}
              >
                <CardHeader className="py-3 px-4">
                  <CardTitle className="text-sm font-medium line-clamp-2">
                    {group.name}
                  </CardTitle>
                </CardHeader>
                <CardContent className="py-2 px-4">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span>
                      {/* Task count would come from API or parent */}
                      Group
                    </span>
                    {group.started_at && (
                      <span>
                        Started {new Date(group.started_at).toLocaleDateString()}
                      </span>
                    )}
                  </div>
                </CardContent>
              </Card>
            ))}

            {/* Empty state */}
            {groupsByState[state.slug].length === 0 &&
              !(state.slug === 'backlog' && ungroupedTaskCount > 0) && (
                <div className="text-center py-8 text-sm text-muted-foreground">
                  No groups
                </div>
              )}
          </div>
        </div>
      ))}
      </div>
    </div>
  );
}

export const TaskGroupBoard = memo(TaskGroupBoardComponent);
