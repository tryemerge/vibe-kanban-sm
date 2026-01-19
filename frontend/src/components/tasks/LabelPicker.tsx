import { useState } from 'react';
import { Check, Plus, X, Tag } from 'lucide-react';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useProjectLabels, useTaskLabelMutations } from '@/hooks/useTaskLabels';
import { LabelBadge, LabelBadges } from '@/components/ui/label-badge';
import type { TaskLabel } from 'shared/types';

interface LabelPickerProps {
  projectId: string;
  taskId: string;
  currentLabels: TaskLabel[];
  onLabelsChange?: () => void;
  size?: 'sm' | 'default';
}

export function LabelPicker({
  projectId,
  taskId,
  currentLabels,
  onLabelsChange,
  size = 'default',
}: LabelPickerProps) {
  const [open, setOpen] = useState(false);
  const { data: allLabels = [], isLoading } = useProjectLabels(projectId);
  const { assignLabel, removeLabel } = useTaskLabelMutations(projectId);

  const currentLabelIds = new Set(currentLabels.map((l) => l.id));

  const handleToggleLabel = async (label: TaskLabel) => {
    if (currentLabelIds.has(label.id)) {
      await removeLabel.mutateAsync({ taskId, labelId: label.id });
    } else {
      await assignLabel.mutateAsync({ taskId, labelId: label.id });
    }
    onLabelsChange?.();
  };

  return (
    <div className="flex flex-col gap-2">
      {/* Display current labels */}
      {currentLabels.length > 0 && (
        <div className="flex flex-wrap gap-1 items-center">
          {currentLabels.map((label) => (
            <div
              key={label.id}
              className="flex items-center gap-0.5 group cursor-pointer"
              onClick={() => handleToggleLabel(label)}
            >
              <LabelBadge label={label} size={size} />
              <X className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
            </div>
          ))}
        </div>
      )}

      {/* Label picker popover */}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="ghost"
            size={size === 'sm' ? 'sm' : 'default'}
            className={cn(
              'justify-start text-muted-foreground hover:text-foreground',
              size === 'sm' && 'h-7 px-2 text-xs'
            )}
          >
            <Tag className={cn('mr-1', size === 'sm' ? 'h-3 w-3' : 'h-4 w-4')} />
            {currentLabels.length === 0 ? 'Add labels' : 'Edit labels'}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-64 p-2" align="start">
          <div className="space-y-2">
            <div className="text-sm font-medium text-muted-foreground px-2 py-1">
              Labels
            </div>
            {isLoading ? (
              <div className="text-sm text-muted-foreground px-2 py-4 text-center">
                Loading...
              </div>
            ) : allLabels.length === 0 ? (
              <div className="text-sm text-muted-foreground px-2 py-4 text-center">
                No labels in this project.
                <br />
                Create labels in Project Settings.
              </div>
            ) : (
              <div className="space-y-1">
                {allLabels.map((label) => {
                  const isSelected = currentLabelIds.has(label.id);
                  return (
                    <button
                      key={label.id}
                      className={cn(
                        'w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors',
                        'hover:bg-muted/50',
                        isSelected && 'bg-muted'
                      )}
                      onClick={() => handleToggleLabel(label)}
                      disabled={assignLabel.isPending || removeLabel.isPending}
                    >
                      <div
                        className="w-4 h-4 rounded-full flex-shrink-0 border"
                        style={{
                          backgroundColor: label.color || '#6b7280',
                          borderColor: label.color || '#6b7280',
                        }}
                      >
                        {isSelected && (
                          <Check className="h-4 w-4 text-white" />
                        )}
                      </div>
                      <span className="flex-1 text-left truncate">
                        {label.name}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
}

/**
 * Compact label display with inline editing capability
 */
interface InlineLabelPickerProps {
  projectId: string;
  taskId: string;
  currentLabels: TaskLabel[];
  onLabelsChange?: () => void;
}

export function InlineLabelPicker({
  projectId,
  taskId,
  currentLabels,
  onLabelsChange,
}: InlineLabelPickerProps) {
  const [open, setOpen] = useState(false);
  const { data: allLabels = [], isLoading } = useProjectLabels(projectId);
  const { assignLabel, removeLabel } = useTaskLabelMutations(projectId);

  const currentLabelIds = new Set(currentLabels.map((l) => l.id));

  const handleToggleLabel = async (label: TaskLabel) => {
    if (currentLabelIds.has(label.id)) {
      await removeLabel.mutateAsync({ taskId, labelId: label.id });
    } else {
      await assignLabel.mutateAsync({ taskId, labelId: label.id });
    }
    onLabelsChange?.();
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors">
          {currentLabels.length > 0 ? (
            <LabelBadges labels={currentLabels} size="sm" maxDisplay={2} />
          ) : (
            <span className="text-xs flex items-center gap-1">
              <Plus className="h-3 w-3" />
              Labels
            </span>
          )}
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-52 p-1.5" align="start">
        {isLoading ? (
          <div className="text-xs text-muted-foreground p-2 text-center">
            Loading...
          </div>
        ) : allLabels.length === 0 ? (
          <div className="text-xs text-muted-foreground p-2 text-center">
            No labels available
          </div>
        ) : (
          <div className="space-y-0.5">
            {allLabels.map((label) => {
              const isSelected = currentLabelIds.has(label.id);
              return (
                <button
                  key={label.id}
                  className={cn(
                    'w-full flex items-center gap-2 px-2 py-1 rounded text-xs transition-colors',
                    'hover:bg-muted/50',
                    isSelected && 'bg-muted'
                  )}
                  onClick={() => handleToggleLabel(label)}
                  disabled={assignLabel.isPending || removeLabel.isPending}
                >
                  <div
                    className="w-3 h-3 rounded-full flex-shrink-0 flex items-center justify-center"
                    style={{ backgroundColor: label.color || '#6b7280' }}
                  >
                    {isSelected && <Check className="h-2 w-2 text-white" />}
                  </div>
                  <span className="truncate">{label.name}</span>
                </button>
              );
            })}
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
