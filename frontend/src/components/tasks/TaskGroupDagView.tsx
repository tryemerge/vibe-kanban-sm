import { useMemo, useRef, useLayoutEffect, useState, useCallback } from 'react';
import type { TaskGroup, TaskGroupDependency } from 'shared/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface TaskGroupDagViewProps {
  groups: TaskGroup[];
  dependencies: TaskGroupDependency[];
}

// ─── Categorization ──────────────────────────────────────────────────────────

type Category = 'unevaluated' | 'pipeline' | 'completed';

function categorize(g: TaskGroup): Category {
  if (g.is_backlog) return 'unevaluated';
  switch (g.state) {
    case 'draft':
    case 'pending':
      return 'unevaluated';
    case 'analyzing':
    case 'ready':
    case 'prereq_eval':
    case 'executing':
      return 'pipeline';
    case 'done':
    case 'failed':
      return 'completed';
    default:
      return 'unevaluated';
  }
}

// ─── Topological level computation ───────────────────────────────────────────

function computeLevels(
  groups: TaskGroup[],
  deps: TaskGroupDependency[]
): Map<string, number> {
  const levels = new Map<string, number>();
  const groupIds = new Set(groups.map((g) => g.id));
  const prereqs = new Map<string, string[]>();

  for (const g of groups) prereqs.set(g.id, []);

  for (const dep of deps) {
    if (groupIds.has(dep.task_group_id) && groupIds.has(dep.depends_on_group_id)) {
      prereqs.get(dep.task_group_id)?.push(dep.depends_on_group_id);
    }
  }

  const computing = new Set<string>();

  function getLevel(id: string): number {
    if (levels.has(id)) return levels.get(id)!;
    if (computing.has(id)) return 0; // cycle guard
    computing.add(id);
    const prs = prereqs.get(id) ?? [];
    const level = prs.length === 0 ? 0 : Math.max(...prs.map(getLevel)) + 1;
    computing.delete(id);
    levels.set(id, level);
    return level;
  }

  for (const g of groups) getLevel(g.id);
  return levels;
}

// ─── State labels / colors ────────────────────────────────────────────────────

