import { memo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Sparkles, BookOpen, FileText, FilePlus2, ChevronDown, ChevronRight, AlertTriangle, Scroll } from 'lucide-react';
import { contextArtifactsApi } from '@/lib/api';
import type { ContextArtifact, TaskGroup } from 'shared/types';

interface PlansSidebarProps {
  projectId: string;
  taskGroups: TaskGroup[];
  onCreateGroupFromPlan?: (plan: ContextArtifact) => void;
  onViewArtifact?: (artifactId: string) => void;
  onApproveBrief?: (brief: ContextArtifact) => void;
}

interface Plan {
  impl: ContextArtifact;
  adr: ContextArtifact;
  brief?: ContextArtifact;
}

function BriefCard({
  brief,
  onViewArtifact,
  onApprove,
}: {
  brief: ContextArtifact;
  onViewArtifact?: (artifactId: string) => void;
  onApprove?: (brief: ContextArtifact) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <Card
      className="border-2 transition-all hover:shadow-md cursor-pointer"
      style={{ borderColor: '#f97316aa', backgroundColor: '#fff7ed' }}
      onClick={() => onViewArtifact?.(brief.id)}
    >
      <CardHeader className="py-2 px-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <CardTitle className="text-xs font-semibold line-clamp-2 leading-snug text-orange-900">
              {brief.title}
            </CardTitle>
            <div className="flex items-center gap-1 mt-1">
              <Badge variant="outline" className="text-[9px] h-4 px-1 gap-0.5 border-orange-400 text-orange-700">
                <AlertTriangle className="h-2.5 w-2.5" />
                BRIEF
              </Badge>
            </div>
          </div>
          <button
            className="shrink-0 text-orange-400 hover:text-orange-600 transition-colors mt-0.5"
            onClick={(e) => { e.stopPropagation(); setExpanded(x => !x); }}
            title={expanded ? 'Collapse' : 'Expand'}
          >
            {expanded
              ? <ChevronDown className="h-3.5 w-3.5" />
              : <ChevronRight className="h-3.5 w-3.5" />
            }
          </button>
        </div>
      </CardHeader>

      {expanded && (
        <CardContent className="px-3 pb-2 pt-0">
          <p className="text-[10px] text-orange-800 leading-relaxed line-clamp-6 whitespace-pre-line mb-2">
            {brief.content}
          </p>
        </CardContent>
      )}

      {onApprove && (
        <CardContent className="px-3 pb-2 pt-0">
          <Button
            size="sm"
            variant="outline"
            className="w-full h-6 text-[10px] gap-1 border-orange-300 text-orange-700 hover:bg-orange-50"
            onClick={(e) => { e.stopPropagation(); onApprove(brief); }}
          >
            <Sparkles className="h-3 w-3" />
            Plan with Agent
          </Button>
        </CardContent>
      )}
    </Card>
  );
}

