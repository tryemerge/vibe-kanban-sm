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
import { Checkbox } from '@/components/ui/checkbox';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Loader2, ArrowRight, AlertCircle, Variable } from 'lucide-react';
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
  const [elseColumnId, setElseColumnId] = useState<string>('');
  const [escalationColumnId, setEscalationColumnId] = useState<string>('');
  const [simpleToColumnId, setSimpleToColumnId] = useState<string>('');
  const [useConditionalLogic, setUseConditionalLogic] = useState(true);
  const [requiresConfirmation, setRequiresConfirmation] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fromColumn = useMemo(
    () => columns.find((c) => c.id === fromColumnId),
    [columns, fromColumnId]
  );

  const deliverableOptions = useMemo(() => {
    if (!fromColumn?.answer_options) return [];
    try {
      const parsed = JSON.parse(fromColumn.answer_options);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }, [fromColumn]);

  const hasDeliverableOptions = deliverableOptions.length > 0;
  // Use conditional logic only if enabled AND column has options
  const showConditionalUI = hasDeliverableOptions && useConditionalLogic;

  // Reset mappings when fromColumn changes
  useEffect(() => {
    if (hasDeliverableOptions) {
      const initialMappings = new Map<string, TransitionConfig>();
      deliverableOptions.forEach((opt: string) => {
        initialMappings.set(opt, { toColumnId: null, maxFailures: null });
      });
      setOptionMappings(initialMappings);
      setUseConditionalLogic(true);
    } else {
      setOptionMappings(new Map());
      setUseConditionalLogic(false);
    }
    setSimpleToColumnId('');
    setElseColumnId('');
    setEscalationColumnId('');
    setRequiresConfirmation(false);
    setError(null);
  }, [fromColumnId, hasDeliverableOptions, deliverableOptions]);

  // Reset on close
  useEffect(() => {
    if (!open) {
      setFromColumnId('');
      setOptionMappings(new Map());
      setSimpleToColumnId('');
      setElseColumnId('');
      setEscalationColumnId('');
      setUseConditionalLogic(true);
      setRequiresConfirmation(false);
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
      if (showConditionalUI && fromColumn?.question) {
        // Create transitions for each option mapping
        for (const [optionValue, config] of optionMappings) {
          if (config.toColumnId) {
            const createData: CreateStateTransition = {
              from_column_id: fromColumnId,
              to_column_id: config.toColumnId,
              else_column_id: elseColumnId || null,
              escalation_column_id: escalationColumnId || null,
              name: optionValue,
              requires_confirmation: requiresConfirmation,
              condition_value: optionValue,
              max_failures: config.maxFailures,
            };
            await stateTransitionsApi.createForBoard(boardId, createData);
          }
        }
      } else if (simpleToColumnId) {
        // Simple transition without conditions (or conditional logic disabled)
        const createData: CreateStateTransition = {
          from_column_id: fromColumnId,
          to_column_id: simpleToColumnId,
          else_column_id: null,
          escalation_column_id: null,
          name: null,
          requires_confirmation: requiresConfirmation,
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

    if (showConditionalUI) {
      // At least one mapping should be defined
      return Array.from(optionMappings.values()).some((m) => m.toColumnId);
    } else {
      return !!simpleToColumnId && simpleToColumnId !== fromColumnId;
    }
  }, [fromColumnId, showConditionalUI, optionMappings, simpleToColumnId]);

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
              'Create routing rules based on column question answers.'
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
                      {col.question && (
                        <span className="text-xs text-muted-foreground ml-1">
                          (has question)
                        </span>
                      )}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Step 2: Show variable info and conditional toggle */}
          {fromColumnId && hasDeliverableOptions && fromColumn?.question && (
            <div className="space-y-4">
              {/* Question display */}
              <div className="flex items-center gap-3 p-3 border rounded-lg bg-muted/50">
                <Variable className="h-5 w-5 text-muted-foreground flex-shrink-0" />
                <div className="flex-1">
                  <div className="text-sm font-medium">
                    {t('settings:boards.transitions.builder.variableName', 'Question')}
                  </div>
                  <span className="text-sm text-primary">
                    {fromColumn.question}
                  </span>
                </div>
                <div className="text-xs text-muted-foreground">
                  {deliverableOptions.length} {t('settings:boards.transitions.builder.optionsCount', 'answers')}
                </div>
              </div>

              {/* Conditional logic toggle */}
              <div className="flex items-center gap-2">
                <Checkbox
                  id="use-conditional"
                  checked={useConditionalLogic}
                  onCheckedChange={(checked) => setUseConditionalLogic(checked === true)}
                />
                <Label htmlFor="use-conditional" className="text-sm font-normal cursor-pointer">
                  {t('settings:boards.transitions.builder.useConditional', 'Use conditional logic')}
                </Label>
              </div>
              {!useConditionalLogic && (
                <p className="text-xs text-muted-foreground pl-6">
                  {t(
                    'settings:boards.transitions.builder.conditionalDisabled',
                    'Conditional logic disabled. Creating a simple unconditional transition.'
                  )}
                </p>
              )}
            </div>
          )}

          {/* Step 3: Map deliverable options or simple to column */}
          {fromColumnId && (
            <>
              {showConditionalUI && fromColumn?.question ? (
                <div className="space-y-4">
                  <div className="text-sm text-muted-foreground">
                    {t('settings:boards.transitions.builder.mapOptionsTo', 'Map options to destination columns:')}
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

                  {/* Else column dropdown */}
                  <div className="flex items-center gap-3 p-3 border rounded-lg bg-amber-500/10 border-amber-500/30">
                    <code className="font-mono bg-amber-500/20 text-amber-700 dark:text-amber-300 px-2 py-1 rounded text-sm min-w-[80px]">
                      else
                    </code>
                    <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                    <Select value={elseColumnId || 'none'} onValueChange={(value) => setElseColumnId(value === 'none' ? '' : value)}>
                      <SelectTrigger className="w-[200px]">
                        <SelectValue
                          placeholder={t('settings:boards.transitions.builder.selectElse', 'Select fallback')}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="none">
                          {t('settings:boards.transitions.builder.noFallback', '(no fallback)')}
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
                    <span className="text-xs text-muted-foreground">
                      {t('settings:boards.transitions.builder.elseHelp', 'When no condition matches')}
                    </span>
                  </div>

                  {/* Escalation column dropdown */}
                  <div className="space-y-2">
                    <Label>
                      {t('settings:boards.transitions.form.escalationColumn', 'Escalation Column')}
                    </Label>
                    <Select value={escalationColumnId || 'none'} onValueChange={(value) => setEscalationColumnId(value === 'none' ? '' : value)}>
                      <SelectTrigger>
                        <SelectValue
                          placeholder={t('settings:boards.transitions.form.selectColumn', 'Select column')}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="none">
                          {t('settings:boards.transitions.form.noneOptional', '(none)')}
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
                    <p className="text-xs text-muted-foreground">
                      {t('settings:boards.transitions.form.escalationHelp', 'Where to go when max failures is reached')}
                    </p>
                  </div>

                  <p className="text-xs text-muted-foreground">
                    {t(
                      'settings:boards.transitions.builder.maxFailuresHelp',
                      'Optional: Set max failures per option. After that many trips through the else path, task goes to escalation column instead.'
                    )}
                  </p>
                </div>
              ) : (
                <div className="space-y-4">
                  {!hasDeliverableOptions && (
                    <p className="text-sm text-muted-foreground">
                      {t(
                        'settings:boards.transitions.builder.noOptions',
                        'This column has no question defined. Creating a simple transition.'
                      )}
                    </p>
                  )}

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

                  {!hasDeliverableOptions && (
                    <p className="text-xs text-muted-foreground">
                      {t(
                        'settings:boards.transitions.builder.addOptionsHint',
                        'Tip: Add a question and answer options to a column to enable conditional routing.'
                      )}
                    </p>
                  )}
                </div>
              )}
            </>
          )}

          {/* Requires confirmation checkbox */}
          {fromColumnId && (
            <div className="flex items-center gap-2 pt-2 border-t">
              <Checkbox
                id="requires-confirmation"
                checked={requiresConfirmation}
                onCheckedChange={(checked) => setRequiresConfirmation(checked === true)}
              />
              <Label htmlFor="requires-confirmation" className="text-sm font-normal cursor-pointer">
                {t('settings:boards.transitions.form.requiresConfirmation', 'Requires confirmation')}
              </Label>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={saving}>
            {t('common:cancel', 'Cancel')}
          </Button>
          <Button onClick={handleSave} disabled={!canSave || saving}>
            {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {showConditionalUI
              ? t('settings:boards.transitions.builder.createTransitions', 'Create Transitions')
              : t('settings:boards.transitions.builder.createTransition', 'Create Transition')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