const STATE_BADGE: Record<string, { label: string; cls: string }> = {
  draft: { label: 'Draft', cls: 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400' },
  analyzing: { label: 'Analyzing', cls: 'bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200' },
  ready: { label: 'Ready', cls: 'bg-sky-100 text-sky-800 dark:bg-sky-900 dark:text-sky-200' },
  prereq_eval: { label: 'PreReq', cls: 'bg-violet-100 text-violet-800 dark:bg-violet-900 dark:text-violet-200' },
  executing: { label: 'Executing', cls: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200' },
  done: { label: 'Done', cls: 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400' },
  failed: { label: 'Failed', cls: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300' },
};

// ─── GroupCard ────────────────────────────────────────────────────────────────

interface GroupCardProps {
  group: TaskGroup;
  onRef?: (el: HTMLDivElement | null) => void;
  compact?: boolean;
}

function GroupCard({ group, onRef, compact }: GroupCardProps) {
  const badge = STATE_BADGE[group.state] ?? { label: group.state, cls: '' };
  const color = group.color;

  return (
    <div
      ref={onRef}
      className={cn(
        'border rounded-md bg-card hover:bg-accent/20 transition-colors',
        compact ? 'p-2 min-w-[120px] max-w-[160px]' : 'p-2.5 min-w-[140px] max-w-[200px]'
      )}
      style={color ? { borderLeftColor: color, borderLeftWidth: 3 } : undefined}
    >
      <p className={cn('font-medium leading-tight line-clamp-2 mb-1.5', compact ? 'text-[11px]' : 'text-xs')}>
        {group.name}
      </p>
      <Badge variant="outline" className={cn('text-[10px] px-1 py-0 h-[18px]', badge.cls)}>
        {badge.label}
      </Badge>
    </div>
  );
}

// ─── Arrow SVG layer ──────────────────────────────────────────────────────────

interface Arrow {
  key: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  satisfied: boolean;
}

// ─── DAG canvas (pipeline groups) ────────────────────────────────────────────

interface DagCanvasProps {
  pipeline: TaskGroup[];
  dependencies: TaskGroupDependency[];
}

function DagCanvas({ pipeline, dependencies }: DagCanvasProps) {
  const levels = useMemo(() => computeLevels(pipeline, dependencies), [pipeline, dependencies]);
  const maxLevel = useMemo(() => Math.max(0, ...Array.from(levels.values())), [levels]);

  const byLevel = useMemo(() => {
    const cols: TaskGroup[][] = Array.from({ length: maxLevel + 1 }, () => []);
    for (const g of pipeline) {
      const lv = levels.get(g.id) ?? 0;
      cols[lv].push(g);
    }
    return cols;
  }, [pipeline, levels, maxLevel]);

  const cardRefs = useRef(new Map<string, HTMLDivElement>());
  const containerRef = useRef<HTMLDivElement>(null);
  const [arrows, setArrows] = useState<Arrow[]>([]);

  const computeArrows = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    const cr = container.getBoundingClientRect();
    const next: Arrow[] = [];

    for (const dep of dependencies) {
      const fromEl = cardRefs.current.get(dep.depends_on_group_id);
      const toEl = cardRefs.current.get(dep.task_group_id);
      if (!fromEl || !toEl) continue;
      const fr = fromEl.getBoundingClientRect();
      const tr = toEl.getBoundingClientRect();
      next.push({
        key: `${dep.depends_on_group_id}→${dep.task_group_id}`,
        x1: fr.right - cr.left,
        y1: fr.top + fr.height / 2 - cr.top,
        x2: tr.left - cr.left,
        y2: tr.top + tr.height / 2 - cr.top,
        satisfied: dep.satisfied_at !== null,
      });
    }
    setArrows(next);
  }, [dependencies]);

  useLayoutEffect(() => {
    computeArrows();
  }, [computeArrows, pipeline, byLevel]);

  if (pipeline.length === 0) {
    return (
      <div className="flex items-center justify-center h-24 text-xs text-muted-foreground">
        No groups in pipeline
      </div>
    );
  }

  return (
    <div ref={containerRef} className="relative inline-flex items-start gap-12 p-4 min-w-full">
      {/* SVG arrows */}
      <svg
        className="absolute inset-0 pointer-events-none overflow-visible"
        style={{ width: '100%', height: '100%' }}
      >
        <defs>
          <marker id="dag-arrow" markerWidth="7" markerHeight="5" refX="7" refY="2.5" orient="auto">
            <polygon points="0 0, 7 2.5, 0 5" fill="currentColor" className="text-muted-foreground/50" />
          </marker>
          <marker id="dag-arrow-done" markerWidth="7" markerHeight="5" refX="7" refY="2.5" orient="auto">
            <polygon points="0 0, 7 2.5, 0 5" fill="currentColor" className="text-emerald-400" />
          </marker>
        </defs>
        {arrows.map((a) => {
          const cx = (a.x1 + a.x2) / 2;
          return (
            <path
              key={a.key}
              d={`M ${a.x1} ${a.y1} C ${cx} ${a.y1}, ${cx} ${a.y2}, ${a.x2} ${a.y2}`}
              fill="none"
              strokeWidth="1.5"
              strokeDasharray={a.satisfied ? undefined : '5 3'}
              markerEnd={a.satisfied ? 'url(#dag-arrow-done)' : 'url(#dag-arrow)'}
              className={a.satisfied ? 'stroke-emerald-400' : 'stroke-muted-foreground/40'}
            />
          );
        })}
      </svg>

      {/* Topological columns — left to right = dependency order */}
      {byLevel.map((col, level) => (
        <div key={level} className="flex flex-col gap-4 z-10 relative">
          {level === 0 && (
            <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1 text-center">
              Ready to start
            </p>
          )}
          {level > 0 && (
            <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1 text-center">
              Depends on ←
            </p>
          )}
          {col.map((g) => (
            <GroupCard
              key={g.id}
              group={g}
              onRef={(el) => {
                if (el) cardRefs.current.set(g.id, el);
                else cardRefs.current.delete(g.id);
              }}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

// ─── Section header ───────────────────────────────────────────────────────────

function SectionHeader({ label, count }: { label: string; count: number }) {
  return (
    <div className="px-3 py-2 border-b bg-muted/30 shrink-0">
      <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      <span className="ml-1.5 text-[11px] text-muted-foreground/60">({count})</span>
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

export function TaskGroupDagView({ groups, dependencies }: TaskGroupDagViewProps) {
  const filtered = useMemo(() => groups.filter((g) => !g.is_backlog), [groups]);

  const unevaluated = useMemo(
    () => filtered.filter((g) => categorize(g) === 'unevaluated'),
    [filtered]
  );
  const pipeline = useMemo(
    () => filtered.filter((g) => categorize(g) === 'pipeline'),
    [filtered]
  );
  const completed = useMemo(
    () => filtered.filter((g) => categorize(g) === 'completed'),
    [filtered]
  );

  return (
    <div className="flex flex-col h-full min-h-0 text-sm">
      {/* Top row: Unevaluated (left) | Completed (right) */}
      <div className="flex shrink-0 border-b" style={{ height: '40%', minHeight: 120, maxHeight: 260 }}>
        {/* Left: Unevaluated */}
        <div className="flex-1 flex flex-col min-w-0 border-r overflow-hidden">
          <SectionHeader label="Unevaluated" count={unevaluated.length} />
          <div className="flex-1 overflow-y-auto p-2 flex flex-col gap-1.5">
            {unevaluated.length === 0 ? (
              <p className="text-xs text-muted-foreground text-center pt-4">—</p>
            ) : (
              unevaluated.map((g) => <GroupCard key={g.id} group={g} compact />)
            )}
          </div>
        </div>

        {/* Right: Completed */}
        <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
          <SectionHeader label="Completed" count={completed.length} />
          <div className="flex-1 overflow-y-auto p-2 flex flex-col gap-1.5">
            {completed.length === 0 ? (
              <p className="text-xs text-muted-foreground text-center pt-4">—</p>
            ) : (
              completed.map((g) => <GroupCard key={g.id} group={g} compact />)
            )}
          </div>
        </div>
      </div>

      {/* Bottom: Plans / Pipeline DAG */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <SectionHeader label="Plans" count={pipeline.length} />
        <div className="flex-1 overflow-auto">
          <DagCanvas pipeline={pipeline} dependencies={dependencies} />
        </div>
      </div>
    </div>
  );
}
