---
title: "Agent Profiles & Configuration"
description: "Configure and customise coding agent variants with different settings for planning, models, and sandbox permissions"
---

Agent profiles let you define multiple named variants for each supported coding agent. Variants capture configuration differences like planning mode, model choice, and sandbox permissions that you can quickly select when creating attempts.

::: info
Agent profiles are used throughout Vibe Kanban wherever agents run: onboarding, default settings, attempt creation, and follow-ups.
:::

## Configuration Access

You can configure agent profiles in two ways through Settings -> Agents:

### Form Editor

Use the guided interface with form fields for each agent setting.

![Agent configuration form editor interface](/images/coding-agent-configurations.png)

### JSON Editor

Edit the underlying `profiles.json` file directly for advanced configurations.

![JSON editor for agent configurations](/images/coding-agent-configurations-json.png)

::: info
The configuration page displays the exact file path where your settings are stored. Vibe Kanban saves only your overrides whilst preserving built-in defaults.
:::

## Configuration Structure

The profiles configuration uses a JSON structure with an `executors` object containing agent variants:

```json profiles.json
{
  "executors": {
    "CLAUDE_CODE": {
      "DEFAULT": { "CLAUDE_CODE": { "dangerously_skip_permissions": true } },
      "PLAN":    { "CLAUDE_CODE": { "plan": true } },
      "ROUTER":  { "CLAUDE_CODE": { "claude_code_router": true, "dangerously_skip_permissions": true } }
    },
    "GEMINI": {
      "DEFAULT": { "GEMINI": { "model": "default", "yolo": true } },
      "FLASH":   { "GEMINI": { "model": "flash",   "yolo": true } }
    },
    "CODEX": {
      "DEFAULT": { "CODEX": { "sandbox": "danger-full-access" } },
      "HIGH":    { "CODEX": { "sandbox": "danger-full-access", "model_reasoning_effort": "high" } }
    }
  }
}
```

::: details Structure Rules
- **Variant names**: Case-insensitive and normalised to SCREAMING_SNAKE_CASE
- **DEFAULT variant**: Reserved and always present for each agent
- **Custom variants**: Add new variants like `PLAN`, `FLASH`, `HIGH` as needed
- **Built-in protection**: Cannot remove built-in executors, but can override values
:::

::: details Configuration Inheritance
- Your custom settings override built-in defaults
- Built-in configurations remain available as fallbacks
- Each variant contains a complete configuration object for its agent
:::

## Agent Configuration Options

### CLAUDE_CODE

| Parameter | Type | Description |
|-----------|------|-------------|
| `plan` | boolean | Enable planning mode for complex tasks |
| `claude_code_router` | boolean | Route requests across multiple Claude Code instances |
| `dangerously_skip_permissions` | boolean | Skip permission prompts (use with caution) |

[View full CLI reference](https://docs.anthropic.com/en/docs/claude-code/cli-reference#cli-flags)

### GEMINI

| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | Choose model variant: `"default"` or `"flash"` |
| `yolo` | boolean | Run without confirmations |

[View full CLI reference](https://google-gemini.github.io/gemini-cli/)

### AMP

| Parameter | Type | Description |
|-----------|------|-------------|
| `dangerously_allow_all` | boolean | Allow all actions without restrictions (unsafe) |

[View full documentation](https://ampcode.com/manual#cli)

### CODEX

| Parameter | Type | Description |
|-----------|------|-------------|
| `sandbox` | string | Execution environment: `"read-only"`, `"workspace-write"`, or `"danger-full-access"` |
| `approval` | string | Approval level: `"untrusted"`, `"on-failure"`, `"on-request"`, or `"never"` |
| `model_reasoning_effort` | string | Reasoning depth: `"low"`, `"medium"`, or `"high"` |
| `model_reasoning_summary` | string | Summary style: `"auto"`, `"concise"`, `"detailed"`, or `"none"` |

[View full documentation](https://github.com/openai/codex)

### CURSOR

| Parameter | Type | Description |
|-----------|------|-------------|
| `force` | boolean | Force execution without confirmation |
| `model` | string | Specify model to use |

[View full CLI reference](https://docs.cursor.com/en/cli/reference/parameters)

### OPENCODE

| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | Specify model to use |
| `agent` | string | Choose agent type |

[View full documentation](https://opencode.ai/docs/cli/#flags-1)

### QWEN_CODE

| Parameter | Type | Description |
|-----------|------|-------------|
| `yolo` | boolean | Run without confirmations |

[View full documentation](https://qwenlm.github.io/qwen-code-docs/en/cli/index)

### DROID

| Parameter | Type | Description |
|-----------|------|-------------|
| `autonomy` | string | Permission level: `"normal"`, `"low"`, `"medium"`, `"high"`, or `"skip-permissions-unsafe"` |
| `model` | string | Specify which model to use |
| `reasoning_effort` | string | Reasoning depth: `"off"`, `"low"`, `"medium"`, or `"high"` |

[View full documentation](https://docs.factory.ai/factory-cli/getting-started/overview)

### Universal Options

These options work across multiple agent types:

| Parameter | Type | Description |
|-----------|------|-------------|
| `append_prompt` | string \| null | Text appended to the system prompt |
| `base_command_override` | string \| null | Override the underlying CLI command |
| `additional_params` | string[] \| null | Additional CLI arguments to pass |

::: warning
Options prefixed with "dangerously_" bypass safety confirmations and can perform destructive actions. Use with extreme caution.
:::

## Using Agent Configurations

<CardGrid :cols="2">
<Card title="Default Configuration" icon="gear">
  Set your default agent and variant in **Settings -> General -> Default Agent Configuration** for consistent behaviour across all attempts.
</Card>

<Card title="Per-Attempt Selection" icon="rocket">
  Override defaults when creating attempts by selecting different agent/variant combinations in the attempt dialogue.
</Card>
</CardGrid>

## Related Configuration

::: info
MCP (Model Context Protocol) servers are configured separately under **Settings -> MCP Servers** but work alongside agent profiles to extend functionality.
:::

<CardGrid :cols="2">
<Card title="MCP Server Configuration" icon="server" href="/integrations/mcp-server-configuration">
  Configure MCP servers within Vibe Kanban for your coding agents
</Card>

<Card title="Vibe Kanban MCP Server" icon="plug" href="/integrations/vibe-kanban-mcp-server">
  Connect external MCP clients to Vibe Kanban's MCP server
</Card>
</CardGrid>
