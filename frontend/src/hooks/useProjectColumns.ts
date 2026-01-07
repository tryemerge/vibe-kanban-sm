import { useQuery } from '@tanstack/react-query';
import { columnsApi } from '@/lib/api';
import type { KanbanColumn } from 'shared/types';

export function useProjectColumns(projectId: string | undefined) {
  return useQuery({
    queryKey: ['projectColumns', projectId],
    queryFn: () => columnsApi.listByProject(projectId!),
    enabled: !!projectId,
    staleTime: 30000, // 30 seconds
  });
}

/**
 * Find a column by its slug (which maps to task status)
 */
export function findColumnBySlug(
  columns: KanbanColumn[] | undefined,
  slug: string
): KanbanColumn | undefined {
  return columns?.find((col) => col.slug === slug);
}

/**
 * Build a map from slug to column for quick lookups
 */
export function buildColumnBySlugMap(
  columns: KanbanColumn[] | undefined
): Map<string, KanbanColumn> {
  const map = new Map<string, KanbanColumn>();
  if (columns) {
    for (const col of columns) {
      map.set(col.slug, col);
    }
  }
  return map;
}
