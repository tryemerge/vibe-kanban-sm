import { useState } from 'react';
import type { TaskWithAttemptStatus } from 'shared/types';
import type { WorkspaceWithSession } from '@/types/attempt';
import VirtualizedList from '@/components/logs/VirtualizedList';
import { TaskFollowUpSection } from '@/components/tasks/TaskFollowUpSection';
import { EntriesProvider } from '@/contexts/EntriesContext';
import { RetryUiProvider } from '@/contexts/RetryUiContext';
import { TaskEventTimeline } from '@/components/tasks/TaskDetails/TaskEventTimeline';
import { TaskContextPanel } from '@/components/tasks/TaskDetails/TaskContextPanel';
import { Button } from '@/components/ui/button';
import { Clock, Brain } from 'lucide-react';
import type { ReactNode } from 'react';

interface TaskAttemptPanelProps {
  attempt: WorkspaceWithSession | undefined;
  task: TaskWithAttemptStatus | null;
  projectId?: string;
  children: (sections: { logs: ReactNode; followUp: ReactNode }) => ReactNode;
}

const TaskAttemptPanel = ({
  attempt,
  task,
  projectId,
  children,
}: TaskAttemptPanelProps) => {
  const [activeTab, setActiveTab] = useState<'logs' | 'timeline' | 'context'>('logs');

  if (!attempt) {
    return <div className="p-6 text-muted-foreground">Loading attempt...</div>;
  }

  if (!task) {
    return <div className="p-6 text-muted-foreground">Loading task...</div>;
  }

  const tabBar = (
    <div className="flex gap-1 px-4 py-2 border-b bg-background/50">
      <Button
        variant={activeTab === 'logs' ? 'secondary' : 'ghost'}
        size="sm"
        onClick={() => setActiveTab('logs')}
      >
        Logs
      </Button>
      <Button
        variant={activeTab === 'timeline' ? 'secondary' : 'ghost'}
        size="sm"
        onClick={() => setActiveTab('timeline')}
      >
        <Clock className="mr-1.5 h-3.5 w-3.5" />
        Timeline
      </Button>
      {projectId && (
        <Button
          variant={activeTab === 'context' ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => setActiveTab('context')}
        >
          <Brain className="mr-1.5 h-3.5 w-3.5" />
          Context
        </Button>
      )}
    </div>
  );

  if (activeTab === 'timeline') {
    return (
      <div className="flex flex-col h-full min-h-0">
        {tabBar}
        <div className="flex-1 min-h-0 overflow-y-auto">
          <TaskEventTimeline taskId={task.id} workspaceId={attempt.id} />
        </div>
      </div>
    );
  }

  if (activeTab === 'context' && projectId) {
    return (
      <div className="flex flex-col h-full min-h-0">
        {tabBar}
        <div className="flex-1 min-h-0 overflow-y-auto">
          <TaskContextPanel taskId={task.id} projectId={projectId} />
        </div>
      </div>
    );
  }

  return (
    <EntriesProvider key={attempt.id}>
      <RetryUiProvider attemptId={attempt.id}>
        <div className="flex flex-col h-full min-h-0">
          {tabBar}
          <div className="flex-1 min-h-0 flex flex-col">
            {children({
              logs: (
                <VirtualizedList key={attempt.id} attempt={attempt} task={task} />
              ),
              followUp: (
                <TaskFollowUpSection task={task} session={attempt.session} />
              ),
            })}
          </div>
        </div>
      </RetryUiProvider>
    </EntriesProvider>
  );
};

export default TaskAttemptPanel;
