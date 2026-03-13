import { useEffect, useRef } from 'react';
import { toast } from 'sonner';
import type { TaskWithAttemptStatus } from 'shared/types';

/**
 * Watches tasks for completion/failure transitions and shows in-app toasts.
 * Fires when a task transitions from has_in_progress_attempt=true to false.
 */
export function useTaskCompletionToasts(tasks: TaskWithAttemptStatus[]) {
  const prevStateRef = useRef<
    Record<string, { inProgress: boolean; failed: boolean; title: string }>
  >({});

  useEffect(() => {
    const prev = prevStateRef.current;
    const next: typeof prev = {};

    for (const task of tasks) {
      const inProgress = task.has_in_progress_attempt;
      const failed = task.last_attempt_failed;

      next[task.id] = { inProgress, failed, title: task.title };

      const prevTask = prev[task.id];
      if (prevTask && prevTask.inProgress && !inProgress) {
        if (failed) {
          toast.error(`"${task.title}" failed`, {
            description: 'The agent encountered an error.',
          });
        } else {
          toast.success(`"${task.title}" completed`, {
            description: 'The agent finished successfully.',
          });
        }
      }
    }

    prevStateRef.current = next;
  }, [tasks]);
}
