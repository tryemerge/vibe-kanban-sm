import { useQuery } from '@tanstack/react-query';
import { contextArtifactsApi } from '@/lib/api';

export const contextArtifactsKeys = {
  all: ['contextArtifacts'] as const,
  list: (projectId: string, artifactType?: string) =>
    [...contextArtifactsKeys.all, 'list', projectId, artifactType] as const,
  preview: (projectId: string, taskId?: string) =>
    [...contextArtifactsKeys.all, 'preview', projectId, taskId] as const,
};

export function useContextArtifacts(
  projectId: string | undefined,
  artifactType?: string
) {
  return useQuery({
    queryKey: contextArtifactsKeys.list(projectId!, artifactType),
    queryFn: () => contextArtifactsApi.list(projectId!, artifactType),
    enabled: !!projectId,
    staleTime: 30000,
  });
}

export function useContextPreview(
  projectId: string | undefined,
  taskId?: string
) {
  return useQuery({
    queryKey: contextArtifactsKeys.preview(projectId!, taskId),
    queryFn: () => contextArtifactsApi.previewContext(projectId!, taskId),
    enabled: !!projectId,
    staleTime: 30000,
  });
}
