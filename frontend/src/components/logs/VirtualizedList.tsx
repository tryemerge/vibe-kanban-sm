import { useEffect, useRef, useState } from 'react';

import DisplayConversationEntry from '../NormalizedConversation/DisplayConversationEntry';
import { useEntries } from '@/contexts/EntriesContext';
import {
  AddEntryType,
  PatchTypeWithKey,
  useConversationHistory,
} from '@/hooks/useConversationHistory';
import { Loader2 } from 'lucide-react';
import { TaskWithAttemptStatus } from 'shared/types';
import type { WorkspaceWithSession } from '@/types/attempt';
import { ApprovalFormProvider } from '@/contexts/ApprovalFormContext';

interface VirtualizedListProps {
  attempt: WorkspaceWithSession;
  task?: TaskWithAttemptStatus;
}

const VirtualizedList = ({ attempt, task }: VirtualizedListProps) => {
  const [entries, setLocalEntries] = useState<PatchTypeWithKey[]>([]);
  const [loading, setLoading] = useState(true);
  const { setEntries, reset } = useEntries();

  const scrollRef = useRef<HTMLDivElement>(null);
  // Track whether the user is at the bottom so we don't hijack manual scrolls
  const stickyToBottom = useRef(true);

  useEffect(() => {
    setLoading(true);
    setLocalEntries([]);
    reset();
    stickyToBottom.current = true;
  }, [attempt.id, reset]);

  const onEntriesUpdated = (
    newEntries: PatchTypeWithKey[],
    addType: AddEntryType,
    newLoading: boolean
  ) => {
    setLocalEntries(newEntries);
    setEntries(newEntries);

    if (loading) {
      setLoading(newLoading);
    }

    // On initial/historic load, always scroll to bottom to show latest
    if (addType === 'initial' || addType === 'historic') {
      stickyToBottom.current = true;
    }
  };

  useConversationHistory({ attempt, onEntriesUpdated });

  // Scroll to bottom whenever entries change, if sticky
  useEffect(() => {
    if (!stickyToBottom.current) return;
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  });

  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const distanceFromBottom =
      el.scrollHeight - el.scrollTop - el.clientHeight;
    stickyToBottom.current = distanceFromBottom < 80;
  };

  return (
    <ApprovalFormProvider>
      <div className="relative flex-1 min-h-0 flex flex-col">
        <div
          ref={scrollRef}
          className="flex-1 overflow-y-auto"
          onScroll={handleScroll}
        >
          <div className="h-2" />
          {entries.map((data) => {
            const key = `l-${data.patchKey}`;
            if (data.type === 'STDOUT') {
              return <p key={key}>{data.content}</p>;
            }
            if (data.type === 'STDERR') {
              return <p key={key}>{data.content}</p>;
            }
            if (data.type === 'NORMALIZED_ENTRY') {
              return (
                <DisplayConversationEntry
                  key={key}
                  expansionKey={data.patchKey}
                  entry={data.content}
                  executionProcessId={data.executionProcessId}
                  taskAttempt={attempt}
                  task={task}
                />
              );
            }
            return null;
          })}
          <div className="h-2" />
        </div>

        {loading && (
          <div className="absolute inset-0 bg-background flex flex-col gap-2 justify-center items-center z-10">
            <Loader2 className="h-8 w-8 animate-spin" />
            <p>Loading History</p>
          </div>
        )}
      </div>
    </ApprovalFormProvider>
  );
};

export default VirtualizedList;
