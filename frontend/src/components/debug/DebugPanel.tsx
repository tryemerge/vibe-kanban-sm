import { useState, useEffect, useCallback, useRef } from 'react';
import { ChevronDown, ChevronUp, X, Bug, RefreshCw, Circle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useProject } from '@/contexts/ProjectContext';
import { useProjectColumns } from '@/hooks';
import { cn } from '@/lib/utils';

interface DebugEvent {
  id: string;
  timestamp: string;
  type: string;
  // Event-specific fields
  task_id?: string;
  task_title?: string;
  from_column?: string | null;
  to_column?: string;
  column_has_agent?: boolean;
  agent_name?: string | null;
  workspace_id?: string;
  branch?: string;
  reusing_existing?: boolean;
  executor?: string;
  system_prompt_length?: number;
  system_prompt_preview?: string;
  start_command_length?: number | null;
  start_command_preview?: string | null;
  column_name?: string;
  full_prompt_length?: number;
  full_prompt?: string;
  session_id?: string;
  commit_hash?: string;
  commit_message?: string;
  success?: boolean;
  decision?: unknown;
  reason?: string;
  message?: string;
  context?: unknown;
}

export function DebugPanel() {
  const [isOpen, setIsOpen] = useState(false);
  const [isMinimized, setIsMinimized] = useState(false);
  const [events, setEvents] = useState<DebugEvent[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [activeTab, setActiveTab] = useState<'events' | 'columns' | 'state'>(
    'events'
  );
  const [panelHeight, setPanelHeight] = useState(288); // Default h-72 = 288px
  const [isResizing, setIsResizing] = useState(false);
  const { projectId } = useProject();
  const { data: columns = [], refetch: refetchColumns } =
    useProjectColumns(projectId);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Handle resize drag
  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newHeight = window.innerHeight - e.clientY;
      setPanelHeight(Math.max(100, Math.min(newHeight, window.innerHeight - 100)));
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing]);

  // Set CSS variable for debug panel height so content can add bottom padding
  useEffect(() => {
    const root = document.documentElement;
    if (isOpen && !isMinimized) {
      root.style.setProperty('--debug-panel-height', `${panelHeight}px`);
    } else if (isOpen && isMinimized) {
      root.style.setProperty('--debug-panel-height', '40px');
    } else {
      root.style.setProperty('--debug-panel-height', '0px');
    }
    return () => {
      root.style.setProperty('--debug-panel-height', '0px');
    };
  }, [isOpen, isMinimized, panelHeight]);

  // Connect to debug WebSocket
  useEffect(() => {
    if (!isOpen) return;

    const connect = () => {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/api/debug/events/ws`;

      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        console.log('[DebugPanel] Connected to debug events');
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as DebugEvent;
          setEvents((prev) => [data, ...prev].slice(0, 200));
        } catch (e) {
          console.error('[DebugPanel] Failed to parse event:', e);
        }
      };

      ws.onclose = () => {
        setIsConnected(false);
        console.log('[DebugPanel] Disconnected, reconnecting in 2s...');
        reconnectTimeoutRef.current = setTimeout(connect, 2000);
      };

      ws.onerror = (err) => {
        console.error('[DebugPanel] WebSocket error:', err);
      };
    };

    connect();

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [isOpen]);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  // Don't render in production
  if (import.meta.env.PROD) {
    return null;
  }

  if (!isOpen) {
    return (
      <Button
        variant="outline"
        size="sm"
        className="fixed bottom-4 left-4 z-50 gap-2 bg-background shadow-lg"
        onClick={() => setIsOpen(true)}
      >
        <Bug className="h-4 w-4" />
        Debug
      </Button>
    );
  }

  return (
    <div
      className={cn(
        'fixed bottom-0 left-0 right-0 z-50 bg-background border-t shadow-lg',
        isResizing && 'select-none'
      )}
      style={{ height: isMinimized ? 40 : panelHeight }}
    >
      {/* Resize handle */}
      {!isMinimized && (
        <div
          className="absolute top-0 left-0 right-0 h-1.5 cursor-ns-resize hover:bg-primary/20 active:bg-primary/30 transition-colors"
          onMouseDown={(e) => {
            e.preventDefault();
            setIsResizing(true);
          }}
        >
          <div className="absolute left-1/2 -translate-x-1/2 top-0.5 w-8 h-0.5 bg-muted-foreground/30 rounded-full" />
        </div>
      )}
      {/* Header */}
      <div className="flex items-center justify-between px-4 h-10 border-b bg-muted/50">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 font-mono text-sm font-medium">
            <Bug className="h-4 w-4" />
            Debug Panel
            <Circle
              className={cn(
                'h-2 w-2',
                isConnected ? 'fill-green-500 text-green-500' : 'fill-red-500 text-red-500'
              )}
            />
          </div>
          <div className="flex gap-1">
            <Button
              variant={activeTab === 'events' ? 'secondary' : 'ghost'}
              size="sm"
              className="h-6 text-xs"
              onClick={() => setActiveTab('events')}
            >
              Events ({events.length})
            </Button>
            <Button
              variant={activeTab === 'columns' ? 'secondary' : 'ghost'}
              size="sm"
              className="h-6 text-xs"
              onClick={() => setActiveTab('columns')}
            >
              Columns
            </Button>
            <Button
              variant={activeTab === 'state' ? 'secondary' : 'ghost'}
              size="sm"
              className="h-6 text-xs"
              onClick={() => setActiveTab('state')}
            >
              State
            </Button>
          </div>
        </div>
        <div className="flex items-center gap-1">
          {activeTab === 'columns' && (
            <Button
              variant="ghost"
              size="sm"
              className="h-6 w-6 p-0"
              onClick={() => refetchColumns()}
            >
              <RefreshCw className="h-3 w-3" />
            </Button>
          )}
          {activeTab === 'events' && (
            <Button
              variant="ghost"
              size="sm"
              className="h-6 text-xs"
              onClick={clearEvents}
            >
              Clear
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            onClick={() => setIsMinimized(!isMinimized)}
          >
            {isMinimized ? (
              <ChevronUp className="h-4 w-4" />
            ) : (
              <ChevronDown className="h-4 w-4" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            onClick={() => setIsOpen(false)}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Content */}
      {!isMinimized && (
        <div className="h-[calc(100%-2.5rem)] overflow-auto p-2 font-mono text-xs">
          {activeTab === 'events' && <EventsTab events={events} />}
          {activeTab === 'columns' && (
            <ColumnsTab columns={columns} projectId={projectId} />
          )}
          {activeTab === 'state' && <StateTab />}
        </div>
      )}
    </div>
  );
}

function EventsTab({ events }: { events: DebugEvent[] }) {
  if (events.length === 0) {
    return (
      <div className="text-muted-foreground p-4 text-center">
        Waiting for events... Click "Start" on a task to see workflow events
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {events.map((event) => (
        <EventRow key={event.id} event={event} />
      ))}
    </div>
  );
}

function EventRow({ event }: { event: DebugEvent }) {
  const time = new Date(event.timestamp).toLocaleTimeString();

  const getEventColor = (type: string) => {
    if (type === 'error') return 'text-red-600 bg-red-50 dark:bg-red-950';
    if (type === 'warn') return 'text-yellow-600 bg-yellow-50 dark:bg-yellow-950';
    if (type === 'task_column_changed') return 'text-blue-600 bg-blue-50 dark:bg-blue-950';
    if (type === 'attempt_created') return 'text-purple-600 bg-purple-50 dark:bg-purple-950';
    if (type === 'agent_starting') return 'text-green-600 bg-green-50 dark:bg-green-950';
    if (type === 'full_prompt_built') return 'text-emerald-600 bg-emerald-50 dark:bg-emerald-950';
    if (type === 'agent_started') return 'text-green-600';
    if (type === 'agent_completed') return 'text-cyan-600 bg-cyan-50 dark:bg-cyan-950';
    if (type === 'commit_made') return 'text-orange-600 bg-orange-50 dark:bg-orange-950';
    if (type === 'decision_file_read') return 'text-pink-600 bg-pink-50 dark:bg-pink-950';
    if (type === 'auto_transition') return 'text-indigo-600 bg-indigo-50 dark:bg-indigo-950';
    return 'text-muted-foreground';
  };

  const renderEventContent = () => {
    switch (event.type) {
      case 'task_column_changed':
        return (
          <>
            <span className="font-medium">{event.task_title}</span>
            <span className="text-muted-foreground"> moved </span>
            {event.from_column && (
              <>
                <span className="text-muted-foreground">from </span>
                <span className="font-medium">{event.from_column}</span>
                <span className="text-muted-foreground"> </span>
              </>
            )}
            <span className="text-muted-foreground">to </span>
            <span className="font-medium">{event.to_column}</span>
            {event.column_has_agent ? (
              <span className="ml-2 text-green-600">
                [Agent: {event.agent_name}]
              </span>
            ) : (
              <span className="ml-2 text-yellow-600">[No Agent]</span>
            )}
          </>
        );

      case 'attempt_created':
        return (
          <>
            <span className="font-medium">
              {event.reusing_existing ? 'Reusing' : 'Created'} workspace
            </span>
            <span className="text-muted-foreground ml-2">
              branch: <span className="font-medium">{event.branch}</span>
            </span>
            <span className="text-muted-foreground ml-2 opacity-60">
              {event.workspace_id?.slice(0, 8)}...
            </span>
          </>
        );

      case 'agent_starting':
        return (
          <div className="flex flex-col gap-1">
            <div>
              <span className="font-medium">{event.agent_name}</span>
              <span className="text-muted-foreground"> starting in </span>
              <span className="font-medium">{event.column_name}</span>
              <span className="text-muted-foreground ml-2">
                executor: {event.executor}
              </span>
            </div>
            <div className="text-[10px] bg-muted/50 p-1 rounded max-w-full overflow-hidden">
              <div className="text-muted-foreground">
                System prompt ({event.system_prompt_length} chars):
              </div>
              <div className="text-foreground/80 whitespace-pre-wrap break-all">
                {event.system_prompt_preview || '(empty)'}
                {(event.system_prompt_length ?? 0) > 200 && '...'}
              </div>
            </div>
            {event.start_command_preview && (
              <div className="text-[10px] bg-muted/50 p-1 rounded max-w-full overflow-hidden">
                <div className="text-muted-foreground">
                  Start command ({event.start_command_length} chars):
                </div>
                <div className="text-foreground/80 whitespace-pre-wrap break-all">
                  {event.start_command_preview}
                  {(event.start_command_length ?? 0) > 200 && '...'}
                </div>
              </div>
            )}
          </div>
        );

      case 'full_prompt_built':
        return (
          <div className="flex flex-col gap-1">
            <div>
              <span className="font-medium">Full Prompt Built</span>
              <span className="text-muted-foreground ml-2">
                for {event.agent_name} ({event.full_prompt_length} chars)
              </span>
            </div>
            <div className="text-[10px] bg-muted/50 p-2 rounded max-h-48 overflow-y-auto whitespace-pre-wrap break-all font-mono">
              {event.full_prompt || '(empty)'}
            </div>
          </div>
        );

      case 'agent_started':
        return (
          <>
            <span className="font-medium">Agent execution started</span>
            <span className="text-muted-foreground ml-2 opacity-60">
              session: {event.session_id?.slice(0, 8)}...
            </span>
          </>
        );

      case 'agent_completed':
        return (
          <>
            <span className="font-medium">Agent execution completed</span>
            <span className={cn('ml-2', event.success ? 'text-green-600' : 'text-red-600')}>
              {event.success ? 'SUCCESS' : 'FAILED'}
            </span>
          </>
        );

      case 'commit_made':
        return (
          <>
            <span className="font-medium">Commit:</span>
            <span className="text-muted-foreground ml-2">
              {event.commit_hash?.slice(0, 7)}
            </span>
            <span className="ml-2">{event.commit_message}</span>
          </>
        );

      case 'decision_file_read':
        return (
          <>
            <span className="font-medium">Decision file found:</span>
            <span className="ml-2">{JSON.stringify(event.decision)}</span>
          </>
        );

      case 'auto_transition':
        return (
          <>
            <span className="font-medium">Auto-transition:</span>
            <span className="text-muted-foreground ml-2">
              {event.from_column} → {event.to_column}
            </span>
            <span className="text-muted-foreground ml-2">
              ({event.reason})
            </span>
          </>
        );

      case 'info':
        return <span>{event.message}</span>;

      case 'warn':
        return <span>{event.message}</span>;

      case 'error':
        return (
          <>
            <span className="font-medium">Error:</span>
            <span className="ml-2">{event.message}</span>
          </>
        );

      default:
        return (
          <span className="text-muted-foreground">
            {event.type}: {JSON.stringify(event).slice(0, 100)}...
          </span>
        );
    }
  };

  return (
    <div
      className={cn(
        'flex gap-2 py-1 px-2 rounded border-l-2',
        getEventColor(event.type)
      )}
    >
      <span className="text-muted-foreground shrink-0 w-20">{time}</span>
      <span className="shrink-0 w-32 font-medium opacity-70">
        {event.type.replace(/_/g, ' ')}
      </span>
      <span className="flex-1 truncate">{renderEventContent()}</span>
    </div>
  );
}

interface ColumnInfo {
  id: string;
  name: string;
  slug: string;
  position: number;
  is_initial: boolean;
  is_terminal: boolean;
  starts_workflow: boolean;
  status: string;
  agent_id: string | null;
}

function ColumnsTab({
  columns,
  projectId,
}: {
  columns: ColumnInfo[];
  projectId: string | undefined;
}) {
  if (!projectId) {
    return (
      <div className="text-muted-foreground">No project selected</div>
    );
  }

  if (columns.length === 0) {
    return <div className="text-muted-foreground">No columns found</div>;
  }

  return (
    <div className="space-y-1">
      <div className="text-muted-foreground mb-2">
        Project: {projectId} | {columns.length} columns
      </div>
      <table className="w-full text-left">
        <thead>
          <tr className="border-b text-muted-foreground">
            <th className="py-1 pr-4">Pos</th>
            <th className="py-1 pr-4">Name</th>
            <th className="py-1 pr-4">Status</th>
            <th className="py-1 pr-4">Flags</th>
            <th className="py-1 pr-4">Agent ID</th>
          </tr>
        </thead>
        <tbody>
          {columns
            .sort((a, b) => a.position - b.position)
            .map((col) => (
              <tr key={col.id} className="border-b border-border/50">
                <td className="py-1 pr-4">{col.position}</td>
                <td className="py-1 pr-4 font-medium">{col.name}</td>
                <td className="py-1 pr-4">
                  <span
                    className={cn(
                      'px-1.5 py-0.5 rounded text-[10px]',
                      col.status === 'todo' && 'bg-blue-100 text-blue-700',
                      col.status === 'in_progress' &&
                        'bg-yellow-100 text-yellow-700',
                      col.status === 'done' && 'bg-green-100 text-green-700',
                      col.status === 'cancelled' && 'bg-gray-100 text-gray-700'
                    )}
                  >
                    {col.status}
                  </span>
                </td>
                <td className="py-1 pr-4">
                  <div className="flex gap-1">
                    {col.is_initial && (
                      <span className="bg-purple-100 text-purple-700 px-1 rounded text-[10px]">
                        initial
                      </span>
                    )}
                    {col.is_terminal && (
                      <span className="bg-red-100 text-red-700 px-1 rounded text-[10px]">
                        terminal
                      </span>
                    )}
                    {col.starts_workflow && (
                      <span className="bg-green-100 text-green-700 px-1 rounded text-[10px]">
                        workflow
                      </span>
                    )}
                  </div>
                </td>
                <td className="py-1 pr-4">
                  {col.agent_id ? (
                    <span className="text-green-600 font-medium">
                      {col.agent_id.slice(0, 8)}...
                    </span>
                  ) : (
                    <span className="text-muted-foreground">none</span>
                  )}
                </td>
              </tr>
            ))}
        </tbody>
      </table>
    </div>
  );
}

function StateTab() {
  const { projectId } = useProject();
  const { data: columns = [] } = useProjectColumns(projectId);

  const workflowColumn = columns.find((c) => c.starts_workflow);
  const initialColumn = columns.find((c) => c.is_initial);
  const terminalColumns = columns.filter((c) => c.is_terminal);

  return (
    <div className="space-y-4">
      <div>
        <div className="text-muted-foreground mb-1">Workflow Summary</div>
        <div className="grid grid-cols-2 gap-2">
          <div className="bg-muted/50 p-2 rounded">
            <div className="text-muted-foreground text-[10px]">
              Initial (Backlog)
            </div>
            <div className="font-medium">
              {initialColumn?.name || 'Not set'}
            </div>
            {initialColumn?.agent_id && (
              <div className="text-green-600 text-[10px]">
                Has agent: {initialColumn.agent_id.slice(0, 8)}...
              </div>
            )}
          </div>
          <div className="bg-muted/50 p-2 rounded">
            <div className="text-muted-foreground text-[10px]">
              Starts Workflow
            </div>
            <div className="font-medium">
              {workflowColumn?.name || 'Not set'}
            </div>
            {workflowColumn?.agent_id ? (
              <div className="text-green-600 text-[10px]">
                Has agent: {workflowColumn.agent_id.slice(0, 8)}...
              </div>
            ) : (
              <div className="text-yellow-600 text-[10px]">No agent!</div>
            )}
          </div>
        </div>
      </div>
      <div>
        <div className="text-muted-foreground mb-1">Terminal Columns</div>
        <div className="flex gap-2">
          {terminalColumns.map((col) => (
            <div key={col.id} className="bg-muted/50 p-2 rounded">
              <div className="font-medium">{col.name}</div>
              <div className="text-[10px] text-muted-foreground">
                {col.status}
              </div>
            </div>
          ))}
          {terminalColumns.length === 0 && (
            <div className="text-muted-foreground">None configured</div>
          )}
        </div>
      </div>
    </div>
  );
}
