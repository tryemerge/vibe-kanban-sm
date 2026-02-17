import { useQuery } from '@tanstack/react-query';
import { taskEventsApi } from '@/lib/api';

export function useTaskEvents(taskId: string | undefined, workspaceId?: string) {
  return useQuery({
    queryKey: ['taskEvents', taskId, workspaceId ?? null],
    queryFn: () => taskEventsApi.getByTaskId(taskId!, workspaceId),
    enabled: !!taskId,
    staleTime: 10000, // 10 seconds - events may update more frequently
  });
}
