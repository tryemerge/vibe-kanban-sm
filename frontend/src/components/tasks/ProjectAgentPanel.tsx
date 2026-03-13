import { useState, useRef, useCallback } from 'react';
import { Send, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import VirtualizedList from '@/components/logs/VirtualizedList';
import { EntriesProvider } from '@/contexts/EntriesContext';
import { RetryUiProvider } from '@/contexts/RetryUiContext';
import { sessionsApi } from '@/lib/api';
import { useAttemptExecution } from '@/hooks';
import type { WorkspaceWithSession } from '@/types/attempt';

interface ProjectAgentPanelProps {
  attempt: WorkspaceWithSession;
}

export function ProjectAgentPanel({ attempt }: ProjectAgentPanelProps) {
  const [input, setInput] = useState('');
  const [isSending, setIsSending] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const sessionId = attempt.session?.id;
  const { isAttemptRunning, stopExecution, isStopping } = useAttemptExecution(
    attempt.id
  );

  const handleSend = useCallback(async () => {
    const prompt = input.trim();
    if (!prompt || !sessionId || isSending || isAttemptRunning) return;

    setIsSending(true);
    try {
      await sessionsApi.followUp(sessionId, {
        prompt,
        variant: null,
        retry_process_id: null,
        force_when_dirty: null,
        perform_git_reset: null,
      });
      setInput('');
    } catch (err) {
      console.error('Failed to send follow-up:', err);
    } finally {
      setIsSending(false);
    }
  }, [input, sessionId, isSending, isAttemptRunning]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        void handleSend();
      }
    },
    [handleSend]
  );

  return (
    <EntriesProvider key={attempt.id}>
      <RetryUiProvider attemptId={attempt.id}>
        <div className="flex flex-col h-full min-h-0">
          {/* Logs */}
          <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
            <VirtualizedList key={attempt.id} attempt={attempt} />
          </div>

          {/* Chat input */}
          <div className="shrink-0 border-t bg-background p-3">
            <div className="flex gap-2 items-end">
              <Textarea
                ref={textareaRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={
                  isAttemptRunning
                    ? 'Agent is working... (you can draft your next message)'
                    : 'Send a message... (⌘Enter to send)'
                }
                disabled={isSending}
                className="min-h-[60px] max-h-[160px] resize-none text-sm"
                rows={2}
              />
              {isAttemptRunning ? (
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => stopExecution()}
                  disabled={isStopping}
                  title="Stop agent"
                >
                  <Square className="h-4 w-4" />
                </Button>
              ) : (
                <Button
                  size="icon"
                  onClick={handleSend}
                  disabled={!input.trim() || isSending || !sessionId}
                  title="Send (⌘Enter)"
                >
                  <Send className="h-4 w-4" />
                </Button>
              )}
            </div>
          </div>
        </div>
      </RetryUiProvider>
    </EntriesProvider>
  );
}
