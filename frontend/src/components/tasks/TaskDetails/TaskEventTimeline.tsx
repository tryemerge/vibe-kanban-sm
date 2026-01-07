import { useMemo } from 'react';
import {
  ArrowRight,
  Play,
  CheckCircle,
  XCircle,
  GitCommit,
  User,
  Bot,
  Settings,
  Plus,
  RefreshCw,
  AlertCircle,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { useTaskEvents } from '@/hooks';
import { cn } from '@/lib/utils';
import type { TaskEventWithNames, TaskEventType, ActorType } from 'shared/types';

interface TaskEventTimelineProps {
  taskId: string;
  className?: string;
}

const EVENT_CONFIG: Record<
  TaskEventType,
  { icon: typeof ArrowRight; label: string; color: string }
> = {
  column_enter: {
    icon: ArrowRight,
    label: 'Moved to column',
    color: 'bg-blue-500/10 text-blue-500 border-blue-500/20',
  },
  column_exit: {
    icon: ArrowRight,
    label: 'Left column',
    color: 'bg-slate-500/10 text-slate-500 border-slate-500/20',
  },
  agent_start: {
    icon: Play,
    label: 'Agent started',
    color: 'bg-amber-500/10 text-amber-500 border-amber-500/20',
  },
  agent_complete: {
    icon: CheckCircle,
    label: 'Agent completed',
    color: 'bg-green-500/10 text-green-500 border-green-500/20',
  },
  agent_failed: {
    icon: XCircle,
    label: 'Agent failed',
    color: 'bg-red-500/10 text-red-500 border-red-500/20',
  },
  commit: {
    icon: GitCommit,
    label: 'Commit made',
    color: 'bg-purple-500/10 text-purple-500 border-purple-500/20',
  },
  manual_action: {
    icon: User,
    label: 'Manual action',
    color: 'bg-indigo-500/10 text-indigo-500 border-indigo-500/20',
  },
  task_created: {
    icon: Plus,
    label: 'Task created',
    color: 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20',
  },
  status_change: {
    icon: RefreshCw,
    label: 'Status changed',
    color: 'bg-cyan-500/10 text-cyan-500 border-cyan-500/20',
  },
};

const ACTOR_ICONS: Record<ActorType, typeof User> = {
  user: User,
  agent: Bot,
  system: Settings,
};

function formatTimeAgo(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSeconds = Math.floor(diffMs / 1000);
  const diffMinutes = Math.floor(diffSeconds / 60);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffSeconds < 60) return 'just now';
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;

  return date.toLocaleDateString();
}

function EventItem({ event }: { event: TaskEventWithNames }) {
  const config = EVENT_CONFIG[event.event_type];
  const Icon = config.icon;
  const ActorIcon = ACTOR_ICONS[event.actor_type];
  const createdAt = new Date(event.created_at);

  const description = useMemo(() => {
    switch (event.event_type) {
      case 'column_enter':
        if (event.from_column_name && event.to_column_name) {
          return `${event.from_column_name} → ${event.to_column_name}`;
        } else if (event.to_column_name) {
          return `Entered ${event.to_column_name}`;
        }
        return config.label;
      case 'column_exit':
        return event.from_column_name
          ? `Exited ${event.from_column_name}`
          : config.label;
      case 'agent_start':
        return event.executor ? `${config.label}: ${event.executor}` : config.label;
      case 'commit':
        if (event.commit_hash && event.commit_message) {
          const shortHash = event.commit_hash.substring(0, 7);
          return `${shortHash}: ${event.commit_message}`;
        }
        return config.label;
      default:
        return config.label;
    }
  }, [event, config.label]);

  return (
    <div className="flex items-start gap-3 py-3 px-2 hover:bg-muted/50 rounded-md transition-colors">
      <div
        className={cn(
          'flex items-center justify-center w-8 h-8 rounded-full border shrink-0',
          config.color
        )}
      >
        <Icon className="h-4 w-4" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-medium text-sm">{description}</span>
          {event.trigger_type && (
            <Badge variant="outline" className="text-xs capitalize">
              {event.trigger_type}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
          <ActorIcon className="h-3 w-3" />
          <span className="capitalize">{event.actor_type}</span>
          <span className="opacity-50">·</span>
          <time dateTime={createdAt.toISOString()} title={createdAt.toLocaleString()}>
            {formatTimeAgo(createdAt)}
          </time>
        </div>
      </div>
    </div>
  );
}

export function TaskEventTimeline({ taskId, className }: TaskEventTimelineProps) {
  const { data: events, isLoading, error } = useTaskEvents(taskId);

  if (isLoading) {
    return (
      <div className={cn('p-4', className)}>
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="flex items-start gap-3">
              <div className="w-8 h-8 rounded-full bg-muted" />
              <div className="flex-1 space-y-2">
                <div className="h-4 bg-muted rounded w-3/4" />
                <div className="h-3 bg-muted rounded w-1/2" />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={cn('p-4 text-center', className)}>
        <AlertCircle className="h-8 w-8 mx-auto text-destructive mb-2" />
        <p className="text-sm text-destructive">Failed to load events</p>
      </div>
    );
  }

  if (!events || events.length === 0) {
    return (
      <div className={cn('p-4 text-center text-muted-foreground', className)}>
        <p className="text-sm">No events recorded yet</p>
      </div>
    );
  }

  return (
    <div className={cn('divide-y divide-border/50', className)}>
      {events.map((event) => (
        <EventItem key={event.id} event={event} />
      ))}
    </div>
  );
}

export default TaskEventTimeline;
