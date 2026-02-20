import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

const config = defineConfig({
  title: 'InDusk',
  description: 'An advanced fork of Vibe Kanban with workflow automation, agent orchestration, and continuous knowledge improvement',
  base: '/',
  lastUpdated: true,
  cleanUrls: true,
  ignoreDeadLinks: true,

  head: [
    ['link', { rel: 'icon', type: 'image/png', href: '/logo/v-192.png' }],
  ],

  srcExclude: [
    '**/adr/**',
    '**/impl/**',
    '**/journal/**',
    '**/architecture/**',
    '**/infinite-dusky/**',
    '**/AGENTS.md',
    '**/README.md',
    '**/CLAUDE.md',
    '**/docs.json',
    '**/whitepaper.md',
  ],

  markdown: {
    lineNumbers: true,
    container: {
      tipLabel: 'Tip',
      warningLabel: 'Warning',
      dangerLabel: 'Danger',
      infoLabel: 'Info',
      detailsLabel: 'Details',
    },
  },

  mermaid: {
    theme: 'default',
    securityLevel: 'loose',
    startOnLoad: true,
    maxTextSize: 50000,
    flowchart: {
      useMaxWidth: true,
      htmlLabels: true,
    },
    themeVariables: {
      nodeTextColor: '#000000',
      mainBkg: '#ffffff',
      textColor: '#000000',
      classFontColor: '#000000',
      labelTextColor: '#000000',
      stateLabelColor: '#000000',
      entityTextColor: '#000000',
      flowchartTextColor: '#000000',
    },
  },

  themeConfig: {
    search: {
      provider: 'local',
    },

    logo: {
      light: '/logo/light.svg',
      dark: '/logo/dark.svg',
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/BloopAI/vibe-kanban' },
    ],

    footer: {
      message: 'InDusk — Enhanced fork of Vibe Kanban',
    },

    nav: [
      { text: 'Vibe Kanban', link: '/', activeMatch: '^/(?!indusk)' },
      { text: 'InDusk', link: '/indusk/about', activeMatch: '/indusk/' },
    ],

    sidebar: {
      '/indusk/': [
        {
          text: 'Overview',
          items: [
            { text: 'Welcome', link: '/indusk/about' },
            { text: 'The Story', link: '/indusk/the-story' },
            { text: 'How It Works', link: '/indusk/how-it-works' },
          ],
        },
        {
          text: 'Core Systems',
          items: [
            { text: 'Workflow Engine', link: '/indusk/workflow-engine' },
            { text: 'Context System', link: '/indusk/context-system' },
          ],
        },
      ],
      '/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Home', link: '/' },
            { text: 'Installation', link: '/getting-started' },
            { text: 'Supported Agents', link: '/supported-coding-agents' },
          ],
        },
        {
          text: 'Agents',
          collapsed: true,
          items: [
            { text: 'Claude Code', link: '/agents/claude-code' },
            { text: 'OpenAI Codex', link: '/agents/openai-codex' },
            { text: 'GitHub Copilot', link: '/agents/github-copilot' },
            { text: 'Gemini CLI', link: '/agents/gemini-cli' },
            { text: 'Amp', link: '/agents/amp' },
            { text: 'Cursor CLI', link: '/agents/cursor-cli' },
            { text: 'Opencode', link: '/agents/opencode' },
            { text: 'Droid', link: '/agents/droid' },
            { text: 'CCR', link: '/agents/ccr' },
            { text: 'Qwen Code', link: '/agents/qwen-code' },
          ],
        },
        {
          text: 'Core Features',
          items: [
            { text: 'Creating Projects', link: '/core-features/creating-projects' },
            { text: 'Creating Tasks', link: '/core-features/creating-tasks' },
            { text: 'Monitoring Execution', link: '/core-features/monitoring-task-execution' },
            { text: 'Testing Your App', link: '/core-features/testing-your-application' },
            { text: 'Reviewing Changes', link: '/core-features/reviewing-code-changes' },
            { text: 'Completing a Task', link: '/core-features/completing-a-task' },
          ],
        },
        {
          text: 'Advanced Features',
          collapsed: true,
          items: [
            { text: 'Subtasks', link: '/core-features/subtasks' },
            { text: 'New Task Attempts', link: '/core-features/new-task-attempts' },
            { text: 'Resolving Rebase Conflicts', link: '/core-features/resolving-rebase-conflicts' },
          ],
        },
        {
          text: 'Configuration',
          collapsed: true,
          items: [
            { text: 'Global Settings', link: '/configuration-customisation/global-settings' },
            { text: 'Agent Configurations', link: '/configuration-customisation/agent-configurations' },
            { text: 'Task Tags', link: '/configuration-customisation/creating-task-tags' },
            { text: 'Keyboard Shortcuts', link: '/configuration-customisation/keyboard-shortcuts' },
          ],
        },
        {
          text: 'Integrations',
          collapsed: true,
          items: [
            { text: 'GitHub Integration', link: '/integrations/github-integration' },
            { text: 'VS Code Extension', link: '/integrations/vscode-extension' },
            { text: 'MCP Server Config', link: '/integrations/mcp-server-configuration' },
            { text: 'Vibe Kanban MCP Server', link: '/integrations/vibe-kanban-mcp-server' },
          ],
        },
      ],
    },
  },

  vite: {
    optimizeDeps: {
      include: ['mermaid'],
    },
    ssr: {
      noExternal: ['mermaid'],
    },
  },
})

export default withMermaid(config)
