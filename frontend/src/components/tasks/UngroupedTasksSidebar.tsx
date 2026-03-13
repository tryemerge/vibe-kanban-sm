import { memo, useState } from 'react';
import { Card, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Sparkles, Loader2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';

interface UngroupedTask {
  id: string;
  title: string;
}

interface UngroupedTasksSidebarProps {
  tasks: UngroupedTask[];
  onViewTask: (taskId: string) => void;
  onGroupTasks?: (prompt?: string) => void;
  isGrouping?: boolean;
  selectedTaskId?: string;
}

function UngroupedTasksSidebarComponent({
  tasks,
  onViewTask,
  onGroupTasks,
  isGrouping = false,
  selectedTaskId,
}: UngroupedTasksSidebarProps) {
  const [prompt, setPrompt] = useState('');

  return (
    <div className="flex flex-col h-full bg-muted/20 border-r">
      {/* Header */}
      <div className="shrink-0 bg-background border-b px-4 py-3">
        <div className="flex items-center justify-between mb-2">
          <h2 className="font-semibold text-sm flex items-center gap-2">
            Ungrouped Tasks
            <Badge variant="secondary" className="text-xs">
              {tasks.length}
            </Badge>
          </h2>
        </div>
        <p className="text-xs text-muted-foreground mb-3">
          Tasks awaiting organization
        </p>
        {tasks.length >= 2 && onGroupTasks && (
          <div className="space-y-2">
            <Textarea
              placeholder="Optional: guide how tasks should be grouped..."
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              disabled={isGrouping}
              className="text-xs min-h-[60px] resize-none"
              rows={2}
            />
            <Button
              variant="default"
              size="sm"
              onClick={() => onGroupTasks(prompt || undefined)}
              disabled={isGrouping}
              className="w-full gap-2"
            >
              {isGrouping ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Sparkles className="h-4 w-4" />
              )}
              Group Tasks
            </Button>
          </div>
        )}
      </div>

      {/* Task List */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {tasks.length === 0 ? (
          <div className="text-center py-8 text-sm text-muted-foreground">
            All tasks are grouped
          </div>
        ) : (
          tasks.map((task) => (
            <Card
              key={task.id}
              className={`cursor-pointer hover:shadow-md transition-all hover:scale-[1.01] ${
                selectedTaskId === task.id ? 'ring-2 ring-primary' : ''
              }`}
              onClick={() => onViewTask(task.id)}
            >
              <CardHeader className="py-2 px-3">
                <CardTitle className="text-xs font-medium line-clamp-2">
                  {task.title}
                </CardTitle>
              </CardHeader>
            </Card>
          ))
        )}
      </div>
    </div>
  );
}

export const UngroupedTasksSidebar = memo(UngroupedTasksSidebarComponent);
