import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { cloneDeep, isEqual } from 'lodash';
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Checkbox } from '@/components/ui/checkbox';
import { JSONEditor } from '@/components/ui/json-editor';
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
import { ChevronDown, Loader2, Plus, Pencil, Trash2 } from 'lucide-react';
import { agentsApi } from '@/lib/api';

import { ExecutorConfigForm } from '@/components/ExecutorConfigForm';
import { useProfiles } from '@/hooks/useProfiles';
import { useUserSystem } from '@/components/ConfigProvider';
import { CreateConfigurationDialog } from '@/components/dialogs/settings/CreateConfigurationDialog';
import { DeleteConfigurationDialog } from '@/components/dialogs/settings/DeleteConfigurationDialog';
import { useAgentAvailability } from '@/hooks/useAgentAvailability';
import { AgentAvailabilityIndicator } from '@/components/AgentAvailabilityIndicator';
import type {
  BaseCodingAgent,
  ExecutorConfigs,
  ExecutorProfileId,
  Agent,
  CreateAgent,
  UpdateAgent,
} from 'shared/types';

type ExecutorsMap = Record<string, Record<string, Record<string, unknown>>>;

export function AgentSettings() {
  const { t } = useTranslation(['settings', 'common']);
  // Use profiles hook for server state
  const {
    profilesContent: serverProfilesContent,
    profilesPath,
    isLoading: profilesLoading,
    isSaving: profilesSaving,
    error: profilesError,
    save: saveProfiles,
  } = useProfiles();

  const { config, updateAndSaveConfig, profiles, reloadSystem } =
    useUserSystem();

  // Local editor state (draft that may differ from server)
  const [localProfilesContent, setLocalProfilesContent] = useState('');
  const [profilesSuccess, setProfilesSuccess] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Form-based editor state
  const [useFormEditor, setUseFormEditor] = useState(true);
  const [selectedExecutorType, setSelectedExecutorType] =
    useState<BaseCodingAgent>('CLAUDE_CODE' as BaseCodingAgent);
  const [selectedConfiguration, setSelectedConfiguration] =
    useState<string>('DEFAULT');
  const [localParsedProfiles, setLocalParsedProfiles] =
    useState<ExecutorConfigs | null>(null);
  const [isDirty, setIsDirty] = useState(false);

  // Default executor profile state
  const [executorDraft, setExecutorDraft] = useState<ExecutorProfileId | null>(
    () => (config?.executor_profile ? cloneDeep(config.executor_profile) : null)
  );
  const [executorSaving, setExecutorSaving] = useState(false);
  const [executorSuccess, setExecutorSuccess] = useState(false);
  const [executorError, setExecutorError] = useState<string | null>(null);

  // Check agent availability when draft executor changes
  const agentAvailability = useAgentAvailability(executorDraft?.executor);

  // Subagents state
  const [subagents, setSubagents] = useState<Agent[]>([]);
  const [subagentsLoading, setSubagentsLoading] = useState(true);
  const [subagentsError, setSubagentsError] = useState<string | null>(null);
  const [subagentDialogOpen, setSubagentDialogOpen] = useState(false);
  const [editingSubagent, setEditingSubagent] = useState<Agent | null>(null);
  const [subagentSaving, setSubagentSaving] = useState(false);
  const [subagentForm, setSubagentForm] = useState<CreateAgent>({
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
    deliverable: null,
  });
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [subagentToDelete, setSubagentToDelete] = useState<Agent | null>(null);

  // Fetch subagents
  const fetchSubagents = useCallback(async () => {
    setSubagentsLoading(true);
    setSubagentsError(null);
    try {
      const agents = await agentsApi.list();
      setSubagents(agents);
    } catch (err) {
      console.error('Failed to fetch subagents:', err);
      setSubagentsError('Failed to load subagents');
    } finally {
      setSubagentsLoading(false);
    }
  }, []);

  // Load subagents on mount
  useEffect(() => {
    fetchSubagents();
  }, [fetchSubagents]);

  // Open create subagent dialog
  const openCreateSubagentDialog = () => {
    setEditingSubagent(null);
    setSubagentForm({
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
      deliverable: null,
    });
    setSubagentDialogOpen(true);
  };

  // Open edit subagent dialog
  const openEditSubagentDialog = (agent: Agent) => {
    setEditingSubagent(agent);
    setSubagentForm({
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
      deliverable: agent.deliverable,
    });
    setSubagentDialogOpen(true);
  };

  // Handle subagent form save
  const handleSubagentSave = async () => {
    setSubagentSaving(true);
    try {
      if (editingSubagent) {
        // Update existing
        const updateData: UpdateAgent = {
          name: subagentForm.name || null,
          role: subagentForm.role || null,
          system_prompt: subagentForm.system_prompt || null,
          capabilities: subagentForm.capabilities,
          tools: subagentForm.tools,
          description: subagentForm.description,
          context_files: subagentForm.context_files,
          executor: subagentForm.executor,
          color: subagentForm.color,
          start_command: subagentForm.start_command,
          deliverable: subagentForm.deliverable,
        };
        await agentsApi.update(editingSubagent.id, updateData);
      } else {
        // Create new
        await agentsApi.create(subagentForm);
      }
      setSubagentDialogOpen(false);
      await fetchSubagents();
    } catch (err) {
      console.error('Failed to save subagent:', err);
      setSubagentsError('Failed to save subagent');
    } finally {
      setSubagentSaving(false);
    }
  };

  // Handle subagent delete
  const handleSubagentDelete = async () => {
    if (!subagentToDelete) return;
    setSubagentSaving(true);
    try {
      await agentsApi.delete(subagentToDelete.id);
      setDeleteConfirmOpen(false);
      setSubagentToDelete(null);
      await fetchSubagents();
    } catch (err) {
      console.error('Failed to delete subagent:', err);
      setSubagentsError('Failed to delete subagent');
    } finally {
      setSubagentSaving(false);
    }
  };

  // Sync server state to local state when not dirty
  useEffect(() => {
    if (!isDirty && serverProfilesContent) {
      setLocalProfilesContent(serverProfilesContent);
      // Parse JSON inside effect to avoid object dependency
      try {
        const parsed = JSON.parse(serverProfilesContent);
        setLocalParsedProfiles(parsed);
      } catch (err) {
        console.error('Failed to parse profiles JSON:', err);
        setLocalParsedProfiles(null);
      }
    }
  }, [serverProfilesContent, isDirty]);

  // Check if executor draft differs from saved config
  const executorDirty =
    executorDraft && config?.executor_profile
      ? !isEqual(executorDraft, config.executor_profile)
      : false;

  // Sync executor draft when config changes (only if not dirty)
  useEffect(() => {
    if (config?.executor_profile) {
      setExecutorDraft((currentDraft) => {
        // Only update if draft matches the old config (not dirty)
        if (!currentDraft || isEqual(currentDraft, config.executor_profile)) {
          return cloneDeep(config.executor_profile);
        }
        return currentDraft;
      });
    }
  }, [config?.executor_profile]);

  // Update executor draft
  const updateExecutorDraft = (newProfile: ExecutorProfileId) => {
    setExecutorDraft(newProfile);
  };

  // Save executor profile
  const handleSaveExecutorProfile = async () => {
    if (!executorDraft || !config) return;

    setExecutorSaving(true);
    setExecutorError(null);

    try {
      await updateAndSaveConfig({ executor_profile: executorDraft });
      setExecutorSuccess(true);
      setTimeout(() => setExecutorSuccess(false), 3000);
      reloadSystem();
    } catch (err) {
      setExecutorError(t('settings.general.save.error'));
      console.error('Error saving executor profile:', err);
    } finally {
      setExecutorSaving(false);
    }
  };

  // Sync raw profiles with parsed profiles
  const syncRawProfiles = (profiles: unknown) => {
    setLocalProfilesContent(JSON.stringify(profiles, null, 2));
  };

  // Mark profiles as dirty
  const markDirty = (nextProfiles: unknown) => {
    setLocalParsedProfiles(nextProfiles as ExecutorConfigs);
    syncRawProfiles(nextProfiles);
    setIsDirty(true);
  };

  // Open create dialog
  const openCreateDialog = async () => {
    try {
      const result = await CreateConfigurationDialog.show({
        executorType: selectedExecutorType,
        existingConfigs: Object.keys(
          localParsedProfiles?.executors?.[selectedExecutorType] || {}
        ),
      });

      if (result.action === 'created' && result.configName) {
        createConfiguration(
          selectedExecutorType,
          result.configName,
          result.cloneFrom
        );
      }
    } catch (error) {
      // User cancelled - do nothing
    }
  };

  // Create new configuration
  const createConfiguration = (
    executorType: string,
    configName: string,
    baseConfig?: string | null
  ) => {
    if (!localParsedProfiles || !localParsedProfiles.executors) return;

    const executorsMap =
      localParsedProfiles.executors as unknown as ExecutorsMap;
    const base =
      baseConfig && executorsMap[executorType]?.[baseConfig]?.[executorType]
        ? executorsMap[executorType][baseConfig][executorType]
        : {};

    const updatedProfiles = {
      ...localParsedProfiles,
      executors: {
        ...localParsedProfiles.executors,
        [executorType]: {
          ...executorsMap[executorType],
          [configName]: {
            [executorType]: base,
          },
        },
      },
    };

    markDirty(updatedProfiles);
    setSelectedConfiguration(configName);
  };

  // Open delete dialog
  const openDeleteDialog = async (configName: string) => {
    try {
      const result = await DeleteConfigurationDialog.show({
        configName,
        executorType: selectedExecutorType,
      });

      if (result === 'deleted') {
        await handleDeleteConfiguration(configName);
      }
    } catch (error) {
      // User cancelled - do nothing
    }
  };

  // Handle delete configuration
  const handleDeleteConfiguration = async (configToDelete: string) => {
    if (!localParsedProfiles) {
      return;
    }

    // Clear any previous errors
    setSaveError(null);

    try {
      // Validate that the configuration exists
      if (
        !localParsedProfiles.executors[selectedExecutorType]?.[configToDelete]
      ) {
        return;
      }

      // Check if this is the last configuration
      const currentConfigs = Object.keys(
        localParsedProfiles.executors[selectedExecutorType] || {}
      );
      if (currentConfigs.length <= 1) {
        return;
      }

      // Remove the configuration from the executor
      const remainingConfigs = {
        ...localParsedProfiles.executors[selectedExecutorType],
      };
      delete remainingConfigs[configToDelete];

      const updatedProfiles = {
        ...localParsedProfiles,
        executors: {
          ...localParsedProfiles.executors,
          [selectedExecutorType]: remainingConfigs,
        },
      };

      const executorsMap = updatedProfiles.executors as unknown as ExecutorsMap;
      // If no configurations left, create a blank DEFAULT (should not happen due to check above)
      if (Object.keys(remainingConfigs).length === 0) {
        executorsMap[selectedExecutorType] = {
          DEFAULT: { [selectedExecutorType]: {} },
        };
      }

      try {
        // Save using hook
        await saveProfiles(JSON.stringify(updatedProfiles, null, 2));

        // Update local state and reset dirty flag
        setLocalParsedProfiles(updatedProfiles);
        setLocalProfilesContent(JSON.stringify(updatedProfiles, null, 2));
        setIsDirty(false);

        // Select the next available configuration
        const nextConfigs = Object.keys(
          executorsMap[selectedExecutorType] || {}
        );
        const nextSelected = nextConfigs[0] || 'DEFAULT';
        setSelectedConfiguration(nextSelected);

        // Show success
        setProfilesSuccess(true);
        setTimeout(() => setProfilesSuccess(false), 3000);

        // Refresh global system so deleted configs are removed elsewhere
        reloadSystem();
      } catch (saveError: unknown) {
        console.error('Failed to save deletion to backend:', saveError);
        setSaveError(t('settings.agents.errors.deleteFailed'));
      }
    } catch (error) {
      console.error('Error deleting configuration:', error);
    }
  };

  const handleProfilesChange = (value: string) => {
    setLocalProfilesContent(value);
    setIsDirty(true);

    // Validate JSON on change
    if (value.trim()) {
      try {
        const parsed = JSON.parse(value);
        setLocalParsedProfiles(parsed);
      } catch (err) {
        // Invalid JSON, keep local content but clear parsed
        setLocalParsedProfiles(null);
      }
    }
  };

  const handleSaveProfiles = async () => {
    // Clear any previous errors
    setSaveError(null);

    try {
      const contentToSave =
        useFormEditor && localParsedProfiles
          ? JSON.stringify(localParsedProfiles, null, 2)
          : localProfilesContent;

      await saveProfiles(contentToSave);
      setProfilesSuccess(true);
      setIsDirty(false);
      setTimeout(() => setProfilesSuccess(false), 3000);

      // Update the local content if using form editor
      if (useFormEditor && localParsedProfiles) {
        setLocalProfilesContent(contentToSave);
      }

      // Refresh global system so new profiles are available elsewhere
      reloadSystem();
    } catch (err: unknown) {
      console.error('Failed to save profiles:', err);
      setSaveError(t('settings.agents.errors.saveFailed'));
    }
  };

  const handleExecutorConfigChange = (
    executorType: string,
    configuration: string,
    formData: unknown
  ) => {
    if (!localParsedProfiles || !localParsedProfiles.executors) return;

    const executorsMap =
      localParsedProfiles.executors as unknown as ExecutorsMap;
    // Update the parsed profiles with the new config
    const updatedProfiles = {
      ...localParsedProfiles,
      executors: {
        ...localParsedProfiles.executors,
        [executorType]: {
          ...executorsMap[executorType],
          [configuration]: {
            [executorType]: formData,
          },
        },
      },
    };

    markDirty(updatedProfiles);
  };

  const handleExecutorConfigSave = async (formData: unknown) => {
    if (!localParsedProfiles || !localParsedProfiles.executors) return;

    // Clear any previous errors
    setSaveError(null);

    // Update the parsed profiles with the saved config
    const updatedProfiles = {
      ...localParsedProfiles,
      executors: {
        ...localParsedProfiles.executors,
        [selectedExecutorType]: {
          ...localParsedProfiles.executors[selectedExecutorType],
          [selectedConfiguration]: {
            [selectedExecutorType]: formData,
          },
        },
      },
    };

    // Update state
    setLocalParsedProfiles(updatedProfiles);

    // Save the updated profiles directly
    try {
      const contentToSave = JSON.stringify(updatedProfiles, null, 2);

      await saveProfiles(contentToSave);
      setProfilesSuccess(true);
      setIsDirty(false);
      setTimeout(() => setProfilesSuccess(false), 3000);

      // Update the local content as well
      setLocalProfilesContent(contentToSave);

      // Refresh global system so new profiles are available elsewhere
      reloadSystem();
    } catch (err: unknown) {
      console.error('Failed to save profiles:', err);
      setSaveError(t('settings.agents.errors.saveConfigFailed'));
    }
  };

  if (profilesLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin" />
        <span className="ml-2">{t('settings.agents.loading')}</span>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {!!profilesError && (
        <Alert variant="destructive">
          <AlertDescription>
            {profilesError instanceof Error
              ? profilesError.message
              : String(profilesError)}
          </AlertDescription>
        </Alert>
      )}

      {profilesSuccess && (
        <Alert variant="success">
          <AlertDescription className="font-medium">
            {t('settings.agents.save.success')}
          </AlertDescription>
        </Alert>
      )}

      {saveError && (
        <Alert variant="destructive">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}

      {executorError && (
        <Alert variant="destructive">
          <AlertDescription>{executorError}</AlertDescription>
        </Alert>
      )}

      {executorSuccess && (
        <Alert variant="success">
          <AlertDescription className="font-medium">
            {t('settings.general.save.success')}
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.general.taskExecution.title')}</CardTitle>
          <CardDescription>
            {t('settings.general.taskExecution.description')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="executor">
              {t('settings.general.taskExecution.executor.label')}
            </Label>
            <div className="grid grid-cols-2 gap-2">
              <Select
                value={executorDraft?.executor ?? ''}
                onValueChange={(value: string) => {
                  const variants = profiles?.[value];
                  const keepCurrentVariant =
                    variants &&
                    executorDraft?.variant &&
                    variants[executorDraft.variant];

                  const newProfile: ExecutorProfileId = {
                    executor: value as BaseCodingAgent,
                    variant: keepCurrentVariant ? executorDraft!.variant : null,
                  };
                  updateExecutorDraft(newProfile);
                }}
                disabled={!profiles}
              >
                <SelectTrigger id="executor">
                  <SelectValue
                    placeholder={t(
                      'settings.general.taskExecution.executor.placeholder'
                    )}
                  />
                </SelectTrigger>
                <SelectContent>
                  {profiles &&
                    Object.entries(profiles)
                      .sort((a, b) => a[0].localeCompare(b[0]))
                      .map(([profileKey]) => (
                        <SelectItem key={profileKey} value={profileKey}>
                          {profileKey}
                        </SelectItem>
                      ))}
                </SelectContent>
              </Select>

              {/* Show variant selector if selected profile has variants */}
              {(() => {
                const currentProfileVariant = executorDraft;
                const selectedProfile =
                  profiles?.[currentProfileVariant?.executor || ''];
                const hasVariants =
                  selectedProfile && Object.keys(selectedProfile).length > 0;

                if (hasVariants) {
                  return (
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          variant="outline"
                          className="w-full h-10 px-2 flex items-center justify-between"
                        >
                          <span className="text-sm truncate flex-1 text-left">
                            {currentProfileVariant?.variant ||
                              t('settings.general.taskExecution.defaultLabel')}
                          </span>
                          <ChevronDown className="h-4 w-4 ml-1 flex-shrink-0" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent>
                        {Object.entries(selectedProfile).map(
                          ([variantLabel]) => (
                            <DropdownMenuItem
                              key={variantLabel}
                              onClick={() => {
                                const newProfile: ExecutorProfileId = {
                                  executor: currentProfileVariant!.executor,
                                  variant: variantLabel,
                                };
                                updateExecutorDraft(newProfile);
                              }}
                              className={
                                currentProfileVariant?.variant === variantLabel
                                  ? 'bg-accent'
                                  : ''
                              }
                            >
                              {variantLabel}
                            </DropdownMenuItem>
                          )
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  );
                } else if (selectedProfile) {
                  // Show disabled button when profile exists but has no variants
                  return (
                    <Button
                      variant="outline"
                      className="w-full h-10 px-2 flex items-center justify-between"
                      disabled
                    >
                      <span className="text-sm truncate flex-1 text-left">
                        {t('settings.general.taskExecution.defaultLabel')}
                      </span>
                    </Button>
                  );
                }
                return null;
              })()}
            </div>
            <AgentAvailabilityIndicator availability={agentAvailability} />
            <p className="text-sm text-muted-foreground">
              {t('settings.general.taskExecution.executor.helper')}
            </p>
          </div>
          <div className="flex justify-end">
            <Button
              onClick={handleSaveExecutorProfile}
              disabled={!executorDirty || executorSaving}
            >
              {executorSaving && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('common:buttons.save')}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.agents.title')}</CardTitle>
          <CardDescription>{t('settings.agents.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Editor type toggle */}
          <div className="flex items-center space-x-2">
            <Checkbox
              id="use-form-editor"
              checked={!useFormEditor}
              onCheckedChange={(checked) => setUseFormEditor(!checked)}
              disabled={profilesLoading || !localParsedProfiles}
            />
            <Label htmlFor="use-form-editor">
              {t('settings.agents.editor.formLabel')}
            </Label>
          </div>

          {useFormEditor &&
          localParsedProfiles &&
          localParsedProfiles.executors ? (
            // Form-based editor
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="executor-type">
                    {t('settings.agents.editor.agentLabel')}
                  </Label>
                  <Select
                    value={selectedExecutorType}
                    onValueChange={(value) => {
                      setSelectedExecutorType(value as BaseCodingAgent);
                      // Reset configuration selection when executor type changes
                      setSelectedConfiguration('DEFAULT');
                    }}
                  >
                    <SelectTrigger id="executor-type">
                      <SelectValue
                        placeholder={t(
                          'settings.agents.editor.agentPlaceholder'
                        )}
                      />
                    </SelectTrigger>
                    <SelectContent>
                      {Object.keys(localParsedProfiles.executors).map(
                        (type) => (
                          <SelectItem key={type} value={type}>
                            {type}
                          </SelectItem>
                        )
                      )}
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="configuration">
                    {t('settings.agents.editor.configLabel')}
                  </Label>
                  <div className="flex gap-2">
                    <Select
                      value={selectedConfiguration}
                      onValueChange={(value) => {
                        if (value === '__create__') {
                          openCreateDialog();
                        } else {
                          setSelectedConfiguration(value);
                        }
                      }}
                      disabled={
                        !localParsedProfiles.executors[selectedExecutorType]
                      }
                    >
                      <SelectTrigger id="configuration">
                        <SelectValue
                          placeholder={t(
                            'settings.agents.editor.configPlaceholder'
                          )}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.keys(
                          localParsedProfiles.executors[selectedExecutorType] ||
                            {}
                        ).map((configuration) => (
                          <SelectItem key={configuration} value={configuration}>
                            {configuration}
                          </SelectItem>
                        ))}
                        <SelectItem value="__create__">
                          {t('settings.agents.editor.createNew')}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <Button
                      variant="destructive"
                      size="sm"
                      className="h-10"
                      onClick={() => openDeleteDialog(selectedConfiguration)}
                      disabled={
                        profilesSaving ||
                        !localParsedProfiles.executors[selectedExecutorType] ||
                        Object.keys(
                          localParsedProfiles.executors[selectedExecutorType] ||
                            {}
                        ).length <= 1
                      }
                      title={
                        Object.keys(
                          localParsedProfiles.executors[selectedExecutorType] ||
                            {}
                        ).length <= 1
                          ? t('settings.agents.editor.deleteTitle')
                          : t('settings.agents.editor.deleteButton', {
                              name: selectedConfiguration,
                            })
                      }
                    >
                      {t('settings.agents.editor.deleteText')}
                    </Button>
                  </div>
                </div>
              </div>

              {(() => {
                const executorsMap =
                  localParsedProfiles.executors as unknown as ExecutorsMap;
                return (
                  !!executorsMap[selectedExecutorType]?.[
                    selectedConfiguration
                  ]?.[selectedExecutorType] && (
                    <ExecutorConfigForm
                      key={`${selectedExecutorType}-${selectedConfiguration}`}
                      executor={selectedExecutorType}
                      value={
                        (executorsMap[selectedExecutorType][
                          selectedConfiguration
                        ][selectedExecutorType] as Record<string, unknown>) ||
                        {}
                      }
                      onChange={(formData) =>
                        handleExecutorConfigChange(
                          selectedExecutorType,
                          selectedConfiguration,
                          formData
                        )
                      }
                      onSave={handleExecutorConfigSave}
                      disabled={profilesSaving}
                      isSaving={profilesSaving}
                      isDirty={isDirty}
                    />
                  )
                );
              })()}
            </div>
          ) : (
            // Raw JSON editor
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="profiles-editor">
                  {t('settings.agents.editor.jsonLabel')}
                </Label>
                <JSONEditor
                  id="profiles-editor"
                  placeholder={t('settings.agents.editor.jsonPlaceholder')}
                  value={
                    profilesLoading
                      ? t('settings.agents.editor.jsonLoading')
                      : localProfilesContent
                  }
                  onChange={handleProfilesChange}
                  disabled={profilesLoading}
                  minHeight={300}
                />
              </div>

              {!profilesError && profilesPath && (
                <div className="space-y-2">
                  <p className="text-sm text-muted-foreground">
                    <span className="font-medium">
                      {t('settings.agents.editor.pathLabel')}
                    </span>{' '}
                    <span className="font-mono text-xs">{profilesPath}</span>
                  </p>
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {!useFormEditor && (
        <div className="sticky bottom-0 z-10 bg-background/80 backdrop-blur-sm border-t py-4">
          <div className="flex justify-end">
            <Button
              onClick={handleSaveProfiles}
              disabled={!isDirty || profilesSaving || !!profilesError}
            >
              {profilesSaving && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('settings.agents.save.button')}
            </Button>
          </div>
        </div>
      )}

      {/* Subagents CRUD Section */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
          <div>
            <CardTitle>Subagents</CardTitle>
            <CardDescription>
              Define custom agents for workflow automation with specific roles, prompts, and capabilities.
            </CardDescription>
          </div>
          <Button onClick={openCreateSubagentDialog} size="sm">
            <Plus className="h-4 w-4 mr-2" />
            New Subagent
          </Button>
        </CardHeader>
        <CardContent>
          {subagentsError && (
            <Alert variant="destructive" className="mb-4">
              <AlertDescription>{subagentsError}</AlertDescription>
            </Alert>
          )}

          {subagentsLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
              <span className="ml-2">Loading subagents...</span>
            </div>
          ) : subagents.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No subagents defined yet.</p>
              <p className="text-sm">Create a subagent to use in workflow automation.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {subagents.map((agent) => (
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
                      onClick={() => openEditSubagentDialog(agent)}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setSubagentToDelete(agent);
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

      {/* Subagent Create/Edit Dialog */}
      <Dialog open={subagentDialogOpen} onOpenChange={setSubagentDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {editingSubagent ? 'Edit Subagent' : 'Create Subagent'}
            </DialogTitle>
            <DialogDescription>
              {editingSubagent
                ? 'Update the subagent configuration.'
                : 'Define a new subagent for workflow automation.'}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="subagent-name">Name *</Label>
              <Input
                id="subagent-name"
                placeholder="e.g., Code Reviewer"
                value={subagentForm.name}
                onChange={(e) =>
                  setSubagentForm({ ...subagentForm, name: e.target.value })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-role">Role *</Label>
              <Input
                id="subagent-role"
                placeholder="e.g., Reviews code for quality and best practices"
                value={subagentForm.role}
                onChange={(e) =>
                  setSubagentForm({ ...subagentForm, role: e.target.value })
                }
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="subagent-executor">Executor</Label>
                <Select
                  value={subagentForm.executor || 'CLAUDE_CODE'}
                  onValueChange={(value) =>
                    setSubagentForm({ ...subagentForm, executor: value })
                  }
                >
                  <SelectTrigger id="subagent-executor">
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
                <Label htmlFor="subagent-color">Color</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="subagent-color"
                    type="color"
                    className="w-12 h-10 p-1 cursor-pointer"
                    value={subagentForm.color || '#6b7280'}
                    onChange={(e) =>
                      setSubagentForm({ ...subagentForm, color: e.target.value })
                    }
                  />
                  <Input
                    placeholder="#6b7280"
                    value={subagentForm.color || ''}
                    onChange={(e) =>
                      setSubagentForm({
                        ...subagentForm,
                        color: e.target.value || null,
                      })
                    }
                    className="flex-1"
                  />
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-description">Description</Label>
              <Input
                id="subagent-description"
                placeholder="Brief description of what this agent does"
                value={subagentForm.description || ''}
                onChange={(e) =>
                  setSubagentForm({
                    ...subagentForm,
                    description: e.target.value || null,
                  })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-prompt">System Prompt *</Label>
              <Textarea
                id="subagent-prompt"
                placeholder="Instructions for the agent..."
                className="min-h-[200px] font-mono text-sm"
                value={subagentForm.system_prompt}
                onChange={(e) =>
                  setSubagentForm({
                    ...subagentForm,
                    system_prompt: e.target.value,
                  })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-start-command">Execution Instructions</Label>
              <Textarea
                id="subagent-start-command"
                placeholder="Provide a detailed list of exactly what the agent should do:

1. First, analyze the task requirements
2. Review relevant code files
3. Make necessary changes
4. Write tests if applicable
5. Commit with a clear message"
                className="min-h-[120px] font-mono text-sm"
                value={subagentForm.start_command || ''}
                onChange={(e) =>
                  setSubagentForm({
                    ...subagentForm,
                    start_command: e.target.value || null,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Best provided as a detailed list of exactly what the agent should do when starting work on a task.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-deliverable">Expected Deliverable</Label>
              <Textarea
                id="subagent-deliverable"
                placeholder="Describe what the agent should produce before handing off:

e.g., 'A detailed implementation plan written to PLAN.md that includes:
- Architecture decisions with rationale
- List of files to create/modify
- Step-by-step implementation steps
- Estimated complexity for each step'"
                className="min-h-[100px] font-mono text-sm"
                value={subagentForm.deliverable || ''}
                onChange={(e) =>
                  setSubagentForm({
                    ...subagentForm,
                    deliverable: e.target.value || null,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Defines what the agent should produce and when to stop. The agent will be told to commit and stop once this deliverable is ready.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-capabilities">
                Capabilities (comma-separated)
              </Label>
              <Input
                id="subagent-capabilities"
                placeholder="e.g., code_review, testing, documentation"
                value={subagentForm.capabilities?.join(', ') || ''}
                onChange={(e) => {
                  const caps = e.target.value
                    .split(',')
                    .map((s) => s.trim())
                    .filter(Boolean);
                  setSubagentForm({
                    ...subagentForm,
                    capabilities: caps.length > 0 ? caps : null,
                  });
                }}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="subagent-tools">Tools (comma-separated)</Label>
              <Input
                id="subagent-tools"
                placeholder="e.g., read, write, bash"
                value={subagentForm.tools?.join(', ') || ''}
                onChange={(e) => {
                  const tools = e.target.value
                    .split(',')
                    .map((s) => s.trim())
                    .filter(Boolean);
                  setSubagentForm({
                    ...subagentForm,
                    tools: tools.length > 0 ? tools : null,
                  });
                }}
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setSubagentDialogOpen(false)}
              disabled={subagentSaving}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubagentSave}
              disabled={
                subagentSaving ||
                !subagentForm.name ||
                !subagentForm.role ||
                !subagentForm.system_prompt
              }
            >
              {subagentSaving && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {editingSubagent ? 'Update' : 'Create'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Subagent</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{subagentToDelete?.name}"? This
              action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteConfirmOpen(false);
                setSubagentToDelete(null);
              }}
              disabled={subagentSaving}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleSubagentDelete}
              disabled={subagentSaving}
            >
              {subagentSaving && (
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
