import type { TaskLabel } from 'shared/types';
import { cn } from '@/lib/utils';

interface LabelBadgeProps {
  label: TaskLabel;
  size?: 'sm' | 'default';
  className?: string;
}

export function LabelBadge({ label, size = 'default', className }: LabelBadgeProps) {
  const backgroundColor = label.color || '#6b7280';
  const isLight = isLightColor(backgroundColor);

  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full font-medium truncate',
        size === 'sm' ? 'px-1.5 py-0.5 text-[10px] max-w-[60px]' : 'px-2 py-0.5 text-xs max-w-[80px]',
        className
      )}
      style={{
        backgroundColor,
        color: isLight ? '#1f2937' : '#ffffff',
      }}
      title={label.name}
    >
      {label.name}
    </span>
  );
}

/**
 * Display multiple labels inline
 */
interface LabelBadgesProps {
  labels: TaskLabel[];
  maxDisplay?: number;
  size?: 'sm' | 'default';
  className?: string;
}

export function LabelBadges({
  labels,
  maxDisplay = 3,
  size = 'default',
  className,
}: LabelBadgesProps) {
  if (labels.length === 0) return null;

  const displayLabels = labels.slice(0, maxDisplay);
  const remaining = labels.length - maxDisplay;

  return (
    <div className={cn('flex flex-wrap gap-1 items-center', className)}>
      {displayLabels.map((label) => (
        <LabelBadge key={label.id} label={label} size={size} />
      ))}
      {remaining > 0 && (
        <span
          className={cn(
            'text-muted-foreground',
            size === 'sm' ? 'text-[10px]' : 'text-xs'
          )}
        >
          +{remaining}
        </span>
      )}
    </div>
  );
}

/**
 * Check if a color is light (should use dark text)
 */
function isLightColor(hexColor: string): boolean {
  // Remove # if present
  const hex = hexColor.replace('#', '');

  // Parse RGB values
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);

  // Calculate luminance
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;

  return luminance > 0.5;
}
