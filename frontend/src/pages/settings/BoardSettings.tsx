import { useEffect, useState, useCallback } from 'react';
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
} from 'lucide-react';
import { boardsApi } from '@/lib/api';
import type {
  Board,
  CreateBoard,
  UpdateBoard,
  KanbanColumn,
  CreateKanbanColumn,
  UpdateKanbanColumn,
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
  });
  const [deleteColumnConfirmOpen, setDeleteColumnConfirmOpen] = useState(false);
  const [columnToDelete, setColumnToDelete] = useState<{
    boardId: string;
    column: KanbanColumn;
  } | null>(null);

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

  // Load boards on mount
  useEffect(() => {
    fetchBoards();
  }, [fetchBoards]);

  // Toggle board expansion
  const toggleBoardExpanded = (boardId: string) => {
    setExpandedBoards((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(boardId)) {
        newSet.delete(boardId);
      } else {
        newSet.add(boardId);
        // Fetch columns if not already loaded
        if (!columnsMap.has(boardId)) {
          fetchColumns(boardId);
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
      <div className="space-y-2">
        {columns.map((column) => (
          <div
            key={column.id}
            className="flex items-center gap-2 p-2 border rounded bg-muted/30"
          >
            <GripVertical className="h-4 w-4 text-muted-foreground cursor-grab" />
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
    </div>
  );
}
