import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Alert } from '@/components/ui/alert';
import { attemptsApi } from '@/lib/api';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';
import { taskKeys } from '@/hooks/useTask';

export interface CancelAttemptDialogProps {
  attemptId: string;
  projectId: string;
  onSuccess?: () => void;
}

const CancelAttemptDialogImpl =
  NiceModal.create<CancelAttemptDialogProps>(({ attemptId, onSuccess }) => {
    const modal = useModal();
    const queryClient = useQueryClient();
    const [isCancelling, setIsCancelling] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleConfirmCancel = async () => {
      setIsCancelling(true);
      setError(null);

      try {
        await attemptsApi.cancel(attemptId);
        // Invalidate task queries to refresh the UI
        queryClient.invalidateQueries({ queryKey: taskKeys.all });
        queryClient.invalidateQueries({ queryKey: ['attempts'] });
        modal.resolve();
        modal.hide();
        onSuccess?.();
      } catch (err: unknown) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to cancel attempt';
        setError(errorMessage);
      } finally {
        setIsCancelling(false);
      }
    };

    const handleClose = () => {
      modal.reject();
      modal.hide();
    };

    return (
      <Dialog open={modal.visible} onOpenChange={(open) => !open && handleClose()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Cancel Attempt</DialogTitle>
            <DialogDescription>
              Are you sure you want to cancel this attempt?
            </DialogDescription>
          </DialogHeader>

          <Alert variant="destructive" className="mb-4">
            <strong>Warning:</strong> This action will stop any running execution,
            delete the worktree and workspace data, and move the task back to the
            Todo column. This cannot be undone.
          </Alert>

          {error && (
            <Alert variant="destructive" className="mb-4">
              {error}
            </Alert>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleClose}
              disabled={isCancelling}
              autoFocus
            >
              Keep Attempt
            </Button>
            <Button
              variant="destructive"
              onClick={handleConfirmCancel}
              disabled={isCancelling}
            >
              {isCancelling ? 'Cancelling...' : 'Cancel Attempt'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  });

export const CancelAttemptDialog = defineModal<CancelAttemptDialogProps, void>(
  CancelAttemptDialogImpl
);
