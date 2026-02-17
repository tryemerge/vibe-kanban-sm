import { useState } from 'react';
import { useProject } from '@/contexts/ProjectContext';
import { useContextArtifacts } from '@/hooks';
import { contextArtifactsApi } from '@/lib/api';
import { useQueryClient } from '@tanstack/react-query';
import { contextArtifactsKeys } from '@/hooks/useContextArtifacts';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Loader2, Trash2, ChevronDown, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ContextArtifact } from 'shared/types';

const TYPE_OPTIONS = [
  { value: 'all', label: 'All Types' },
  { value: 'adr', label: 'ADR' },
  { value: 'pattern', label: 'Pattern' },
  { value: 'iplan', label: 'Implementation Plan' },
  { value: 'module_memory', label: 'Module Memory' },
  { value: 'decision', label: 'Decision' },
  { value: 'dependency', label: 'Dependency' },
  { value: 'changelog_entry', label: 'Changelog Entry' },
];

const SCOPE_OPTIONS = [
  { value: 'all', label: 'All Scopes' },
  { value: 'global', label: 'Global' },
  { value: 'task', label: 'Task' },
  { value: 'path', label: 'Path' },
];

const TYPE_COLORS: Record<string, string> = {
  adr: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  pattern: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  iplan: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
  module_memory: 'bg-green-500/10 text-green-600 border-green-500/20',
  decision: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  dependency: 'bg-slate-500/10 text-slate-600 border-slate-500/20',
  changelog_entry: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
};

const SCOPE_COLORS: Record<string, string> = {
  global: 'bg-emerald-500/10 text-emerald-600 border-emerald-500/20',
  task: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
  path: 'bg-sky-500/10 text-sky-600 border-sky-500/20',
};

function ArtifactRow({
  artifact,
  onDelete,
}: {
  artifact: ContextArtifact;
  onDelete: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border rounded-lg">
      <div className="flex items-center gap-2 p-3">
        <button
          onClick={() => setExpanded(!expanded)}
          className="flex items-center gap-2 flex-1 min-w-0 text-left hover:bg-accent/50 rounded transition-colors"
        >
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
          )}
          <Badge
            variant="outline"
            className={cn('text-[10px] px-1.5 py-0 flex-shrink-0', TYPE_COLORS[artifact.artifact_type] || '')}
          >
            {artifact.artifact_type}
          </Badge>
          <Badge
            variant="outline"
            className={cn('text-[10px] px-1.5 py-0 flex-shrink-0', SCOPE_COLORS[artifact.scope] || '')}
          >
            {artifact.scope}
          </Badge>
          <span className="text-sm font-medium truncate flex-1">
            {artifact.title}
          </span>
          <span className="text-xs text-muted-foreground flex-shrink-0">
            ~{artifact.token_estimate} tok
          </span>
          <span className="text-xs text-muted-foreground flex-shrink-0">
            v{artifact.version}
          </span>
        </button>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 w-7 p-0 flex-shrink-0"
          onClick={() => onDelete(artifact.id)}
        >
          <Trash2 className="h-3.5 w-3.5 text-destructive" />
        </Button>
      </div>
      {expanded && (
        <div className="px-3 pb-3 border-t">
          <pre className="mt-2 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/50 rounded p-2 max-h-64 overflow-auto">
            {artifact.content}
          </pre>
          <div className="mt-1 text-[10px] text-muted-foreground">
            {artifact.path && <>{artifact.path} &middot; </>}
            {new Date(artifact.created_at).toLocaleDateString()}
            {artifact.updated_at !== artifact.created_at && (
              <> &middot; updated {new Date(artifact.updated_at).toLocaleDateString()}</>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export function KnowledgeBaseSettings() {
  const { projectId } = useProject();
  const queryClient = useQueryClient();
  const [typeFilter, setTypeFilter] = useState('all');
  const [scopeFilter, setScopeFilter] = useState('all');

  const { data: allArtifacts = [], isLoading } = useContextArtifacts(
    projectId,
    typeFilter === 'all' ? undefined : typeFilter
  );

  const filtered = allArtifacts.filter((a) => {
    if (scopeFilter !== 'all' && a.scope !== scopeFilter) return false;
    return true;
  });

  const totalTokens = filtered.reduce((sum, a) => sum + (a.token_estimate || 0), 0);

  const handleDelete = async (artifactId: string) => {
    try {
      await contextArtifactsApi.delete(artifactId);
      if (projectId) {
        queryClient.invalidateQueries({
          queryKey: contextArtifactsKeys.all,
        });
      }
    } catch (err) {
      console.error('Failed to delete artifact:', err);
    }
  };

  if (!projectId) {
    return (
      <div className="text-muted-foreground text-sm">
        Select a project to view its knowledge base.
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Knowledge Base</CardTitle>
          <CardDescription>
            Context artifacts that are injected into agent prompts. Agents create these
            during workflow execution to build shared project knowledge.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Filters + summary */}
          <div className="flex items-center gap-3 flex-wrap">
            <Select value={typeFilter} onValueChange={setTypeFilter}>
              <SelectTrigger className="w-[180px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select value={scopeFilter} onValueChange={setScopeFilter}>
              <SelectTrigger className="w-[140px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {SCOPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <span className="text-sm text-muted-foreground ml-auto">
              {filtered.length} artifact{filtered.length !== 1 ? 's' : ''} &middot; ~{totalTokens.toLocaleString()} tokens
            </span>
          </div>

          {/* Artifact list */}
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : filtered.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground text-sm">
              {allArtifacts.length === 0
                ? 'No context artifacts yet. Artifacts are created by agents during workflow execution.'
                : 'No artifacts match the current filters.'}
            </div>
          ) : (
            <div className="space-y-2">
              {filtered.map((artifact) => (
                <ArtifactRow
                  key={artifact.id}
                  artifact={artifact}
                  onDelete={handleDelete}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
