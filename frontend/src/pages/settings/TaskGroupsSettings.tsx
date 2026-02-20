import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { taskGroupsApi } from '@/lib/api';
import { useProjects } from '@/hooks/useProjects';
import {
  useTaskGroupMutations,
  useTaskGroupDependencies,
  taskGroupsKeys,
} from '@/hooks/useTaskGroups';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Plus,
  Trash2,
  Lock,
  Pencil,
  GripVertical,
  ArrowRight,
  Loader2,
} from 'lucide-react';
import type { TaskGroup, Project } from 'shared/types';

const GROUP_COLORS = [
  '#3b82f6', // blue
  '#10b981', // emerald
  '#f59e0b', // amber
  '#ef4444', // red
  '#8b5cf6', // violet
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#f97316', // orange
];

export function TaskGroupsSettings() {
  const { projects } = useProjects();

  const [selectedProjectId, setSelectedProjectId] = useState<string>('');

  // Auto-select first project
  const projectId = selectedProjectId || projects[0]?.id || '';

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Task Groups</CardTitle>
          <CardDescription>
            Group related tasks for sequential execution. Tasks within a group
            automatically depend on the previous task.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Select value={projectId} onValueChange={setSelectedProjectId}>
            <SelectTrigger className="w-64">
              <SelectValue placeholder="Select a project" />
            </SelectTrigger>
            <SelectContent>
              {projects.map((p: Project) => (
                <SelectItem key={p.id} value={p.id}>
                  {p.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {projectId && <TaskGroupsList projectId={projectId} />}
        </CardContent>
      </Card>
    </div>
  );
}

function TaskGroupsList({ projectId }: { projectId: string }) {
  const { data: groups = [], isLoading } = useQuery({
    queryKey: taskGroupsKeys.byProject(projectId),
    queryFn: () => taskGroupsApi.list(projectId),
    enabled: !!projectId,
  });

  const { data: dependencies = [] } = useTaskGroupDependencies(projectId);

  const {
    createGroup,
    updateGroup,
    deleteGroup,
    addDependency,
    removeDependency,
  } = useTaskGroupMutations(projectId);

  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupColor, setNewGroupColor] = useState(GROUP_COLORS[0]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');

  const handleCreate = () => {
    if (!newGroupName.trim()) return;
    createGroup.mutate(
      { name: newGroupName.trim(), color: newGroupColor },
      {
        onSuccess: () => {
          setNewGroupName('');
          setNewGroupColor(
            GROUP_COLORS[(groups.length + 1) % GROUP_COLORS.length]
          );
        },
      }
    );
  };

  const handleStartEdit = (group: TaskGroup) => {
    setEditingId(group.id);
    setEditName(group.name);
  };

  const handleSaveEdit = (groupId: string) => {
    if (!editName.trim()) return;
    updateGroup.mutate(
      { groupId, data: { name: editName.trim(), color: null } },
      { onSuccess: () => setEditingId(null) }
    );
  };

  const handleDelete = (groupId: string) => {
    deleteGroup.mutate(groupId);
  };

  const handleAddDependency = (groupId: string, dependsOnGroupId: string) => {
    addDependency.mutate({ groupId, dependsOnGroupId });
  };

  const handleRemoveDependency = (depId: string) => {
    removeDependency.mutate(depId);
  };

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        Loading task groups...
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Create new group */}
      <div className="flex items-center gap-2">
        <div
          className="w-6 h-6 rounded shrink-0 cursor-pointer border"
          style={{ backgroundColor: newGroupColor }}
          onClick={() => {
            const idx = GROUP_COLORS.indexOf(newGroupColor);
            setNewGroupColor(
              GROUP_COLORS[(idx + 1) % GROUP_COLORS.length]
            );
          }}
        />
        <Input
          placeholder="New group name..."
          value={newGroupName}
          onChange={(e) => setNewGroupName(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
          className="max-w-xs"
        />
        <Button
          size="sm"
          onClick={handleCreate}
          disabled={!newGroupName.trim() || createGroup.isPending}
        >
          <Plus className="h-4 w-4 mr-1" />
          Add Group
        </Button>
      </div>

      {/* Group list */}
      {groups.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No task groups yet. Create one to start grouping related tasks.
        </p>
      ) : (
        <div className="space-y-2">
          {groups.map((group) => {
            const isStarted = !!group.started_at;
            const groupDeps = dependencies.filter(
              (d) => d.task_group_id === group.id
            );

            return (
              <div
                key={group.id}
                className="flex items-center gap-3 p-3 rounded-lg border bg-card"
              >
                <GripVertical className="h-4 w-4 text-muted-foreground shrink-0" />
                <div
                  className="w-4 h-4 rounded shrink-0"
                  style={{
                    backgroundColor: group.color || '#6b7280',
                  }}
                />

                {editingId === group.id ? (
                  <Input
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleSaveEdit(group.id);
                      if (e.key === 'Escape') setEditingId(null);
                    }}
                    onBlur={() => handleSaveEdit(group.id)}
                    autoFocus
                    className="max-w-xs h-7"
                  />
                ) : (
                  <span className="font-medium text-sm">{group.name}</span>
                )}

                {isStarted && (
                  <Badge
                    variant="secondary"
                    className="gap-1 text-xs shrink-0"
                  >
                    <Lock className="h-3 w-3" />
                    Started
                  </Badge>
                )}

                {/* Dependency display */}
                {groupDeps.length > 0 && (
                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    <ArrowRight className="h-3 w-3" />
                    depends on:{' '}
                    {groupDeps.map((dep) => {
                      const prereq = groups.find(
                        (g) => g.id === dep.depends_on_group_id
                      );
                      return (
                        <Badge
                          key={dep.id}
                          variant="outline"
                          className="text-xs cursor-pointer hover:bg-destructive/10"
                          onClick={() => handleRemoveDependency(dep.id)}
                          title="Click to remove dependency"
                        >
                          {prereq?.name ?? 'Unknown'}
                        </Badge>
                      );
                    })}
                  </div>
                )}

                <div className="flex-1" />

                {/* Add dependency dropdown */}
                {!isStarted && groups.length > 1 && (
                  <Select
                    onValueChange={(value) =>
                      handleAddDependency(group.id, value)
                    }
                  >
                    <SelectTrigger className="w-36 h-7 text-xs">
                      <SelectValue placeholder="Depends on..." />
                    </SelectTrigger>
                    <SelectContent>
                      {groups
                        .filter(
                          (g) =>
                            g.id !== group.id &&
                            !groupDeps.some(
                              (d) => d.depends_on_group_id === g.id
                            )
                        )
                        .map((g) => (
                          <SelectItem key={g.id} value={g.id}>
                            {g.name}
                          </SelectItem>
                        ))}
                    </SelectContent>
                  </Select>
                )}

                {!isStarted && (
                  <>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => handleStartEdit(group)}
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-destructive hover:text-destructive"
                      onClick={() => handleDelete(group.id)}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
