import { useEffect, useState, useCallback } from 'react';
import { TransitionBuilderDialog } from '@/components/dialogs/TransitionBuilderDialog';
import { useTranslation } from 'react-i18next';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Loader2,
  Plus,
  Pencil,
  Trash2,
  LayoutGrid,
  ChevronDown,
  ChevronRight,
  GripVertical,
  Columns3,
  ArrowRight,
  GitBranch,
} from 'lucide-react';
import { boardsApi, agentsApi, stateTransitionsApi } from '@/lib/api';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type {
  Board,
  CreateBoard,
  UpdateBoard,
  KanbanColumn,
  CreateKanbanColumn,
  UpdateKanbanColumn,
  Agent,
  TaskStatus,
  StateTransitionWithColumns,
  CreateStateTransition,
} from 'shared/types';

export function BoardSettings() {
  const { t } = useTranslation(['settings', 'common']);

  // Boards state
  const [boards, setBoards] = useState<Board[]>([]);
  const [boardsLoading, setBoardsLoading] = useState(true);
  const [boardsError, setBoardsError] = useState<string | null>(null);
  const [boardDialogOpen, setBoardDialogOpen] = useState(false);
  const [editingBoard, setEditingBoard] = useState<Board | null>(null);
  const [boardSaving, setBoardSaving] = useState(false);
  const [boardForm, setBoardForm] = useState<CreateBoard>({
    name: '',
    description: null,
  });
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [boardToDelete, setBoardToDelete] = useState<Board | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Expanded boards (for showing columns)
  const [expandedBoards, setExpandedBoards] = useState<Set<string>>(new Set());

  // Columns state (per board)
  const [columnsMap, setColumnsMap] = useState<Map<string, KanbanColumn[]>>(
    new Map()
  );
  const [columnsLoadingMap, setColumnsLoadingMap] = useState<Map<string, boolean>>(
    new Map()
  );

  // Column dialog state
  const [columnDialogOpen, setColumnDialogOpen] = useState(false);
  const [editingColumn, setEditingColumn] = useState<KanbanColumn | null>(null);
  const [columnBoardId, setColumnBoardId] = useState<string | null>(null);
  const [columnSaving, setColumnSaving] = useState(false);
  const [columnForm, setColumnForm] = useState<CreateKanbanColumn>({
    name: '',
    slug: '',
    position: 0,
    color: null,
    is_initial: false,
    is_terminal: false,
    starts_workflow: false,
    status: null,
    agent_id: null,
    deliverable: null,
    deliverable_variable: null,
    deliverable_options: null,
  });
  const [deleteColumnConfirmOpen, setDeleteColumnConfirmOpen] = useState(false);
  const [columnToDelete, setColumnToDelete] = useState<{
    boardId: string;
    column: KanbanColumn;
  } | null>(null);

  // Agents state
  const [agents, setAgents] = useState<Agent[]>([]);
  const [agentsLoading, setAgentsLoading] = useState(true);

  // Transitions state (per board)
  const [transitionsMap, setTransitionsMap] = useState<Map<string, StateTransitionWithColumns[]>>(
    new Map()
  );
  const [transitionsLoadingMap, setTransitionsLoadingMap] = useState<Map<string, boolean>>(
    new Map()
  );

  // Transition dialog state
  const [transitionDialogOpen, setTransitionDialogOpen] = useState(false);
  const [transitionBoardId, setTransitionBoardId] = useState<string | null>(null);
  const [transitionSaving, setTransitionSaving] = useState(false);
  const [transitionForm, setTransitionForm] = useState<CreateStateTransition>({
    from_column_id: '',
    to_column_id: '',
    else_column_id: null,
    escalation_column_id: null,
    name: null,
    requires_confirmation: false,
    condition_key: null,
    condition_value: null,
    max_failures: null,
  });
  const [deleteTransitionConfirmOpen, setDeleteTransitionConfirmOpen] = useState(false);
  const [transitionToDelete, setTransitionToDelete] = useState<{
    boardId: string;
    transition: StateTransitionWithColumns;
  } | null>(null);

  // Transition Builder Dialog state
  const [transitionBuilderOpen, setTransitionBuilderOpen] = useState(false);
  const [transitionBuilderBoardId, setTransitionBuilderBoardId] = useState<string | null>(null);

  // Fetch agents
  const fetchAgents = useCallback(async () => {
    setAgentsLoading(true);
    try {
      const result = await agentsApi.list();
      setAgents(result);
    } catch (err) {
      console.error('Failed to fetch agents:', err);
    } finally {
      setAgentsLoading(false);
    }
  }, []);

  // Fetch boards
  const fetchBoards = useCallback(async () => {
    setBoardsLoading(true);
    setBoardsError(null);
    try {
      const result = await boardsApi.list();
      setBoards(result);
    } catch (err) {
      console.error('Failed to fetch boards:', err);
      setBoardsError(t('settings.boards.errors.loadFailed'));
    } finally {
      setBoardsLoading(false);
    }
  }, [t]);

  // Fetch columns for a board
  const fetchColumns = useCallback(async (boardId: string) => {
    setColumnsLoadingMap((prev) => new Map(prev).set(boardId, true));
    try {
      const columns = await boardsApi.listColumns(boardId);
      setColumnsMap((prev) => new Map(prev).set(boardId, columns));
    } catch (err) {
      console.error('Failed to fetch columns for board:', boardId, err);
    } finally {
      setColumnsLoadingMap((prev) => new Map(prev).set(boardId, false));
    }
  }, []);

  // Fetch transitions for a board
  const fetchTransitions = useCallback(async (boardId: string) => {
    setTransitionsLoadingMap((prev) => new Map(prev).set(boardId, true));
    try {
      const transitions = await stateTransitionsApi.listByBoard(boardId);
      setTransitionsMap((prev) => new Map(prev).set(boardId, transitions));
    } catch (err) {
      console.error('Failed to fetch transitions for board:', boardId, err);
    } finally {
      setTransitionsLoadingMap((prev) => new Map(prev).set(boardId, false));
    }
  }, []);

  // Load boards and agents on mount
  useEffect(() => {
    fetchBoards();
    fetchAgents();
  }, [fetchBoards, fetchAgents]);

  // Toggle board expansion
  const toggleBoardExpanded = (boardId: string) => {
    setExpandedBoards((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(boardId)) {
        newSet.delete(boardId);
      } else {
        newSet.add(boardId);
        // Fetch columns and transitions if not already loaded
        if (!columnsMap.has(boardId)) {
          fetchColumns(boardId);
        }
        if (!transitionsMap.has(boardId)) {
          fetchTransitions(boardId);
        }
      }
      return newSet;
    });
  };

  // Open create board dialog
  const openCreateBoardDialog = () => {
    setEditingBoard(null);
    setBoardForm({
      name: '',
      description: null,
    });
    setBoardDialogOpen(true);
  };

  // Open edit board dialog
  const openEditBoardDialog = (board: Board) => {
    setEditingBoard(board);
    setBoardForm({
      name: board.name,
      description: board.description,
    });
    setBoardDialogOpen(true);
  };

  // Handle board form save
  const handleBoardSave = async () => {
    setBoardSaving(true);
    setBoardsError(null);
    try {
      if (editingBoard) {
        // Update existing
        const updateData: UpdateBoard = {
          name: boardForm.name || null,
          description: boardForm.description,
        };
        await boardsApi.update(editingBoard.id, updateData);
        setSuccessMessage(t('settings.boards.save.updateSuccess'));
      } else {
        // Create new
        await boardsApi.create(boardForm);
        setSuccessMessage(t('settings.boards.save.createSuccess'));
      }
      setBoardDialogOpen(false);
      await fetchBoards();
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      console.error('Failed to save board:', err);
      setBoardsError(t('settings.boards.errors.saveFailed'));
    } finally {
      setBoardSaving(false);
    }
  };

  // Handle board delete
  const handleBoardDelete = async () => {
    if (!boardToDelete) return;
    setBoardSaving(true);
    setBoardsError(null);
    try {
      await boardsApi.delete(boardToDelete.id);
      setDeleteConfirmOpen(false);
      setBoardToDelete(null);
      setSuccessMessage(t('settings.boards.save.deleteSuccess'));
      await fetchBoards();
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      console.error('Failed to delete board:', err);
      setBoardsError(
        err instanceof Error
          ? err.message
          : t('settings.boards.errors.deleteFailed')
      );
    } finally {
      setBoardSaving(false);
    }
  };

  // Generate slug from name
  const generateSlug = (name: string): string => {
    return name
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_|_$/g, '');
  };

  // Open create column dialog
  const openCreateColumnDialog = (boardId: string) => {
    setEditingColumn(null);
    setColumnBoardId(boardId);
    const existingColumns = columnsMap.get(boardId) || [];
    setColumnForm({
      name: '',
      slug: '',
      position: existingColumns.length,
      color: null,
      is_initial: existingColumns.length === 0,
      is_terminal: false,
      starts_workflow: false,
      status: 'todo',
      agent_id: null,
      deliverable: null,
      deliverable_variable: null,
      deliverable_options: null,
    });
    setColumnDialogOpen(true);
  };

  // Open edit column dialog
  const openEditColumnDialog = (boardId: string, column: KanbanColumn) => {
    setEditingColumn(column);
    setColumnBoardId(boardId);
    setColumnForm({
      name: column.name,
      slug: column.slug,
      position: column.position,
      color: column.color,
      is_initial: column.is_initial,
      is_terminal: column.is_terminal,
      starts_workflow: column.starts_workflow,
      status: column.status,
      agent_id: column.agent_id ?? null,
      deliverable: column.deliverable ?? null,
      deliverable_variable: column.deliverable_variable ?? null,
      deliverable_options: column.deliverable_options ?? null,
    });
    setColumnDialogOpen(true);
  };

  // Handle column form save
  const handleColumnSave = async () => {
    if (!columnBoardId) return;
    setColumnSaving(true);
    setBoardsError(null);
    try {
      if (editingColumn) {
        // Update existing
        const updateData: UpdateKanbanColumn = {
          name: columnForm.name || null,
          slug: columnForm.slug || null,
          position: columnForm.position,
          color: columnForm.color,
          is_initial: columnForm.is_initial,
          is_terminal: columnForm.is_terminal,
          starts_workflow: columnForm.starts_workflow,
          status: columnForm.status,
          agent_id: columnForm.agent_id,
          deliverable: columnForm.deliverable,
          deliverable_variable: columnForm.deliverable_variable,
          deliverable_options: columnForm.deliverable_options,
        };
        await boardsApi.updateColumn(columnBoardId, editingColumn.id, updateData);
        setSuccessMessage(t('settings.boards.columns.save.updateSuccess'));
      } else {
        // Create new
        await boardsApi.createColumn(columnBoardId, columnForm);
        setSuccessMessage(t('settings.boards.columns.save.createSuccess'));
      }
      setColumnDialogOpen(false);
      await fetchColumns(columnBoardId);
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      console.error('Failed to save column:', err);
      setBoardsError(t('settings.boards.columns.errors.saveFailed'));
    } finally {
      setColumnSaving(false);
    }
  };

  // Handle column delete
  const handleColumnDelete = async () => {
    if (!columnToDelete) return;
    setColumnSaving(true);
    setBoardsError(null);
    try {
      await boardsApi.deleteColumn(
        columnToDelete.boardId,
        columnToDelete.column.id
      );
      setDeleteColumnConfirmOpen(false);
      setColumnToDelete(null);
      setSuccessMessage(t('settings.boards.columns.save.deleteSuccess'));
      await fetchColumns(columnToDelete.boardId);
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      console.error('Failed to delete column:', err);
      setBoardsError(
        err instanceof Error
          ? err.message
          : t('settings.boards.columns.errors.deleteFailed')
      );
    } finally {
      setColumnSaving(false);
    }
  };

  // Open create transition dialog
  const openCreateTransitionDialog = (boardId: string) => {
    setTransitionBoardId(boardId);
    const columns = columnsMap.get(boardId) || [];
    setTransitionForm({
      from_column_id: columns.length > 0 ? columns[0].id : '',
      to_column_id: columns.length > 1 ? columns[1].id : '',
      else_column_id: null,
      escalation_column_id: null,
      name: null,
      requires_confirmation: false,
      condition_key: null,
      condition_value: null,
      max_failures: null,
    });
    setTransitionDialogOpen(true);
  };

  // Handle transition form save
  const handleTransitionSave = async () => {
    if (!transitionBoardId) return;
    setTransitionSaving(true);
    setBoardsError(null);
    try {
      await stateTransitionsApi.createForBoard(transitionBoardId, transitionForm);
      setSuccessMessage(t('settings.boards.transitions.save.createSuccess', 'Transition created successfully'));
      setTransitionDialogOpen(false);
      await fetchTransitions(transitionBoardId);
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      console.error('Failed to save transition:', err);
      setBoardsError(t('settings.boards.transitions.errors.saveFailed', 'Failed to save transition'));
    } finally {
      setTransitionSaving(false);
    }
  };

  // Handle transition delete
  const handleTransitionDelete = async () => {
    if (!transitionToDelete) return;
    setTransitionSaving(true);
    setBoardsError(null);
    try {
      await stateTransitionsApi.deleteFromBoard(
        transitionToDelete.boardId,
        transitionToDelete.transition.id
      );
      setDeleteTransitionConfirmOpen(false);
      setTransitionToDelete(null);
      setSuccessMessage(t('settings.boards.transitions.save.deleteSuccess', 'Transition deleted successfully'));
      await fetchTransitions(transitionToDelete.boardId);
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      console.error('Failed to delete transition:', err);
      setBoardsError(
        err instanceof Error
          ? err.message
          : t('settings.boards.transitions.errors.deleteFailed', 'Failed to delete transition')
      );
    } finally {
      setTransitionSaving(false);
    }
  };

  // Drag and drop sensors for column reordering
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  // Handle column drag end for reordering
  const handleColumnDragEnd = async (boardId: string, event: DragEndEvent) => {
    const { active, over } = event;

    if (!over || active.id === over.id) {
      return;
    }

    const columns = columnsMap.get(boardId) || [];
    const oldIndex = columns.findIndex((col) => col.id === active.id);
    const newIndex = columns.findIndex((col) => col.id === over.id);

    if (oldIndex === -1 || newIndex === -1) {
      return;
    }

    // Optimistically update the UI
    const newColumns = arrayMove(columns, oldIndex, newIndex);
    setColumnsMap((prev) => new Map(prev).set(boardId, newColumns));

    // Persist the new order to the server
    try {
      const columnIds = newColumns.map((col) => col.id);
      await boardsApi.reorderColumns(boardId, columnIds);
    } catch (err) {
      console.error('Failed to reorder columns:', err);
      // Revert on error
      setColumnsMap((prev) => new Map(prev).set(boardId, columns));
      setBoardsError(t('settings.boards.columns.errors.reorderFailed'));
    }
  };

  // Sortable column item component
  const SortableColumnItem = ({
    column,
    boardId,
  }: {
    column: KanbanColumn;
    boardId: string;
  }) => {
    const {
      attributes,
      listeners,
      setNodeRef,
      transform,
      transition,
      isDragging,
    } = useSortable({ id: column.id });

    const style = {
      transform: CSS.Transform.toString(transform),
      transition,
      opacity: isDragging ? 0.5 : 1,
    };

    return (
      <div
        ref={setNodeRef}
        style={style}
        className="flex items-center gap-2 p-2 border rounded bg-muted/30"
      >
        <button
          type="button"
          className="touch-none cursor-grab active:cursor-grabbing"
          {...attributes}
          {...listeners}
        >
          <GripVertical className="h-4 w-4 text-muted-foreground" />
        </button>
        <div
          className="w-3 h-3 rounded-full flex-shrink-0"
          style={{ backgroundColor: column.color || '#6b7280' }}
        />
        <div className="flex-1 min-w-0">
          <span className="font-medium text-sm">{column.name}</span>
          <span className="text-xs text-muted-foreground ml-2">
            ({column.slug})
          </span>
          {column.is_initial && (
            <span className="ml-2 text-xs bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 px-1.5 py-0.5 rounded">
              {t('settings.boards.columns.initial')}
            </span>
          )}
          {column.is_terminal && (
            <span className="ml-2 text-xs bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300 px-1.5 py-0.5 rounded">
              {t('settings.boards.columns.terminal')}
            </span>
          )}
          {column.starts_workflow && (
            <span className="ml-2 text-xs bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300 px-1.5 py-0.5 rounded">
              {t('settings.boards.columns.startsWorkflow', 'Workflow')}
            </span>
          )}
          {column.agent_id && (
            <span className="ml-2 text-xs bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300 px-1.5 py-0.5 rounded">
              {agents.find((a) => a.id === column.agent_id)?.name ||
                t('settings.boards.columns.unknownAgent')}
            </span>
          )}
          {column.deliverable && (
            <span className="ml-2 text-xs bg-cyan-100 text-cyan-700 dark:bg-cyan-900 dark:text-cyan-300 px-1.5 py-0.5 rounded" title={column.deliverable}>
              {t('settings.boards.columns.hasDeliverable', 'Deliverable')}
            </span>
          )}
          <span className="ml-2 text-xs bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300 px-1.5 py-0.5 rounded">
            {t(`settings.boards.columns.status.${column.status}`)}
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => openEditColumnDialog(boardId, column)}
        >
          <Pencil className="h-3 w-3" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setColumnToDelete({ boardId, column });
            setDeleteColumnConfirmOpen(true);
          }}
        >
          <Trash2 className="h-3 w-3 text-destructive" />
        </Button>
      </div>
    );
  };

  // Render columns for a board
  const renderColumns = (boardId: string) => {
    const isLoading = columnsLoadingMap.get(boardId);
    const columns = columnsMap.get(boardId) || [];

    if (isLoading) {
      return (
        <div className="flex items-center justify-center py-4">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="ml-2 text-sm text-muted-foreground">
            {t('settings.boards.columns.loading')}
          </span>
        </div>
      );
    }

    if (columns.length === 0) {
      return (
        <div className="text-center py-4 text-muted-foreground">
          <Columns3 className="h-8 w-8 mx-auto mb-2 opacity-50" />
          <p className="text-sm">{t('settings.boards.columns.empty')}</p>
        </div>
      );
    }

    return (
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={(event) => handleColumnDragEnd(boardId, event)}
      >
        <SortableContext
          items={columns.map((col) => col.id)}
          strategy={verticalListSortingStrategy}
        >
          <div className="space-y-2">
            {columns.map((column) => (
              <SortableColumnItem
                key={column.id}
                column={column}
                boardId={boardId}
              />
            ))}
          </div>
        </SortableContext>
      </DndContext>
    );
  };

  // Render transitions for a board
  const renderTransitions = (boardId: string) => {
    const isLoading = transitionsLoadingMap.get(boardId);
    const transitions = transitionsMap.get(boardId) || [];

    if (isLoading) {
      return (
        <div className="flex items-center justify-center py-4">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="ml-2 text-sm text-muted-foreground">
            {t('settings.boards.transitions.loading', 'Loading transitions...')}
          </span>
        </div>
      );
    }

    if (transitions.length === 0) {
      return (
        <div className="text-center py-4 text-muted-foreground">
          <GitBranch className="h-8 w-8 mx-auto mb-2 opacity-50" />
          <p className="text-sm">{t('settings.boards.transitions.empty', 'No transitions defined')}</p>
          <p className="text-xs mt-1">{t('settings.boards.transitions.emptyHelp', 'Define transitions to control workflow routing')}</p>
        </div>
      );
    }

    return (
      <div className="space-y-2">
        {transitions.map((transition) => (
          <div
            key={transition.id}
            className="flex items-center gap-2 p-2 border rounded bg-muted/30"
          >
            <GitBranch className="h-4 w-4 text-muted-foreground flex-shrink-0" />
            <div className="flex-1 min-w-0 flex flex-wrap items-center gap-2">
              <span className="text-sm font-medium">{transition.from_column_name}</span>
              <ArrowRight className="h-3 w-3 text-muted-foreground" />
              <span className="text-sm font-medium text-green-600 dark:text-green-400">{transition.to_column_name}</span>
              {transition.name && (
                <span className="text-xs text-muted-foreground ml-2">
                  "{transition.name}"
                </span>
              )}
              {transition.condition_key && (
                <span className="ml-2 text-xs bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300 px-1.5 py-0.5 rounded">
                  if {transition.condition_key}={transition.condition_value}
                </span>
              )}
              {transition.else_column_name && (
                <span className="ml-2 text-xs bg-orange-100 text-orange-700 dark:bg-orange-900 dark:text-orange-300 px-1.5 py-0.5 rounded">
                  else → {transition.else_column_name}
                </span>
              )}
              {transition.escalation_column_name && (
                <span className="ml-2 text-xs bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300 px-1.5 py-0.5 rounded">
                  escalate → {transition.escalation_column_name}
                </span>
              )}
              {transition.max_failures !== null && (
                <span className="ml-2 text-xs bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300 px-1.5 py-0.5 rounded">
                  max {String(transition.max_failures)} failures
                </span>
              )}
              {transition.requires_confirmation && (
                <span className="ml-2 text-xs bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 px-1.5 py-0.5 rounded">
                  {t('settings.boards.transitions.requiresConfirm', 'confirm')}
                </span>
              )}
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setTransitionToDelete({ boardId, transition });
                setDeleteTransitionConfirmOpen(true);
              }}
            >
              <Trash2 className="h-3 w-3 text-destructive" />
            </Button>
          </div>
        ))}
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {boardsError && (
        <Alert variant="destructive">
          <AlertDescription>{boardsError}</AlertDescription>
        </Alert>
      )}

      {successMessage && (
        <Alert variant="success">
          <AlertDescription className="font-medium">
            {successMessage}
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
          <div>
            <CardTitle>{t('settings.boards.title')}</CardTitle>
            <CardDescription>{t('settings.boards.description')}</CardDescription>
          </div>
          <Button onClick={openCreateBoardDialog} size="sm">
            <Plus className="h-4 w-4 mr-2" />
            {t('settings.boards.actions.create')}
          </Button>
        </CardHeader>
        <CardContent>
          {boardsLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
              <span className="ml-2">{t('settings.boards.loading')}</span>
            </div>
          ) : boards.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <LayoutGrid className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>{t('settings.boards.empty.title')}</p>
              <p className="text-sm">{t('settings.boards.empty.description')}</p>
            </div>
          ) : (
            <div className="space-y-3">
              {boards.map((board) => (
                <div key={board.id} className="border rounded-lg">
                  <div className="flex items-center justify-between p-4 hover:bg-accent/50 transition-colors">
                    <button
                      className="flex items-center gap-2 flex-1 min-w-0 text-left"
                      onClick={() => toggleBoardExpanded(board.id)}
                    >
                      {expandedBoards.has(board.id) ? (
                        <ChevronDown className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-muted-foreground" />
                      )}
                      <LayoutGrid className="h-4 w-4 text-muted-foreground" />
                      <div className="flex-1 min-w-0">
                        <h4 className="font-medium truncate">{board.name}</h4>
                        {board.description && (
                          <p className="text-sm text-muted-foreground truncate">
                            {board.description}
                          </p>
                        )}
                      </div>
                    </button>
                    <div className="flex items-center gap-2 ml-4">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          openEditBoardDialog(board);
                        }}
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          setBoardToDelete(board);
                          setDeleteConfirmOpen(true);
                        }}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </div>
                  </div>
                  {expandedBoards.has(board.id) && (
                    <div className="px-4 pb-4 pt-0 border-t">
                      {/* Columns Section */}
                      <div className="flex items-center justify-between py-3">
                        <h5 className="text-sm font-medium">
                          {t('settings.boards.columns.title')}
                        </h5>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => openCreateColumnDialog(board.id)}
                        >
                          <Plus className="h-3 w-3 mr-1" />
                          {t('settings.boards.columns.add')}
                        </Button>
                      </div>
                      {renderColumns(board.id)}

                      {/* Transitions Section */}
                      <div className="flex items-center justify-between py-3 mt-4 border-t">
                        <h5 className="text-sm font-medium">
                          {t('settings.boards.transitions.title', 'State Transitions')}
                        </h5>
                        <div className="flex items-center gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setTransitionBuilderBoardId(board.id);
                              setTransitionBuilderOpen(true);
                            }}
                            disabled={(columnsMap.get(board.id) || []).length < 2}
                          >
                            <GitBranch className="h-3 w-3 mr-1" />
                            {t('settings.boards.transitions.builder', 'Builder')}
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => openCreateTransitionDialog(board.id)}
                            disabled={(columnsMap.get(board.id) || []).length < 2}
                          >
                            <Plus className="h-3 w-3 mr-1" />
                            {t('settings.boards.transitions.add', 'Add Transition')}
                          </Button>
                        </div>
                      </div>
                      {renderTransitions(board.id)}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Board Create/Edit Dialog */}
      <Dialog open={boardDialogOpen} onOpenChange={setBoardDialogOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {editingBoard
                ? t('settings.boards.dialog.editTitle')
                : t('settings.boards.dialog.createTitle')}
            </DialogTitle>
            <DialogDescription>
              {editingBoard
                ? t('settings.boards.dialog.editDescription')
                : t('settings.boards.dialog.createDescription')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="board-name">
                {t('settings.boards.form.name')} *
              </Label>
              <Input
                id="board-name"
                placeholder={t('settings.boards.form.namePlaceholder')}
                value={boardForm.name}
                onChange={(e) =>
                  setBoardForm({ ...boardForm, name: e.target.value })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="board-description">
                {t('settings.boards.form.description')}
              </Label>
              <Textarea
                id="board-description"
                placeholder={t('settings.boards.form.descriptionPlaceholder')}
                className="min-h-[100px]"
                value={boardForm.description || ''}
                onChange={(e) =>
                  setBoardForm({
                    ...boardForm,
                    description: e.target.value || null,
                  })
                }
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setBoardDialogOpen(false)}
              disabled={boardSaving}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              onClick={handleBoardSave}
              disabled={boardSaving || !boardForm.name}
            >
              {boardSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {editingBoard
                ? t('common:buttons.update')
                : t('common:buttons.create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Board Delete Confirmation Dialog */}
      <Dialog open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('settings.boards.dialog.deleteTitle')}</DialogTitle>
            <DialogDescription>
              {t('settings.boards.dialog.deleteDescription', {
                name: boardToDelete?.name,
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteConfirmOpen(false);
                setBoardToDelete(null);
              }}
              disabled={boardSaving}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              variant="destructive"
              onClick={handleBoardDelete}
              disabled={boardSaving}
            >
              {boardSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('common:buttons.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Column Create/Edit Dialog */}
      <Dialog open={columnDialogOpen} onOpenChange={setColumnDialogOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {editingColumn
                ? t('settings.boards.columns.dialog.editTitle')
                : t('settings.boards.columns.dialog.createTitle')}
            </DialogTitle>
            <DialogDescription>
              {editingColumn
                ? t('settings.boards.columns.dialog.editDescription')
                : t('settings.boards.columns.dialog.createDescription')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="column-name">
                {t('settings.boards.columns.form.name')} *
              </Label>
              <Input
                id="column-name"
                placeholder={t('settings.boards.columns.form.namePlaceholder')}
                value={columnForm.name}
                onChange={(e) => {
                  const name = e.target.value;
                  setBoardForm({ ...boardForm, name });
                  setColumnForm({
                    ...columnForm,
                    name,
                    // Auto-generate slug if not editing
                    slug: editingColumn ? columnForm.slug : generateSlug(name),
                  });
                }}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="column-slug">
                {t('settings.boards.columns.form.slug')} *
              </Label>
              <Input
                id="column-slug"
                placeholder={t('settings.boards.columns.form.slugPlaceholder')}
                value={columnForm.slug}
                onChange={(e) =>
                  setColumnForm({ ...columnForm, slug: e.target.value })
                }
              />
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.columns.form.slugHelper')}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="column-color">
                {t('settings.boards.columns.form.color')}
              </Label>
              <div className="flex items-center gap-2">
                <Input
                  id="column-color"
                  type="color"
                  className="w-12 h-10 p-1 cursor-pointer"
                  value={columnForm.color || '#6b7280'}
                  onChange={(e) =>
                    setColumnForm({ ...columnForm, color: e.target.value })
                  }
                />
                <Input
                  placeholder="#6b7280"
                  value={columnForm.color || ''}
                  onChange={(e) =>
                    setColumnForm({
                      ...columnForm,
                      color: e.target.value || null,
                    })
                  }
                  className="flex-1"
                />
              </div>
            </div>

            <div className="flex items-center gap-6">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="column-initial"
                  checked={columnForm.is_initial || false}
                  onCheckedChange={(checked) =>
                    setColumnForm({
                      ...columnForm,
                      is_initial: checked === true,
                    })
                  }
                />
                <Label
                  htmlFor="column-initial"
                  className="text-sm font-normal cursor-pointer"
                >
                  {t('settings.boards.columns.form.isInitial')}
                </Label>
              </div>

              <div className="flex items-center space-x-2">
                <Checkbox
                  id="column-terminal"
                  checked={columnForm.is_terminal || false}
                  onCheckedChange={(checked) =>
                    setColumnForm({
                      ...columnForm,
                      is_terminal: checked === true,
                    })
                  }
                />
                <Label
                  htmlFor="column-terminal"
                  className="text-sm font-normal cursor-pointer"
                >
                  {t('settings.boards.columns.form.isTerminal')}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="column-starts-workflow"
                  checked={columnForm.starts_workflow || false}
                  onCheckedChange={(checked) =>
                    setColumnForm({
                      ...columnForm,
                      starts_workflow: checked === true,
                    })
                  }
                />
                <Label
                  htmlFor="column-starts-workflow"
                  className="text-sm font-normal cursor-pointer"
                >
                  {t('settings.boards.columns.form.startsWorkflow', 'Starts Workflow (creates attempt)')}
                </Label>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="column-status">
                {t('settings.boards.columns.form.status')} *
              </Label>
              <Select
                value={columnForm.status || 'todo'}
                onValueChange={(value) =>
                  setColumnForm({
                    ...columnForm,
                    status: value as TaskStatus,
                  })
                }
              >
                <SelectTrigger id="column-status">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="todo">
                    {t('settings.boards.columns.status.todo')}
                  </SelectItem>
                  <SelectItem value="inprogress">
                    {t('settings.boards.columns.status.inprogress')}
                  </SelectItem>
                  <SelectItem value="inreview">
                    {t('settings.boards.columns.status.inreview')}
                  </SelectItem>
                  <SelectItem value="done">
                    {t('settings.boards.columns.status.done')}
                  </SelectItem>
                  <SelectItem value="cancelled">
                    {t('settings.boards.columns.status.cancelled')}
                  </SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.columns.form.statusHelper')}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="column-agent">
                {t('settings.boards.columns.form.agent')}
              </Label>
              <Select
                value={columnForm.agent_id || 'none'}
                onValueChange={(value) =>
                  setColumnForm({
                    ...columnForm,
                    agent_id: value === 'none' ? null : value,
                  })
                }
                disabled={agentsLoading}
              >
                <SelectTrigger id="column-agent">
                  <SelectValue placeholder={agentsLoading ? 'Loading agents...' : t('settings.boards.columns.form.agentPlaceholder')} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">
                    {t('settings.boards.columns.form.noAgent')}
                  </SelectItem>
                  {agents.map((agent) => (
                    <SelectItem key={agent.id} value={agent.id}>
                      {agent.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.columns.form.agentHelper')}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="column-deliverable">
                {t('settings.boards.columns.form.deliverable', 'Expected Deliverable')}
              </Label>
              <Textarea
                id="column-deliverable"
                placeholder={t('settings.boards.columns.form.deliverablePlaceholder', 'Describe what should be produced before moving to the next column...')}
                className="min-h-[80px] font-mono text-sm"
                value={columnForm.deliverable || ''}
                onChange={(e) =>
                  setColumnForm({
                    ...columnForm,
                    deliverable: e.target.value || null,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.columns.form.deliverableHelper', 'The agent will be instructed to produce this output before moving on. This helps prevent agents from going beyond their scope.')}
              </p>
            </div>

            {/* Structured Deliverable Options */}
            <div className="space-y-4 border-t pt-4 mt-4">
              <div className="space-y-1">
                <h4 className="text-sm font-medium">
                  {t('settings.boards.columns.form.structuredDeliverable', 'Structured Decision Output')}
                </h4>
                <p className="text-xs text-muted-foreground">
                  {t('settings.boards.columns.form.structuredDeliverableHelper', 'Define a variable and allowed values for the agent to set in .vibe/decision.json. Used for conditional transitions.')}
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="column-deliverable-variable">
                  {t('settings.boards.columns.form.deliverableVariable', 'Variable Name')}
                </Label>
                <Input
                  id="column-deliverable-variable"
                  placeholder={t('settings.boards.columns.form.deliverableVariablePlaceholder', 'e.g., decision, review_outcome')}
                  value={columnForm.deliverable_variable || ''}
                  onChange={(e) =>
                    setColumnForm({
                      ...columnForm,
                      deliverable_variable: e.target.value || null,
                    })
                  }
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="column-deliverable-options">
                  {t('settings.boards.columns.form.deliverableOptions', 'Allowed Values (comma-separated)')}
                </Label>
                <Input
                  id="column-deliverable-options"
                  placeholder={t('settings.boards.columns.form.deliverableOptionsPlaceholder', 'e.g., approve, reject, needs_work')}
                  defaultValue={columnForm.deliverable_options ? JSON.parse(columnForm.deliverable_options).join(', ') : ''}
                  key={editingColumn?.id || 'new'}
                  onBlur={(e) => {
                    const values = e.target.value
                      .split(',')
                      .map((v) => v.trim())
                      .filter((v) => v.length > 0);
                    setColumnForm({
                      ...columnForm,
                      deliverable_options: values.length > 0 ? JSON.stringify(values) : null,
                    });
                  }}
                />
                <p className="text-xs text-muted-foreground">
                  {t('settings.boards.columns.form.deliverableOptionsHelper', 'The agent will be instructed to set the variable to one of these values.')}
                </p>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setColumnDialogOpen(false)}
              disabled={columnSaving}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              onClick={handleColumnSave}
              disabled={columnSaving || !columnForm.name || !columnForm.slug}
            >
              {columnSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {editingColumn
                ? t('common:buttons.update')
                : t('common:buttons.create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Column Delete Confirmation Dialog */}
      <Dialog
        open={deleteColumnConfirmOpen}
        onOpenChange={setDeleteColumnConfirmOpen}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t('settings.boards.columns.dialog.deleteTitle')}
            </DialogTitle>
            <DialogDescription>
              {t('settings.boards.columns.dialog.deleteDescription', {
                name: columnToDelete?.column.name,
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteColumnConfirmOpen(false);
                setColumnToDelete(null);
              }}
              disabled={columnSaving}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              variant="destructive"
              onClick={handleColumnDelete}
              disabled={columnSaving}
            >
              {columnSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('common:buttons.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Transition Create Dialog */}
      <Dialog open={transitionDialogOpen} onOpenChange={setTransitionDialogOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t('settings.boards.transitions.dialog.createTitle', 'Create State Transition')}
            </DialogTitle>
            <DialogDescription>
              {t('settings.boards.transitions.dialog.createDescription', 'Define a routing rule between columns. Projects using this board will inherit these transitions.')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="transition-from">
                {t('settings.boards.transitions.form.fromColumn', 'From Column')} *
              </Label>
              <Select
                value={transitionForm.from_column_id}
                onValueChange={(value) =>
                  setTransitionForm({ ...transitionForm, from_column_id: value })
                }
              >
                <SelectTrigger id="transition-from">
                  <SelectValue placeholder={t('settings.boards.transitions.form.selectColumn', 'Select column')} />
                </SelectTrigger>
                <SelectContent>
                  {(columnsMap.get(transitionBoardId || '') || []).map((col) => (
                    <SelectItem key={col.id} value={col.id}>
                      {col.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="transition-to">
                {t('settings.boards.transitions.form.toColumn', 'To Column')} *
              </Label>
              <Select
                value={transitionForm.to_column_id}
                onValueChange={(value) =>
                  setTransitionForm({ ...transitionForm, to_column_id: value })
                }
              >
                <SelectTrigger id="transition-to">
                  <SelectValue placeholder={t('settings.boards.transitions.form.selectColumn', 'Select column')} />
                </SelectTrigger>
                <SelectContent>
                  {(columnsMap.get(transitionBoardId || '') || [])
                    .filter((col) => col.id !== transitionForm.from_column_id)
                    .map((col) => (
                      <SelectItem key={col.id} value={col.id}>
                        {col.name}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="transition-name">
                {t('settings.boards.transitions.form.name', 'Name (optional)')}
              </Label>
              <Input
                id="transition-name"
                placeholder={t('settings.boards.transitions.form.namePlaceholder', 'e.g., "Approve", "Reject"')}
                value={transitionForm.name || ''}
                onChange={(e) =>
                  setTransitionForm({
                    ...transitionForm,
                    name: e.target.value || null,
                  })
                }
              />
            </div>

            <div className="space-y-2">
              <Label>{t('settings.boards.transitions.form.conditionTitle', 'Condition (optional)')}</Label>
              <p className="text-xs text-muted-foreground mb-2">
                {t('settings.boards.transitions.form.conditionHelp', 'Route based on decision file value (e.g., decision=approve)')}
              </p>
              <div className="flex gap-2">
                <Input
                  placeholder={t('settings.boards.transitions.form.conditionKey', 'key')}
                  value={transitionForm.condition_key || ''}
                  onChange={(e) =>
                    setTransitionForm({
                      ...transitionForm,
                      condition_key: e.target.value || null,
                    })
                  }
                  className="flex-1"
                />
                <span className="flex items-center text-muted-foreground">=</span>
                <Input
                  placeholder={t('settings.boards.transitions.form.conditionValue', 'value')}
                  value={transitionForm.condition_value || ''}
                  onChange={(e) =>
                    setTransitionForm({
                      ...transitionForm,
                      condition_value: e.target.value || null,
                    })
                  }
                  className="flex-1"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="transition-else">
                {t('settings.boards.transitions.form.elseColumn', 'Else Column (on condition failure)')}
              </Label>
              <Select
                value={transitionForm.else_column_id || 'none'}
                onValueChange={(value) =>
                  setTransitionForm({ ...transitionForm, else_column_id: value === 'none' ? null : value })
                }
              >
                <SelectTrigger id="transition-else">
                  <SelectValue placeholder={t('settings.boards.transitions.form.selectColumn', 'Select column')} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">{t('settings.boards.transitions.form.noneOptional', '(none)')}</SelectItem>
                  {(columnsMap.get(transitionBoardId || '') || [])
                    .filter((col) => col.id !== transitionForm.from_column_id && col.id !== transitionForm.to_column_id)
                    .map((col) => (
                      <SelectItem key={col.id} value={col.id}>
                        {col.name}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.transitions.form.elseHelp', 'Where to go when condition doesn\'t match')}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="transition-escalation">
                {t('settings.boards.transitions.form.escalationColumn', 'Escalation Column')}
              </Label>
              <Select
                value={transitionForm.escalation_column_id || 'none'}
                onValueChange={(value) =>
                  setTransitionForm({ ...transitionForm, escalation_column_id: value === 'none' ? null : value })
                }
              >
                <SelectTrigger id="transition-escalation">
                  <SelectValue placeholder={t('settings.boards.transitions.form.selectColumn', 'Select column')} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">{t('settings.boards.transitions.form.noneOptional', '(none)')}</SelectItem>
                  {(columnsMap.get(transitionBoardId || '') || [])
                    .filter((col) => col.id !== transitionForm.from_column_id)
                    .map((col) => (
                      <SelectItem key={col.id} value={col.id}>
                        {col.name}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.transitions.form.escalationHelp', 'Where to go when max failures is reached')}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="transition-max-failures">
                {t('settings.boards.transitions.form.maxFailures', 'Max Failures (before escalation)')}
              </Label>
              <Input
                id="transition-max-failures"
                type="number"
                min="1"
                placeholder={t('settings.boards.transitions.form.maxFailuresPlaceholder', 'e.g., 3')}
                value={transitionForm.max_failures ? String(transitionForm.max_failures) : ''}
                onChange={(e) =>
                  setTransitionForm({
                    ...transitionForm,
                    // Use number, not BigInt - BigInt can't be JSON serialized
                    max_failures: e.target.value ? parseInt(e.target.value, 10) : null,
                  } as CreateStateTransition)
                }
              />
              <p className="text-xs text-muted-foreground">
                {t('settings.boards.transitions.form.maxFailuresHelp', 'After this many trips through the else path, escalate instead')}
              </p>
            </div>

            <div className="flex items-center space-x-2">
              <Checkbox
                id="transition-requires-confirmation"
                checked={transitionForm.requires_confirmation || false}
                onCheckedChange={(checked) =>
                  setTransitionForm({
                    ...transitionForm,
                    requires_confirmation: checked === true,
                  })
                }
              />
              <Label
                htmlFor="transition-requires-confirmation"
                className="text-sm font-normal cursor-pointer"
              >
                {t('settings.boards.transitions.form.requiresConfirmation', 'Requires confirmation')}
              </Label>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setTransitionDialogOpen(false)}
              disabled={transitionSaving}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              onClick={handleTransitionSave}
              disabled={
                transitionSaving ||
                !transitionForm.from_column_id ||
                !transitionForm.to_column_id ||
                transitionForm.from_column_id === transitionForm.to_column_id
              }
            >
              {transitionSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('common:buttons.create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Transition Delete Confirmation Dialog */}
      <Dialog
        open={deleteTransitionConfirmOpen}
        onOpenChange={setDeleteTransitionConfirmOpen}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t('settings.boards.transitions.dialog.deleteTitle', 'Delete Transition')}
            </DialogTitle>
            <DialogDescription>
              {t('settings.boards.transitions.dialog.deleteDescription', 'Are you sure you want to delete this transition? This may affect workflow routing for projects using this board.')}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteTransitionConfirmOpen(false);
                setTransitionToDelete(null);
              }}
              disabled={transitionSaving}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              variant="destructive"
              onClick={handleTransitionDelete}
              disabled={transitionSaving}
            >
              {transitionSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('common:buttons.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Transition Builder Dialog */}
      {transitionBuilderBoardId && (
        <TransitionBuilderDialog
          open={transitionBuilderOpen}
          onOpenChange={setTransitionBuilderOpen}
          boardId={transitionBuilderBoardId}
          columns={columnsMap.get(transitionBuilderBoardId) || []}
          onSuccess={() => {
            setSuccessMessage(t('settings.boards.transitions.save.createSuccess', 'Transitions created successfully'));
            fetchTransitions(transitionBuilderBoardId);
            setTimeout(() => setSuccessMessage(null), 3000);
          }}
        />
      )}
    </div>
  );
}
