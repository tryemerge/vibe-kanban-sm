import { useQuery } from '@tanstack/react-query';
import { taskEventsApi } from '@/lib/api';

export function useTaskEvents(taskId: string | undefined) {
  return useQuery({
    queryKey: ['taskEvents', taskId],
    queryFn: () => taskEventsApi.getByTaskId(taskId!),
    enabled: !!taskId,
    staleTime: 10000, // 10 seconds - events may update more frequently
  });
}
