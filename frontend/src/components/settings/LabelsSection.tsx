import { useState, useEffect, useCallback } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Loader2, Plus, Pencil, Trash2, GripVertical, Tag } from 'lucide-react';
import { labelsApi } from '@/lib/api';
import type { TaskLabel } from 'shared/types';

// Predefined color palette for labels
const LABEL_COLORS = [
  '#ef4444', // red
  '#f97316', // orange
  '#eab308', // yellow
  '#22c55e', // green
  '#14b8a6', // teal
  '#3b82f6', // blue
  '#8b5cf6', // purple
  '#ec4899', // pink
  '#6b7280', // gray
];

interface LabelsSectionProps {
  projectId: string;
}

interface LabelFormState {
  name: string;
  color: string;
}

export function LabelsSection({ projectId }: LabelsSectionProps) {
  const [labels, setLabels] = useState<TaskLabel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Dialog state
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingLabel, setEditingLabel] = useState<TaskLabel | null>(null);
  const [formState, setFormState] = useState<LabelFormState>({
    name: '',
    color: LABEL_COLORS[0],
  });
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  // Fetch labels
  const fetchLabels = useCallback(async () => {
    if (!projectId) return;

    setLoading(true);
    setError(null);
    try {
      const data = await labelsApi.listByProject(projectId);
      setLabels(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load labels');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    fetchLabels();
  }, [fetchLabels]);

  // Open dialog for creating new label
  const handleCreate = () => {
    setEditingLabel(null);
    setFormState({ name: '', color: LABEL_COLORS[0] });
    setDialogOpen(true);
  };

  // Open dialog for editing label
  const handleEdit = (label: TaskLabel) => {
    setEditingLabel(label);
    setFormState({
      name: label.name,
      color: label.color || LABEL_COLORS[0],
    });
    setDialogOpen(true);
  };

  // Save label (create or update)
  const handleSave = async () => {
    if (!formState.name.trim()) return;

    setSaving(true);
    setError(null);
    try {
      if (editingLabel) {
        // Update existing label
        const updated = await labelsApi.update(projectId, editingLabel.id, {
          name: formState.name.trim(),
          color: formState.color,
          position: null,
        });
        setLabels((prev) =>
          prev.map((l) => (l.id === updated.id ? updated : l))
        );
      } else {
        // Create new label
        const created = await labelsApi.create(projectId, {
          name: formState.name.trim(),
          color: formState.color,
          position: labels.length,
        });
        setLabels((prev) => [...prev, created]);
      }
      setDialogOpen(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save label');
    } finally {
      setSaving(false);
    }
  };

  // Delete label
  const handleDelete = async (labelId: string) => {
    if (!window.confirm('Are you sure you want to delete this label? It will be removed from all tasks.')) {
      return;
    }

    setDeleting(labelId);
    setError(null);
    try {
      await labelsApi.delete(projectId, labelId);
      setLabels((prev) => prev.filter((l) => l.id !== labelId));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete label');
    } finally {
      setDeleting(null);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Tag className="h-5 w-5" />
          Labels
        </CardTitle>
        <CardDescription>
          Create labels to organize and filter tasks in this project
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {loading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-5 w-5 animate-spin" />
            <span className="ml-2 text-sm text-muted-foreground">
              Loading labels...
            </span>
          </div>
        ) : (
          <div className="space-y-2">
            {labels.map((label) => (
              <div
                key={label.id}
                className="flex items-center justify-between p-3 border rounded-md group"
              >
                <div className="flex items-center gap-3">
                  <GripVertical className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 cursor-grab" />
                  <div
                    className="w-4 h-4 rounded-full flex-shrink-0"
                    style={{ backgroundColor: label.color || '#6b7280' }}
                  />
                  <span className="font-medium">{label.name}</span>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleEdit(label)}
                    title="Edit label"
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleDelete(label.id)}
                    disabled={deleting === label.id}
                    title="Delete label"
                  >
                    {deleting === label.id ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Trash2 className="h-4 w-4" />
                    )}
                  </Button>
                </div>
              </div>
            ))}

            {labels.length === 0 && !loading && (
              <div className="text-center py-4 text-sm text-muted-foreground">
                No labels created yet
              </div>
            )}

            <Button
              variant="outline"
              size="sm"
              onClick={handleCreate}
              className="w-full"
            >
              <Plus className="h-4 w-4 mr-2" />
              Add Label
            </Button>
          </div>
        )}

        {/* Create/Edit Dialog */}
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {editingLabel ? 'Edit Label' : 'Create Label'}
              </DialogTitle>
              <DialogDescription>
                {editingLabel
                  ? 'Update the label name and color'
                  : 'Create a new label to organize tasks'}
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="label-name">Name</Label>
                <Input
                  id="label-name"
                  value={formState.name}
                  onChange={(e) =>
                    setFormState((prev) => ({ ...prev, name: e.target.value }))
                  }
                  placeholder="e.g., Bug, Feature, Enhancement"
                  autoFocus
                />
              </div>
              <div className="space-y-2">
                <Label>Color</Label>
                <div className="flex flex-wrap gap-2">
                  {LABEL_COLORS.map((color) => (
                    <button
                      key={color}
                      type="button"
                      className={`w-8 h-8 rounded-full border-2 transition-all ${
                        formState.color === color
                          ? 'border-foreground scale-110'
                          : 'border-transparent hover:scale-105'
                      }`}
                      style={{ backgroundColor: color }}
                      onClick={() =>
                        setFormState((prev) => ({ ...prev, color }))
                      }
                    />
                  ))}
                </div>
              </div>
              {/* Preview */}
              <div className="space-y-2">
                <Label>Preview</Label>
                <div className="flex items-center gap-2 p-2 border rounded-md bg-muted/50">
                  <div
                    className="w-3 h-3 rounded-full flex-shrink-0"
                    style={{ backgroundColor: formState.color }}
                  />
                  <span className="text-sm font-medium">
                    {formState.name || 'Label name'}
                  </span>
                </div>
              </div>
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setDialogOpen(false)}
                disabled={saving}
              >
                Cancel
              </Button>
              <Button
                onClick={handleSave}
                disabled={saving || !formState.name.trim()}
              >
                {saving && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                {editingLabel ? 'Save Changes' : 'Create Label'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
