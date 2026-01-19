# ADR 2026-01-18-004: Swim Lanes and Task Labels

## Status
Proposed

## Context
Users need better organization for tasks on the kanban board. Currently all tasks in a column are shown in a flat list. For projects with many tasks, this becomes unwieldy.

Requirements:
1. Group tasks visually (swim lanes)
2. Click task to expand inline for details
3. Collapsible lanes with remembered state
4. Flexible grouping - start with labels, but support other attributes later

## Decision

### 1. Generic Swim Lane System
Swim lanes are a UI-level grouping mechanism, not a database concept. The frontend queries tasks and groups them by a configurable attribute.

```typescript
type SwimLaneGroupBy =
  | { type: 'label'; labelId?: string }  // Group by label, optionally filter to one
  | { type: 'assignee' }                  // Future: group by assignee
  | { type: 'priority' }                  // Future: group by priority
  | { type: 'none' };                     // No grouping (current behavior)

interface SwimLaneConfig {
  groupBy: SwimLaneGroupBy;
  collapsedLanes: string[];  // IDs of collapsed lanes
  showUnlabeled: boolean;    // Show tasks without the grouping attribute
}
```

Store config per-project in localStorage: `swimLaneConfig:${projectId}`

### 2. Task Labels (First Grouping Attribute)

#### Schema
```sql
-- Labels are project-scoped
CREATE TABLE task_labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,  -- Hex color for visual distinction
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, name)
);

-- Many-to-many: tasks can have multiple labels
CREATE TABLE task_label_assignments (
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES task_labels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (task_id, label_id)
);

CREATE INDEX idx_task_label_assignments_task ON task_label_assignments(task_id);
CREATE INDEX idx_task_label_assignments_label ON task_label_assignments(label_id);
```

#### API
```
GET    /api/projects/:id/labels          - List labels for project
POST   /api/projects/:id/labels          - Create label
PUT    /api/projects/:id/labels/:id      - Update label
DELETE /api/projects/:id/labels/:id      - Delete label

POST   /api/tasks/:id/labels/:labelId    - Add label to task
DELETE /api/tasks/:id/labels/:labelId    - Remove label from task
```

### 3. Inline Task Expansion
When user clicks a task card:
1. Card expands in place (CSS transition)
2. Shows full description, labels, triggers, etc.
3. Click again or click elsewhere to collapse
4. Only one task expanded at a time per column

```typescript
interface TaskCardProps {
  task: TaskWithAttemptStatus;
  expanded: boolean;
  onToggleExpand: () => void;
}
```

### 4. Swim Lane UI Component

```typescript
interface SwimLaneProps {
  title: string;
  tasks: TaskWithAttemptStatus[];
  collapsed: boolean;
  onToggleCollapse: () => void;
  color?: string;  // Optional accent color from label
}

// In KanbanColumn:
function KanbanColumn({ column, tasks, swimLaneConfig }) {
  const groupedTasks = useMemo(() => {
    if (swimLaneConfig.groupBy.type === 'none') {
      return [{ id: 'all', title: null, tasks }];
    }
    if (swimLaneConfig.groupBy.type === 'label') {
      return groupTasksByLabel(tasks, labels);
    }
    // Future: other grouping types
  }, [tasks, swimLaneConfig, labels]);

  return (
    <div className="column">
      {groupedTasks.map(lane => (
        <SwimLane
          key={lane.id}
          title={lane.title}
          tasks={lane.tasks}
          collapsed={swimLaneConfig.collapsedLanes.includes(lane.id)}
          onToggleCollapse={() => toggleLaneCollapse(lane.id)}
        />
      ))}
    </div>
  );
}
```

### 5. Label Management UI
- Project Settings > Labels section
- Create/edit/delete labels with name + color
- Drag to reorder (affects swim lane order)

### 6. Task Label Assignment UI
- In expanded task card: label chips with + button
- Click + to show dropdown of available labels
- Click X on chip to remove
- Or: dedicated "Labels" field in task edit dialog

## Consequences

### Positive
- Flexible system supports multiple grouping strategies
- Labels are reusable across many features (filtering, search, reports)
- Inline expansion keeps context (no modal switching)
- Collapsed lanes save screen space
- Remembered state improves UX

### Negative
- More complex state management in frontend
- Need to handle edge cases (task in multiple labels = appears in multiple lanes)
- Label management adds settings overhead

### Neutral
- No changes to core task model (labels are separate table)
- Can add other grouping types without schema changes
- Works alongside existing column-based workflow

## Implementation Order

1. **Phase 1: Labels Backend**
   - Migration for task_labels and task_label_assignments
   - TaskLabel model with CRUD
   - API routes for label management
   - API routes for task-label assignment

2. **Phase 2: Labels Frontend**
   - Label management in Project Settings
   - Label display on task cards
   - Label assignment UI in task details

3. **Phase 3: Swim Lanes**
   - SwimLane component
   - SwimLaneConfig in localStorage
   - Grouping logic (by label first)
   - Collapse/expand with state persistence

4. **Phase 4: Inline Task Expansion**
   - Expandable task card component
   - Transition animations
   - Expanded state management

## Future Extensions
- Group by assignee (when multi-user is added)
- Group by priority (add priority field to tasks)
- Group by custom field (add custom_fields JSON to tasks)
- Filter + Group combination (show only "urgent" label, grouped by assignee)

## Related
- ADR 2026-01-18-001: Structured Deliverables
- ADR 2026-01-18-002: Task Auto-start Triggers
- ADR 2026-01-18-003: Agent File Locking
