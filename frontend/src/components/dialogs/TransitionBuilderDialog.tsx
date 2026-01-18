import { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Loader2, ArrowRight, AlertCircle } from 'lucide-react';
import type { KanbanColumn, CreateStateTransition } from 'shared/types';
import { stateTransitionsApi } from '@/lib/api';

interface TransitionConfig {
  toColumnId: string | null;
  maxFailures: number | null;
}

interface TransitionBuilderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  boardId: string;
  columns: KanbanColumn[];
  onSuccess?: () => void;
}

export function TransitionBuilderDialog({
  open,
  onOpenChange,
  boardId,
  columns,
  onSuccess,
}: TransitionBuilderDialogProps) {
  const { t } = useTranslation(['settings']);
  const [fromColumnId, setFromColumnId] = useState<string>('');
  const [optionMappings, setOptionMappings] = useState<Map<string, TransitionConfig>>(new Map());
  const [simpleToColumnId, setSimpleToColumnId] = useState<string>('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fromColumn = useMemo(
    () => columns.find((c) => c.id === fromColumnId),
    [columns, fromColumnId]
  );

  const deliverableOptions = useMemo(() => {
    if (!fromColumn?.deliverable_options) return [];
    try {
      const parsed = JSON.parse(fromColumn.deliverable_options);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }, [fromColumn]);

  const hasDeliverableOptions = deliverableOptions.length > 0;

  // Reset mappings when fromColumn changes
  useEffect(() => {
    if (hasDeliverableOptions) {
      const initialMappings = new Map<string, TransitionConfig>();
      deliverableOptions.forEach((opt: string) => {
        initialMappings.set(opt, { toColumnId: null, maxFailures: null });
      });
      setOptionMappings(initialMappings);
    } else {
      setOptionMappings(new Map());
    }
    setSimpleToColumnId('');
    setError(null);
  }, [fromColumnId, hasDeliverableOptions, deliverableOptions]);

  // Reset on close
  useEffect(() => {
    if (!open) {
      setFromColumnId('');
      setOptionMappings(new Map());
      setSimpleToColumnId('');
      setError(null);
    }
  }, [open]);

  const updateMapping = (option: string, updates: Partial<TransitionConfig>) => {
    const newMappings = new Map(optionMappings);
    const current = newMappings.get(option) || { toColumnId: null, maxFailures: null };
    newMappings.set(option, { ...current, ...updates });
    setOptionMappings(newMappings);
  };

  const handleSave = async () => {
    if (!fromColumnId) return;
    setSaving(true);
    setError(null);

    try {
      if (hasDeliverableOptions && fromColumn?.deliverable_variable) {
        // Create transitions for each option mapping
        for (const [optionValue, config] of optionMappings) {
          if (config.toColumnId) {
            const createData: CreateStateTransition = {
              from_column_id: fromColumnId,
              to_column_id: config.toColumnId,
              else_column_id: null,
              escalation_column_id: null,
              name: optionValue,
              requires_confirmation: false,
              condition_key: fromColumn.deliverable_variable,
              condition_value: optionValue,
              max_failures: config.maxFailures,
            };
            await stateTransitionsApi.createForBoard(boardId, createData);
          }
        }
      } else if (simpleToColumnId) {
        // Simple transition without conditions
        const createData: CreateStateTransition = {
          from_column_id: fromColumnId,
          to_column_id: simpleToColumnId,
          else_column_id: null,
          escalation_column_id: null,
          name: null,
          requires_confirmation: false,
          condition_key: null,
          condition_value: null,
          max_failures: null,
        };
        await stateTransitionsApi.createForBoard(boardId, createData);
      }

      onOpenChange(false);
      onSuccess?.();
    } catch (err) {
      console.error('Failed to create transitions:', err);
      setError(
        err instanceof Error
          ? err.message
          : t('settings:boards.transitions.errors.saveFailed', 'Failed to save transition')
      );
    } finally {
      setSaving(false);
    }
  };

  const canSave = useMemo(() => {
    if (!fromColumnId) return false;

    if (hasDeliverableOptions) {
      // At least one mapping should be defined
      return Array.from(optionMappings.values()).some((m) => m.toColumnId);
    } else {
      return !!simpleToColumnId && simpleToColumnId !== fromColumnId;
    }
  }, [fromColumnId, hasDeliverableOptions, optionMappings, simpleToColumnId]);

  const otherColumns = useMemo(
    () => columns.filter((c) => c.id !== fromColumnId),
    [columns, fromColumnId]
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {t('settings:boards.transitions.builder.title', 'Build Transitions')}
          </DialogTitle>
          <DialogDescription>
            {t(
              'settings:boards.transitions.builder.description',
              'Create routing rules based on column deliverable options.'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {error && (
            <div className="flex items-center gap-2 p-3 bg-destructive/10 text-destructive rounded-md">
              <AlertCircle className="h-4 w-4" />
              <span className="text-sm">{error}</span>
            </div>
          )}

          {/* Step 1: Select From Column */}
          <div className="space-y-2">
            <Label htmlFor="from-column">
              {t('settings:boards.transitions.form.fromColumn', 'From Column')} *
            </Label>
            <Select value={fromColumnId} onValueChange={setFromColumnId}>
              <SelectTrigger id="from-column">
                <SelectValue
                  placeholder={t('settings:boards.transitions.form.selectColumn', 'Select column')}
                />
              </SelectTrigger>
              <SelectContent>
                {columns.map((col) => (
                  <SelectItem key={col.id} value={col.id}>
                    <div className="flex items-center gap-2">
                      {col.color && (
                        <span
                          className="w-3 h-3 rounded-full"
                          style={{ backgroundColor: col.color }}
                        />
                      )}
                      <span>{col.name}</span>
                      {col.deliverable_options && (
                        <span className="text-xs text-muted-foreground ml-1">
                          ({t('settings:boards.transitions.builder.hasOptions', 'has options')})
                        </span>
                      )}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Step 2: Map deliverable options or simple to column */}
          {fromColumnId && (
            <>
              {hasDeliverableOptions && fromColumn?.deliverable_variable ? (
                <div className="space-y-4">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <span>
                      {t('settings:boards.transitions.builder.mapOptions', 'Map')}
                    </span>
                    <code className="px-1.5 py-0.5 bg-muted rounded text-xs font-mono">
                      {fromColumn.deliverable_variable}
                    </code>
                    <span>
                      {t('settings:boards.transitions.builder.optionsTo', 'options to columns:')}
                    </span>
                  </div>

                  {deliverableOptions.map((option: string) => (
                    <div
                      key={option}
                      className="flex items-center gap-3 p-3 border rounded-lg bg-muted/30"
                    >
                      <code className="font-mono bg-muted px-2 py-1 rounded text-sm min-w-[80px]">
                        {option}
                      </code>
                      <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                      <Select
                        value={optionMappings.get(option)?.toColumnId || 'none'}
                        onValueChange={(value) =>
                          updateMapping(option, { toColumnId: value === 'none' ? null : value })
                        }
                      >
                        <SelectTrigger className="w-[200px]">
                          <SelectValue
                            placeholder={t('settings:boards.transitions.form.selectColumn', 'Select column')}
                          />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="none">
                            {t('settings:boards.transitions.builder.skip', '(skip)')}
                          </SelectItem>
                          {otherColumns.map((col) => (
                            <SelectItem key={col.id} value={col.id}>
                              <div className="flex items-center gap-2">
                                {col.color && (
                                  <span
                                    className="w-2 h-2 rounded-full"
                                    style={{ backgroundColor: col.color }}
                                  />
                                )}
                                {col.name}
                              </div>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <Input
                        type="number"
                        min={1}
                        placeholder={t('settings:boards.transitions.builder.maxFailures', 'Max fails')}
                        className="w-24"
                        value={optionMappings.get(option)?.maxFailures ?? ''}
                        onChange={(e) =>
                          updateMapping(option, {
                            maxFailures: e.target.value ? parseInt(e.target.value) : null,
                          })
                        }
                      />
                    </div>
                  ))}

                  <p className="text-xs text-muted-foreground">
                    {t(
                      'settings:boards.transitions.builder.maxFailuresHelp',
                      'Optional: Set max failures for escalation paths (leave empty for unlimited)'
                    )}
                  </p>
                </div>
              ) : (
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    {t(
                      'settings:boards.transitions.builder.noOptions',
                      'This column has no deliverable options defined. Creating a simple transition.'
                    )}
                  </p>

                  <div className="space-y-2">
                    <Label htmlFor="simple-to-column">
                      {t('settings:boards.transitions.form.toColumn', 'To Column')} *
                    </Label>
                    <Select value={simpleToColumnId} onValueChange={setSimpleToColumnId}>
                      <SelectTrigger id="simple-to-column">
                        <SelectValue
                          placeholder={t('settings:boards.transitions.form.selectColumn', 'Select column')}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {otherColumns.map((col) => (
                          <SelectItem key={col.id} value={col.id}>
                            <div className="flex items-center gap-2">
                              {col.color && (
                                <span
                                  className="w-2 h-2 rounded-full"
                                  style={{ backgroundColor: col.color }}
                                />
                              )}
                              {col.name}
                            </div>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <p className="text-xs text-muted-foreground">
                    {t(
                      'settings:boards.transitions.builder.addOptionsHint',
                      'Tip: Add deliverable options to a column to enable conditional routing.'
                    )}
                  </p>
                </div>
              )}
            </>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={saving}>
            {t('common:cancel', 'Cancel')}
          </Button>
          <Button onClick={handleSave} disabled={!canSave || saving}>
            {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {hasDeliverableOptions
              ? t('settings:boards.transitions.builder.createTransitions', 'Create Transitions')
              : t('settings:boards.transitions.builder.createTransition', 'Create Transition')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
