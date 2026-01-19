import { type ReactNode } from 'react';
import { ChevronDown, ChevronRight, Tag } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';

interface SwimLaneProps {
  title: string | null;
  taskCount: number;
  collapsed: boolean;
  onToggleCollapse: () => void;
  color?: string | null;
  children: ReactNode;
  className?: string;
}

/**
 * A collapsible swim lane that groups tasks within a kanban column
 */
export function SwimLane({
  title,
  taskCount,
  collapsed,
  onToggleCollapse,
  color,
  children,
  className,
}: SwimLaneProps) {
  // If no title (ungrouped/default lane), just render children
  if (!title) {
    return <div className={cn('flex flex-col', className)}>{children}</div>;
  }

  return (
    <div className={cn('flex flex-col', className)}>
      {/* Lane Header */}
      <Button
        variant="ghost"
        onClick={onToggleCollapse}
        className={cn(
          'flex items-center gap-2 px-3 py-2 h-auto justify-start rounded-none',
          'border-b border-dashed bg-muted/30 hover:bg-muted/50',
          'sticky top-[52px] z-10' // Below column header
        )}
      >
        {collapsed ? (
          <ChevronRight className="h-4 w-4 shrink-0" />
        ) : (
          <ChevronDown className="h-4 w-4 shrink-0" />
        )}

        {/* Label color indicator */}
        {color ? (
          <div
            className="h-3 w-3 rounded-full shrink-0"
            style={{ backgroundColor: color }}
          />
        ) : (
          <Tag className="h-3 w-3 shrink-0 text-muted-foreground" />
        )}

        <span className="text-sm font-medium truncate">{title}</span>

        <span className="text-xs text-muted-foreground ml-auto">
          {taskCount}
        </span>
      </Button>

      {/* Lane Content */}
      {!collapsed && (
        <div className="flex flex-col">{children}</div>
      )}
    </div>
  );
}

/**
 * Container for ungrouped tasks (no label) in swim lane view
 */
export function UnlabeledSwimLane({
  taskCount,
  collapsed,
  onToggleCollapse,
  children,
}: {
  taskCount: number;
  collapsed: boolean;
  onToggleCollapse: () => void;
  children: ReactNode;
}) {
  return (
    <SwimLane
      title="No Label"
      taskCount={taskCount}
      collapsed={collapsed}
      onToggleCollapse={onToggleCollapse}
      color={null}
    >
      {children}
    </SwimLane>
  );
}
