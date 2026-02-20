---
title: "Creating Task Tags"
description: "Create reusable text snippets that can be quickly inserted into task descriptions using @mentions. Task tags are available globally across all projects."
---

## What are task tags?

Task tags are reusable text snippets that you can quickly insert into task descriptions by typing `@` followed by the tag name. When you select a tag, its content is automatically inserted at your cursor position.

::: tip
Task tags use snake_case naming (no spaces allowed). For example: `bug_report`, `feature_request`, or `code_review_checklist`.
:::

## Managing task tags

Access task tags from **Settings -> General -> Task Tags**. Tags are available globally across all projects in your workspace.

![Task tags management interface showing the tag list with names and content](/images/screenshot-task-tags-manager.png)

<Steps>
<Step title="Create a new tag">
  Click **Add Tag** to create a new task tag.

  ![Create task tag dialogue showing tag name and content fields](/images/screenshot-create-task-tag.png)

  - **Tag name**: Use snake_case without spaces (e.g., `acceptance_criteria`)
  - **Content**: The text that will be inserted when the tag is used
</Step>

<Step title="Edit existing tags">
  Click the edit icon next to any tag to modify its name or content.
</Step>

<Step title="Remove unwanted tags">
  Click the delete icon to remove tags you no longer need.

  ::: warning
  Deleting a tag does not affect existing tasks that already have the tag's content inserted.
  :::
</Step>
</Steps>

## Using task tags

Insert task tags into task descriptions and follow-up messages using @mention autocomplete.

<Steps>
<Step title="Trigger autocomplete">
  When creating or editing a task description, type `@` to trigger the autocomplete dropdown.

  ![Autocomplete dropdown showing available tags after typing @ symbol](/images/screenshot-task-tag-autocomplete.png)
</Step>

<Step title="Search and select">
  Continue typing to filter tags by name, then:
  - Click on a tag to select it
  - Use arrow keys to navigate and press Enter to select
  - Press Escape to close the dropdown

  ::: tip ✓
  The tag's content is automatically inserted at your cursor position, replacing the @query.
  :::
</Step>
</Steps>

## Common use cases

::: details Bug report templates
Create a `bug_report` tag with standardised bug reporting fields:

```
**Description:**

**Steps to reproduce:**
1.
2.
3.

**Expected behaviour:**

**Actual behaviour:**

**Environment:**
```
:::

::: details Acceptance criteria checklists
Create an `acceptance_criteria` tag for feature requirements:

```
**Acceptance criteria:**
- [ ] Functionality works as specified
- [ ] Unit tests added
- [ ] Documentation updated
- [ ] Accessibility requirements met
- [ ] Performance benchmarks passed
```
:::

::: details Code review guidelines
Create a `code_review` tag with review checklist items:

```
**Code review checklist:**
- [ ] Code follows project conventions
- [ ] Tests cover edge cases
- [ ] No security vulnerabilities introduced
- [ ] Performance impact assessed
- [ ] Documentation is clear
```
:::

::: tip
Task tags work in all text fields that support the @mention feature, including task descriptions and follow-up messages, making it easy to maintain consistency across your tasks.
:::