function PlanCard({
  plan,
  onCreateGroup,
  onViewArtifact,
  onViewBrief,
}: {
  plan: Plan;
  onCreateGroup?: (impl: ContextArtifact) => void;
  onViewArtifact?: (artifactId: string) => void;
  onViewBrief?: (briefId: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <Card className="border-2 transition-all hover:shadow-md" style={{ borderColor: '#7c3aed22' }}>
      <CardHeader className="py-2 px-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <CardTitle className="text-xs font-semibold line-clamp-2 leading-snug">
              {plan.impl.title}
            </CardTitle>
            <div className="flex items-center gap-1 mt-1 flex-wrap">
              {plan.brief && (
                <Badge
                  variant="outline"
                  className="text-[9px] h-4 px-1 gap-0.5 border-orange-300 text-orange-600 cursor-pointer hover:bg-orange-50"
                  onClick={() => (onViewBrief ?? onViewArtifact)?.(plan.brief!.id)}
                  title="View Brief"
                >
                  <Scroll className="h-2.5 w-2.5" />
                  BRIEF
                </Badge>
              )}
              <Badge
                variant="outline"
                className="text-[9px] h-4 px-1 gap-0.5 border-amber-300 text-amber-600 cursor-pointer hover:bg-amber-50"
                onClick={() => onViewArtifact?.(plan.adr.id)}
                title="View ADR"
              >
                <BookOpen className="h-2.5 w-2.5" />
                ADR
              </Badge>
              <Badge
                variant="outline"
                className="text-[9px] h-4 px-1 gap-0.5 border-blue-300 text-blue-600 cursor-pointer hover:bg-blue-50"
                onClick={() => onViewArtifact?.(plan.impl.id)}
                title="View Plan"
              >
                <FileText className="h-2.5 w-2.5" />
                IMPL
              </Badge>
            </div>
          </div>
          <button
            className="shrink-0 text-muted-foreground hover:text-foreground transition-colors mt-0.5"
            onClick={() => setExpanded(e => !e)}
            title={expanded ? 'Collapse' : 'Expand'}
          >
            {expanded
              ? <ChevronDown className="h-3.5 w-3.5" />
              : <ChevronRight className="h-3.5 w-3.5" />
            }
          </button>
        </div>
      </CardHeader>

      {expanded && (
        <CardContent className="px-3 pb-2 pt-0">
          <p className="text-[10px] text-muted-foreground leading-relaxed line-clamp-4 whitespace-pre-line mb-2">
            {plan.impl.content}
          </p>
        </CardContent>
      )}

      {onCreateGroup && (
        <CardContent className="px-3 pb-2 pt-0">
          <Button
            size="sm"
            variant="outline"
            className="w-full h-6 text-[10px] gap-1"
            onClick={() => onCreateGroup(plan.impl)}
          >
            <Sparkles className="h-3 w-3" />
            Create Group
          </Button>
        </CardContent>
      )}
    </Card>
  );
}

function PlansSidebarComponent({
  projectId,
  taskGroups,
  onCreateGroupFromPlan,
  onViewArtifact,
  onApproveBrief,
}: PlansSidebarProps) {
  const [uploading, setUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const { data: artifacts = [] } = useQuery({
    queryKey: ['projectArtifacts', projectId],
    queryFn: () => contextArtifactsApi.list(projectId!),
    enabled: !!projectId,
    refetchInterval: 5000,
  });

  // Find which artifact IDs are already used by a task group
  const usedArtifactIds = new Set(
    taskGroups.map(g => g.artifact_id).filter(Boolean)
  );

  // All iplan artifacts not yet linked to a group
  const freeIplans = artifacts.filter(
    a => a.artifact_type === 'iplan' && !usedArtifactIds.has(a.id)
  );

  // Build plan pairs: iplan + linked ADR (same chain_id)
  // If chain_id matches a brief's id, attach the brief too
  const allBriefs = artifacts.filter(a => a.artifact_type === 'brief');
  const briefById = new Map(allBriefs.map(b => [b.id, b]));

  const plans: Plan[] = freeIplans.flatMap(impl => {
    const adr = impl.chain_id
      ? artifacts.find(a => a.artifact_type === 'adr' && a.chain_id === impl.chain_id)
      : undefined;
    if (!adr) return [];
    // A brief is linked if its id matches the plan's chain_id
    const brief = impl.chain_id ? briefById.get(impl.chain_id) : undefined;
    return [{ impl, adr, brief }];
  });

  // Briefs that haven't been converted to a plan yet
  // A brief is "converted" if its id appears as the chain_id of any iplan
  const convertedBriefIds = new Set(
    artifacts.filter(a => a.artifact_type === 'iplan' && a.chain_id).map(a => a.chain_id!)
  );
  const briefs = allBriefs.filter(b => !convertedBriefIds.has(b.id));

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !projectId) return;
    setUploading(true);
    try {
      const content = await file.text();
      const title = file.name.replace(/\.(md|txt)$/i, '');
      await contextArtifactsApi.create({
        project_id: projectId,
        artifact_type: 'brief',
        title: `Brief: ${title}`,
        content,
        scope: 'global',
      });
    } catch (err) {
      console.error('Failed to upload brief:', err);
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  return (
    <div className="flex flex-col h-full bg-muted/20 border-r">
      <div className="shrink-0 bg-background border-b px-4 py-3">
        <div className="flex items-center justify-between">
          <h2 className="font-semibold text-sm flex items-center gap-2">
            Plans
            <Badge variant="secondary" className="text-xs">
              {plans.length}
            </Badge>
            {briefs.length > 0 && (
              <Badge className="text-xs bg-orange-100 text-orange-700 border border-orange-300 gap-1">
                <AlertTriangle className="h-3 w-3" />
                {briefs.length} brief{briefs.length !== 1 ? 's' : ''}
              </Badge>
            )}
          </h2>
          <button
            className="text-muted-foreground hover:text-foreground transition-colors"
            title="Upload brief (.md or .txt)"
            onClick={() => fileInputRef.current?.click()}
            disabled={uploading}
          >
            <FilePlus2 className="h-3.5 w-3.5" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".md,.txt,text/plain,text/markdown"
            className="hidden"
            onChange={handleFileUpload}
          />
        </div>
        <p className="text-xs text-muted-foreground mt-1">
          Design docs awaiting implementation
        </p>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-2 min-h-0">
        {briefs.length > 0 && (
          <div className="space-y-2">
            <p className="text-[10px] font-medium text-orange-600 uppercase tracking-wide px-1">
              Needs Planning
            </p>
            {briefs.map(brief => (
              <BriefCard
                key={brief.id}
                brief={brief}
                onViewArtifact={onViewArtifact}
                onApprove={onApproveBrief}
              />
            ))}
          </div>
        )}

        {plans.length === 0 && briefs.length === 0 ? (
          <div className="text-center py-6 text-xs text-muted-foreground">
            No plans yet — ask the Project Agent to create one
          </div>
        ) : plans.length === 0 ? null : (
          <>
            {briefs.length > 0 && (
              <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide px-1 pt-1">
                Ready to Execute
              </p>
            )}
            {plans.map(plan => (
              <PlanCard
                key={plan.impl.id}
                plan={plan}
                onCreateGroup={onCreateGroupFromPlan}
                onViewArtifact={onViewArtifact}
              />
            ))}
          </>
        )}
      </div>
    </div>
  );
}

export const PlansSidebar = memo(PlansSidebarComponent);
