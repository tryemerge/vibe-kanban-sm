import { useState, useCallback, useEffect } from 'react';

/**
 * Swim lane grouping options
 */
export type SwimLaneGroupBy =
  | { type: 'label'; labelId?: string }  // Group by label, optionally filter to one
  | { type: 'task_group' }               // Group by task group (ADR-012)
  | { type: 'assignee' }                  // Future: group by assignee
  | { type: 'priority' }                  // Future: group by priority
  | { type: 'none' };                     // No grouping (current behavior)

/**
 * Configuration for swim lane display
 */
export interface SwimLaneConfig {
  groupBy: SwimLaneGroupBy;
  collapsedLanes: string[];  // IDs of collapsed lanes
  showUnlabeled: boolean;    // Show tasks without the grouping attribute
}

const DEFAULT_CONFIG: SwimLaneConfig = {
  groupBy: { type: 'none' },
  collapsedLanes: [],
  showUnlabeled: true,
};

function getStorageKey(projectId: string): string {
  return `swimLaneConfig:${projectId}`;
}

function loadConfig(projectId: string): SwimLaneConfig {
  try {
    const stored = localStorage.getItem(getStorageKey(projectId));
    if (stored) {
      const parsed = JSON.parse(stored) as Partial<SwimLaneConfig>;
      return {
        ...DEFAULT_CONFIG,
        ...parsed,
      };
    }
  } catch (e) {
    console.error('Failed to load swim lane config:', e);
  }
  return DEFAULT_CONFIG;
}

function saveConfig(projectId: string, config: SwimLaneConfig): void {
  try {
    localStorage.setItem(getStorageKey(projectId), JSON.stringify(config));
  } catch (e) {
    console.error('Failed to save swim lane config:', e);
  }
}

/**
 * Hook for managing swim lane configuration per project
 * Persists to localStorage
 */
export function useSwimLaneConfig(projectId: string | undefined) {
  const [config, setConfigState] = useState<SwimLaneConfig>(() =>
    projectId ? loadConfig(projectId) : DEFAULT_CONFIG
  );

  // Reload config when projectId changes
  useEffect(() => {
    if (projectId) {
      setConfigState(loadConfig(projectId));
    } else {
      setConfigState(DEFAULT_CONFIG);
    }
  }, [projectId]);

  const setConfig = useCallback(
    (newConfig: SwimLaneConfig | ((prev: SwimLaneConfig) => SwimLaneConfig)) => {
      setConfigState((prev) => {
        const next =
          typeof newConfig === 'function' ? newConfig(prev) : newConfig;
        if (projectId) {
          saveConfig(projectId, next);
        }
        return next;
      });
    },
    [projectId]
  );

  const setGroupBy = useCallback(
    (groupBy: SwimLaneGroupBy) => {
      setConfig((prev) => ({ ...prev, groupBy }));
    },
    [setConfig]
  );

  const toggleLaneCollapse = useCallback(
    (laneId: string) => {
      setConfig((prev) => {
        const isCollapsed = prev.collapsedLanes.includes(laneId);
        return {
          ...prev,
          collapsedLanes: isCollapsed
            ? prev.collapsedLanes.filter((id) => id !== laneId)
            : [...prev.collapsedLanes, laneId],
        };
      });
    },
    [setConfig]
  );

  const setShowUnlabeled = useCallback(
    (show: boolean) => {
      setConfig((prev) => ({ ...prev, showUnlabeled: show }));
    },
    [setConfig]
  );

  const isLaneCollapsed = useCallback(
    (laneId: string) => config.collapsedLanes.includes(laneId),
    [config.collapsedLanes]
  );

  const isEnabled = config.groupBy.type !== 'none';

  return {
    config,
    setConfig,
    setGroupBy,
    toggleLaneCollapse,
    setShowUnlabeled,
    isLaneCollapsed,
    isEnabled,
  };
}
