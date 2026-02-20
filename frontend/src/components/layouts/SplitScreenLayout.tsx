import { useState, useRef, useEffect, ReactNode } from 'react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { ChevronUp, ChevronDown } from 'lucide-react';

interface SplitScreenLayoutProps {
  topPanel: ReactNode;
  bottomPanel: ReactNode;
  initialSplit?: number; // Percentage (0-100) for top panel height
  minTopHeight?: number; // Minimum height in pixels for top panel
  minBottomHeight?: number; // Minimum height in pixels for bottom panel
}

export function SplitScreenLayout({
  topPanel,
  bottomPanel,
  initialSplit = 30,
  minTopHeight = 200,
  minBottomHeight = 300,
}: SplitScreenLayoutProps) {
  const [splitPosition, setSplitPosition] = useState(initialSplit);
  const [isDragging, setIsDragging] = useState(false);
  const [isTopCollapsed, setIsTopCollapsed] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Handle mouse move during drag
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging || !containerRef.current) return;

      const containerRect = containerRef.current.getBoundingClientRect();
      const containerHeight = containerRect.height;
      const mouseY = e.clientY - containerRect.top;
      const newSplit = (mouseY / containerHeight) * 100;

      // Calculate min/max based on pixel heights
      const minTopPercent = (minTopHeight / containerHeight) * 100;
      const maxTopPercent = 100 - (minBottomHeight / containerHeight) * 100;

      // Clamp the split position
      const clampedSplit = Math.max(
        minTopPercent,
        Math.min(maxTopPercent, newSplit)
      );

      setSplitPosition(clampedSplit);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'ns-resize';
      document.body.style.userSelect = 'none';
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isDragging, minTopHeight, minBottomHeight]);

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const toggleTopPanel = () => {
    setIsTopCollapsed(!isTopCollapsed);
  };

  const topHeight = isTopCollapsed ? 0 : splitPosition;
  const bottomHeight = isTopCollapsed ? 100 : 100 - splitPosition;

  return (
    <div ref={containerRef} className="flex flex-col h-full w-full relative">
      {/* Top Panel - TaskGroup Board */}
      <div
        className={cn(
          'flex flex-col overflow-hidden transition-all duration-300 ease-in-out border-b',
          isTopCollapsed && 'h-0'
        )}
        style={{
          height: isTopCollapsed ? '0%' : `${topHeight}%`,
        }}
      >
        <div className="flex items-center justify-between px-4 py-2 bg-muted/50 border-b">
          <h2 className="text-sm font-semibold">Task Groups</h2>
          <Button
            variant="ghost"
            size="sm"
            onClick={toggleTopPanel}
            className="h-6 px-2"
          >
            <ChevronUp className="h-4 w-4" />
          </Button>
        </div>
        <div className="flex-1 overflow-hidden">{topPanel}</div>
      </div>

      {/* Divider / Resize Handle */}
      {!isTopCollapsed && (
        <div
          className={cn(
            'relative h-1 bg-border hover:bg-primary/20 cursor-ns-resize transition-colors group',
            isDragging && 'bg-primary/30'
          )}
          onMouseDown={handleMouseDown}
        >
          <div className="absolute inset-x-0 -top-1 -bottom-1 flex items-center justify-center">
            <div className="h-1 w-12 rounded-full bg-muted-foreground/30 group-hover:bg-primary/40 transition-colors" />
          </div>
        </div>
      )}

      {/* Bottom Panel - Task Board */}
      <div
        className="flex flex-col overflow-hidden transition-all duration-300 ease-in-out"
        style={{
          height: `${bottomHeight}%`,
        }}
      >
        <div className="flex items-center justify-between px-4 py-2 bg-muted/50 border-b">
          <h2 className="text-sm font-semibold">Tasks</h2>
          {isTopCollapsed && (
            <Button
              variant="ghost"
              size="sm"
              onClick={toggleTopPanel}
              className="h-6 px-2"
            >
              <ChevronDown className="h-4 w-4" />
            </Button>
          )}
        </div>
        <div className="flex-1 overflow-hidden">{bottomPanel}</div>
      </div>
    </div>
  );
}
