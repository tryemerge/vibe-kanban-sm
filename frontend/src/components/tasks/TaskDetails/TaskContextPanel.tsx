import { useState } from 'react';
import { useContextArtifacts, useContextPreview } from '@/hooks';
import { Badge } from '@/components/ui/badge';
import { Loader2, ChevronDown, ChevronRight, Brain, FileText } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ContextArtifact } from 'shared/types';

interface TaskContextPanelProps {
  taskId: string;
  projectId: string;
  className?: string;
}

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

function ArtifactItem({ artifact }: { artifact: ContextArtifact }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border rounded-lg">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 p-3 text-left hover:bg-accent/50 transition-colors"
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <Badge
              variant="outline"
              className={cn('text-[10px] px-1.5 py-0', TYPE_COLORS[artifact.artifact_type] || '')}
            >
              {artifact.artifact_type}
            </Badge>
            <Badge
              variant="outline"
              className={cn('text-[10px] px-1.5 py-0', SCOPE_COLORS[artifact.scope] || '')}
            >
              {artifact.scope}
            </Badge>
            <span className="text-sm font-medium truncate">
              {artifact.title}
            </span>
          </div>
        </div>
        <span className="text-xs text-muted-foreground flex-shrink-0">
          ~{artifact.token_estimate} tok
        </span>
      </button>
      {expanded && (
        <div className="px-3 pb-3 border-t">
          <pre className="mt-2 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/50 rounded p-2 max-h-48 overflow-auto">
            {artifact.content}
          </pre>
          <div className="mt-1 text-[10px] text-muted-foreground">
            v{artifact.version}
            {artifact.path && <> &middot; {artifact.path}</>}
            {' '}&middot; {new Date(artifact.created_at).toLocaleDateString()}
          </div>
        </div>
      )}
    </div>
  );
}

export function TaskContextPanel({
  taskId,
  projectId,
  className,
}: TaskContextPanelProps) {
  const { data: allArtifacts, isLoading: artifactsLoading } =
    useContextArtifacts(projectId);
  const { data: preview, isLoading: previewLoading } =
    useContextPreview(projectId, taskId);

  const taskArtifacts = (allArtifacts || []).filter(
    (a) => a.source_task_id === taskId
  );

  if (artifactsLoading && previewLoading) {
    return (
      <div className={cn('flex items-center justify-center py-8', className)}>
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className={cn('space-y-6', className)}>
      {/* Section A: Artifacts produced by this task */}
      <div>
        <div className="flex items-center gap-2 mb-3">
          <FileText className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">Produced by this task</h3>
          <span className="text-xs text-muted-foreground">
            ({taskArtifacts.length})
          </span>
        </div>
        {taskArtifacts.length === 0 ? (
          <p className="text-xs text-muted-foreground italic pl-6">
            This task hasn't produced any artifacts yet.
          </p>
        ) : (
          <div className="space-y-2">
            {taskArtifacts.map((artifact) => (
              <ArtifactItem key={artifact.id} artifact={artifact} />
            ))}
          </div>
        )}
      </div>

      {/* Section B: Context that would be injected */}
      <div>
        <div className="flex items-center gap-2 mb-3">
          <Brain className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">Context injected</h3>
        </div>
        {previewLoading ? (
          <div className="flex items-center gap-2 text-xs text-muted-foreground pl-6">
            <Loader2 className="h-3 w-3 animate-spin" />
            Loading preview...
          </div>
        ) : preview ? (
          <div className="space-y-2">
            {/* Token budget bar */}
            <div className="flex items-center gap-3 pl-6">
              <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary rounded-full transition-all"
                  style={{
                    width: `${Math.min(100, (preview.tokens_used / preview.token_budget) * 100)}%`,
                  }}
                />
              </div>
              <span className="text-xs text-muted-foreground whitespace-nowrap">
                {preview.tokens_used.toLocaleString()} / {preview.token_budget.toLocaleString()} tokens
              </span>
            </div>
            <div className="text-[10px] text-muted-foreground pl-6">
              {preview.artifacts_included} of {preview.artifacts_total} artifacts included
            </div>
            {preview.context ? (
              <pre className="text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/50 rounded p-3 max-h-64 overflow-auto">
                {preview.context}
              </pre>
            ) : (
              <p className="text-xs text-muted-foreground italic pl-6">
                No context artifacts exist for this project yet.
              </p>
            )}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground italic pl-6">
            No context artifacts exist for this project yet.
          </p>
        )}
      </div>
    </div>
  );
}
