import { useEffect, useState, useCallback } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Loader2, Plus, Pencil, Trash2 } from 'lucide-react';
import { agentsApi } from '@/lib/api';
import type { Agent, CreateAgent, UpdateAgent } from 'shared/types';

export function AgentDefinitionsSettings() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [agentsLoading, setAgentsLoading] = useState(true);
  const [agentsError, setAgentsError] = useState<string | null>(null);
  const [agentDialogOpen, setAgentDialogOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<Agent | null>(null);
  const [agentSaving, setAgentSaving] = useState(false);
  const [agentForm, setAgentForm] = useState<CreateAgent>({
    name: '',
    role: '',
    system_prompt: '',
    capabilities: null,
    tools: null,
    description: null,
    context_files: null,
    executor: 'CLAUDE_CODE',
    color: null,
    start_command: null,
  });
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [agentToDelete, setAgentToDelete] = useState<Agent | null>(null);

  const fetchAgents = useCallback(async () => {
    setAgentsLoading(true);
    setAgentsError(null);
    try {
      const result = await agentsApi.list();
      setAgents(result);
    } catch (err) {
      console.error('Failed to fetch agents:', err);
      setAgentsError('Failed to load agents');
    } finally {
      setAgentsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAgents();
  }, [fetchAgents]);

  const openCreateAgentDialog = () => {
    setEditingAgent(null);
    setAgentForm({
      name: '',
      role: '',
      system_prompt: '',
      capabilities: null,
      tools: null,
      description: null,
      context_files: null,
      executor: 'CLAUDE_CODE',
      color: null,
      start_command: null,
    });
    setAgentDialogOpen(true);
  };

  const openEditAgentDialog = (agent: Agent) => {
    setEditingAgent(agent);
    setAgentForm({
      name: agent.name,
      role: agent.role,
      system_prompt: agent.system_prompt,
      capabilities: agent.capabilities ? JSON.parse(agent.capabilities) : null,
      tools: agent.tools ? JSON.parse(agent.tools) : null,
      description: agent.description,
      context_files: agent.context_files ? JSON.parse(agent.context_files) : null,
      executor: agent.executor,
      color: agent.color,
      start_command: agent.start_command,
    });
    setAgentDialogOpen(true);
  };

  const handleAgentSave = async () => {
    setAgentSaving(true);
    try {
      if (editingAgent) {
        const updateData: UpdateAgent = {
          name: agentForm.name || null,
          role: agentForm.role || null,
          system_prompt: agentForm.system_prompt || null,
          capabilities: agentForm.capabilities,
          tools: agentForm.tools,
          description: agentForm.description,
          context_files: agentForm.context_files,
          executor: agentForm.executor,
          color: agentForm.color,
          start_command: agentForm.start_command,
        };
        await agentsApi.update(editingAgent.id, updateData);
      } else {
        await agentsApi.create(agentForm);
      }
      setAgentDialogOpen(false);
      await fetchAgents();
    } catch (err) {
      console.error('Failed to save agent:', err);
      setAgentsError('Failed to save agent');
    } finally {
      setAgentSaving(false);
    }
  };

  const handleAgentDelete = async () => {
    if (!agentToDelete) return;
    setAgentSaving(true);
    try {
      await agentsApi.delete(agentToDelete.id);
      setDeleteConfirmOpen(false);
      setAgentToDelete(null);
      await fetchAgents();
    } catch (err) {
      console.error('Failed to delete agent:', err);
      setAgentsError('Failed to delete agent');
    } finally {
      setAgentSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
          <div>
            <CardTitle>Agents</CardTitle>
            <CardDescription>
              Define agents for workflow automation with specific roles, prompts, and capabilities.
            </CardDescription>
          </div>
          <Button onClick={openCreateAgentDialog} size="sm">
            <Plus className="h-4 w-4 mr-2" />
            New Agent
          </Button>
        </CardHeader>
        <CardContent>
          {agentsError && (
            <Alert variant="destructive" className="mb-4">
              <AlertDescription>{agentsError}</AlertDescription>
            </Alert>
          )}

          {agentsLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
              <span className="ml-2">Loading agents...</span>
            </div>
          ) : agents.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No agents defined yet.</p>
              <p className="text-sm">Create an agent to use in workflow automation.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {agents.map((agent) => (
                <div
                  key={agent.id}
                  className="flex items-center justify-between p-4 border rounded-lg hover:bg-accent/50 transition-colors"
                >
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    <div
                      className="w-4 h-4 rounded-full flex-shrink-0"
                      style={{ backgroundColor: agent.color || '#6b7280' }}
                    />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <h4 className="font-medium truncate">{agent.name}</h4>
                        <span className="text-xs px-2 py-0.5 bg-secondary rounded-full">
                          {agent.executor}
                        </span>
                      </div>
                      <p className="text-sm text-muted-foreground truncate">
                        {agent.role}
                      </p>
                      {agent.description && (
                        <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                          {agent.description}
                        </p>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => openEditAgentDialog(agent)}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setAgentToDelete(agent);
                        setDeleteConfirmOpen(true);
                      }}
                    >
                      <Trash2 className="h-4 w-4 text-destructive" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Agent Create/Edit Dialog */}
      <Dialog open={agentDialogOpen} onOpenChange={setAgentDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {editingAgent ? 'Edit Agent' : 'Create Agent'}
            </DialogTitle>
            <DialogDescription>
              {editingAgent
                ? 'Update the agent configuration.'
                : 'Define a new agent for workflow automation.'}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="agent-name">Name *</Label>
              <Input
                id="agent-name"
                placeholder="e.g., Code Reviewer"
                value={agentForm.name}
                onChange={(e) =>
                  setAgentForm({ ...agentForm, name: e.target.value })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-role">Role *</Label>
              <Input
                id="agent-role"
                placeholder="e.g., Reviews code for quality and best practices"
                value={agentForm.role}
                onChange={(e) =>
                  setAgentForm({ ...agentForm, role: e.target.value })
                }
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="agent-executor">Executor</Label>
                <Select
                  value={agentForm.executor || 'CLAUDE_CODE'}
                  onValueChange={(value) =>
                    setAgentForm({ ...agentForm, executor: value })
                  }
                >
                  <SelectTrigger id="agent-executor">
                    <SelectValue placeholder="Select executor" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="CLAUDE_CODE">Claude Code</SelectItem>
                    <SelectItem value="GEMINI">Gemini</SelectItem>
                    <SelectItem value="CODEX">Codex</SelectItem>
                    <SelectItem value="AMP">Amp</SelectItem>
                    <SelectItem value="CURSOR_AGENT">Cursor Agent</SelectItem>
                    <SelectItem value="OPENCODE">Opencode</SelectItem>
                    <SelectItem value="QWEN_CODE">Qwen Code</SelectItem>
                    <SelectItem value="COPILOT">Copilot</SelectItem>
                    <SelectItem value="DROID">Droid</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="agent-color">Color</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="agent-color"
                    type="color"
                    className="w-12 h-10 p-1 cursor-pointer"
                    value={agentForm.color || '#6b7280'}
                    onChange={(e) =>
                      setAgentForm({ ...agentForm, color: e.target.value })
                    }
                  />
                  <Input
                    placeholder="#6b7280"
                    value={agentForm.color || ''}
                    onChange={(e) =>
                      setAgentForm({
                        ...agentForm,
                        color: e.target.value || null,
                      })
                    }
                    className="flex-1"
                  />
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-description">Description</Label>
              <Input
                id="agent-description"
                placeholder="Brief description of what this agent does"
                value={agentForm.description || ''}
                onChange={(e) =>
                  setAgentForm({
                    ...agentForm,
                    description: e.target.value || null,
                  })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-prompt">System Prompt *</Label>
              <Textarea
                id="agent-prompt"
                placeholder="Instructions for the agent..."
                className="min-h-[200px] font-mono text-sm"
                value={agentForm.system_prompt}
                onChange={(e) =>
                  setAgentForm({
                    ...agentForm,
                    system_prompt: e.target.value,
                  })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-start-command">Execution Instructions</Label>
              <Textarea
                id="agent-start-command"
                placeholder="Provide a detailed list of exactly what the agent should do:

1. First, analyze the task requirements
2. Review relevant code files
3. Make necessary changes
4. Write tests if applicable
5. Commit with a clear message"
                className="min-h-[120px] font-mono text-sm"
                value={agentForm.start_command || ''}
                onChange={(e) =>
                  setAgentForm({
                    ...agentForm,
                    start_command: e.target.value || null,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Best provided as a detailed list of exactly what the agent should do when starting work on a task.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-capabilities">
                Capabilities (comma-separated)
              </Label>
              <Input
                id="agent-capabilities"
                placeholder="e.g., code_review, testing, documentation"
                value={agentForm.capabilities?.join(', ') || ''}
                onChange={(e) => {
                  const caps = e.target.value
                    .split(',')
                    .map((s) => s.trim())
                    .filter(Boolean);
                  setAgentForm({
                    ...agentForm,
                    capabilities: caps.length > 0 ? caps : null,
                  });
                }}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-tools">Tools (comma-separated)</Label>
              <Input
                id="agent-tools"
                placeholder="e.g., read, write, bash"
                value={agentForm.tools?.join(', ') || ''}
                onChange={(e) => {
                  const tools = e.target.value
                    .split(',')
                    .map((s) => s.trim())
                    .filter(Boolean);
                  setAgentForm({
                    ...agentForm,
                    tools: tools.length > 0 ? tools : null,
                  });
                }}
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setAgentDialogOpen(false)}
              disabled={agentSaving}
            >
              Cancel
            </Button>
            <Button
              onClick={handleAgentSave}
              disabled={
                agentSaving ||
                !agentForm.name ||
                !agentForm.role ||
                !agentForm.system_prompt
              }
            >
              {agentSaving && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {editingAgent ? 'Update' : 'Create'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Agent</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{agentToDelete?.name}"? This
              action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteConfirmOpen(false);
                setAgentToDelete(null);
              }}
              disabled={agentSaving}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleAgentDelete}
              disabled={agentSaving}
            >
              {agentSaving && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
