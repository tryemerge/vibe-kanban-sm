import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import {
  Loader2,
  LayoutTemplate,
  AlertTriangle,
  Check,
  GitPullRequest,
  LayoutGrid,
} from 'lucide-react';
import { templatesApi } from '@/lib/api';
import { useProjects } from '@/hooks';
import type { TemplateInfo } from 'shared/types';

// Icon mapping for template icons
const iconMap: Record<string, React.ElementType> = {
  GitPullRequest: GitPullRequest,
  LayoutGrid: LayoutGrid,
  LayoutTemplate: LayoutTemplate,
};

export function TemplatesSettings() {
  const { t } = useTranslation(['settings', 'common']);

  // Templates state
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [templatesLoading, setTemplatesLoading] = useState(true);
  const [templatesError, setTemplatesError] = useState<string | null>(null);

  // Projects from hook
  const { projects, isLoading: projectsLoading } = useProjects();

  // Apply dialog state
  const [applyDialogOpen, setApplyDialogOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<TemplateInfo | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<string>('');
  const [applying, setApplying] = useState(false);

  // Success message
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Fetch templates
  const fetchTemplates = useCallback(async () => {
    setTemplatesLoading(true);
    setTemplatesError(null);
    try {
      const result = await templatesApi.list();
      setTemplates(result);
    } catch (err) {
      console.error('Failed to fetch templates:', err);
      setTemplatesError(t('settings:templates.errors.loadFailed'));
    } finally {
      setTemplatesLoading(false);
    }
  }, [t]);

  // Load templates on mount
  useEffect(() => {
    fetchTemplates();
  }, [fetchTemplates]);

  // Open apply dialog
  const openApplyDialog = (template: TemplateInfo) => {
    setSelectedTemplate(template);
    setSelectedProjectId('');
    setApplyDialogOpen(true);
  };

  // Handle apply template
  const handleApplyTemplate = async () => {
    if (!selectedTemplate || !selectedProjectId) return;

    setApplying(true);
    setTemplatesError(null);
    try {
      const result = await templatesApi.applyToProject(
        selectedProjectId,
        selectedTemplate.id
      );
      setApplyDialogOpen(false);
      setSuccessMessage(
        t('settings:templates.apply.success', {
          agents: result.agents_created,
          columns: result.columns_created,
          transitions: result.transitions_created,
        })
      );
      setTimeout(() => setSuccessMessage(null), 5000);
    } catch (err) {
      console.error('Failed to apply template:', err);
      setTemplatesError(
        err instanceof Error
          ? err.message
          : t('settings:templates.errors.applyFailed')
      );
    } finally {
      setApplying(false);
    }
  };

  // Render template card
  const renderTemplateCard = (template: TemplateInfo) => {
    const IconComponent = iconMap[template.icon] || LayoutTemplate;

    return (
      <div
        key={template.id}
        className="border rounded-lg p-4 hover:border-primary/50 transition-colors"
      >
        <div className="flex items-start gap-4">
          <div className="h-12 w-12 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
            <IconComponent className="h-6 w-6 text-primary" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base">{template.name}</h3>
            <p className="text-sm text-muted-foreground mt-1">
              {template.description}
            </p>
            <Button
              variant="outline"
              size="sm"
              className="mt-3"
              onClick={() => openApplyDialog(template)}
              disabled={projects.length === 0}
            >
              {t('settings:templates.apply.button')}
            </Button>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {templatesError && (
        <Alert variant="destructive">
          <AlertDescription>{templatesError}</AlertDescription>
        </Alert>
      )}

      {successMessage && (
        <Alert variant="success">
          <Check className="h-4 w-4" />
          <AlertDescription className="font-medium">
            {successMessage}
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t('settings:templates.title')}</CardTitle>
          <CardDescription>
            {t('settings:templates.description')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {templatesLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
              <span className="ml-2">{t('settings:templates.loading')}</span>
            </div>
          ) : templates.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <LayoutTemplate className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>{t('settings:templates.empty')}</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {templates.map(renderTemplateCard)}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Apply Template Dialog */}
      <Dialog open={applyDialogOpen} onOpenChange={setApplyDialogOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t('settings:templates.apply.title')}
            </DialogTitle>
            <DialogDescription>
              {t('settings:templates.apply.description', {
                name: selectedTemplate?.name,
              })}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <Alert>
              <AlertTriangle className="h-4 w-4" />
              <AlertDescription>
                {t('settings:templates.apply.warning')}
              </AlertDescription>
            </Alert>

            <div className="space-y-2">
              <Label htmlFor="project-select">
                {t('settings:templates.apply.selectProject')} *
              </Label>
              <Select
                value={selectedProjectId}
                onValueChange={setSelectedProjectId}
                disabled={projectsLoading}
              >
                <SelectTrigger id="project-select">
                  <SelectValue
                    placeholder={
                      projectsLoading
                        ? t('settings:templates.apply.loadingProjects')
                        : t('settings:templates.apply.projectPlaceholder')
                    }
                  />
                </SelectTrigger>
                <SelectContent>
                  {projects.map((project) => (
                    <SelectItem key={project.id} value={project.id}>
                      {project.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setApplyDialogOpen(false)}
              disabled={applying}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button
              onClick={handleApplyTemplate}
              disabled={applying || !selectedProjectId}
            >
              {applying && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('settings:templates.apply.confirm')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
