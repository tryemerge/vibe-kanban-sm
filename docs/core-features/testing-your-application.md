---
title: "Testing Your Application"
description: "Live preview your web applications with embedded browser viewing and precise component selection for seamless development workflows"
---

<video controls class="w-full aspect-video rounded-xl" src="https://vkcdn.britannio.dev/showcase/flat-task-panel/vk-onb-companion-demo-3.mp4"></video>

## Overview

Preview Mode provides an embedded browser experience within Vibe Kanban, allowing you to test and iterate on your web applications without leaving the development environment. This feature eliminates the need to switch between your browser and Vibe Kanban by providing live preview capabilities and precise component selection tools.

**Key Benefits:**
- **Embedded viewing**: See your application running directly in Vibe Kanban
- **Precise component selection**: Click to select specific UI components for targeted feedback
- **Dev Server Logs**: Monitor development server output with expandable/collapsible logs at the bottom
- **Seamless workflows**: No context switching between tools

## Setting Up Preview Mode

<Steps>
<Step title="Configure Development Server Script">
  Navigate to your project settings and configure the development server script that starts your local development environment.

  **Common examples:**
  - `npm run dev` (Vite, Next.js)
  - `npm start` (Create React App)
  - `yarn dev` (Yarn projects)
  - `pnpm dev` (PNPM projects)

  ![Development server script configuration interface](/images/preview-mode-dev-script-config.png)

  ::: info
  You may also need to configure a setup script (e.g., `npm install`) to install dependencies before the development server starts. Configure this in project settings under Setup Scripts.
  :::

  ::: tip
  Ensure your development server prints the URL (e.g., `http://localhost:3000`) to stdout/stderr for automatic detection.
  :::
</Step>

<Step title="Install Web Companion">
  For precise component selection, install the `vibe-kanban-web-companion` package in your application.

  ::: info
  **Recommended**: Use the "Install companion automatically" button in the Preview tab to have Vibe Kanban create a task that installs and configures the companion for you.

  ![Install companion automatically button in Preview tab](/images/preview-mode-install-companion-button.png)
  :::

  **Manual Installation:**

  Add the dependency to your project:
  ```bash
  npm install vibe-kanban-web-companion
  ```

  Then add the companion to your application:

  ### Create React App

  Add to your `src/index.js` or `src/index.tsx`:
  ```jsx
  import { VibeKanbanWebCompanion } from 'vibe-kanban-web-companion';
  import React from 'react';
  import ReactDOM from 'react-dom/client';
  import App from './App';

  const root = ReactDOM.createRoot(document.getElementById('root'));
  root.render(
    <React.StrictMode>
      <VibeKanbanWebCompanion />
      <App />
    </React.StrictMode>
  );
  ```

  ### Next.js

  Add to your `pages/_app.js` or `pages/_app.tsx`:
  ```jsx
  import { VibeKanbanWebCompanion } from 'vibe-kanban-web-companion'
  import type { AppProps } from 'next/app'

  function MyApp({ Component, pageProps }: AppProps) {
    return (
      <>
        <VibeKanbanWebCompanion />
        <Component {...pageProps} />
      </>
    )
  }
  ```

  ### Vite

  Add to your `src/main.jsx` or `src/main.tsx`:
  ```jsx
  import { VibeKanbanWebCompanion } from "vibe-kanban-web-companion";
  import React from "react";
  import ReactDOM from "react-dom/client";
  import App from "./App";

  ReactDOM.createRoot(document.getElementById("root")).render(
    <React.StrictMode>
      <VibeKanbanWebCompanion />
      <App />
    </React.StrictMode>
  );
  ```

  ::: info
  The Web Companion is automatically tree-shaken from production builds, so it only runs in development mode.
  :::
</Step>

<Step title="Start Development Server">
  In the Preview section, click the **Start Dev Server** button to start your development server.

  ![Starting development server from task interface](/images/preview-mode-start-dev-server.png)

  The system will:
  - Launch your configured development script
  - Detect the URL of your website and load it
</Step>
</Steps>

## Using Preview Mode

### Accessing the Preview

Once your development server is running and a URL is detected:

1. **Click the Preview button** (eye icon) in the task interface
2. **View embedded application** in the iframe
3. **Interact with your app** directly within Vibe Kanban

![Preview mode showing embedded application with toolbar controls](/images/vk-preview-interface.png)

### Preview Toolbar Controls

The preview toolbar provides essential controls for managing your preview experience:

![Preview toolbar showing refresh, copy URL, open in browser, and stop server controls](/images/vk-preview-toolbar.png)

- **Refresh**: Reload the preview iframe
- **Copy URL**: Copy the development server URL to clipboard
- **Open in Browser**: Open the application in your default browser
- **Stop Dev Server**: Stop the running development server

### Dev Server Logs

At the bottom of the Preview panel, you'll find Dev Server Logs that can be expanded or collapsed. These logs show real-time output from your development server, making it easy to monitor server activity, errors, and debugging information without leaving the preview.

![Dev Server Logs showing expandable/collapsible log output at bottom of preview](/images/vk-dev-server-logs.png)

### Component Selection

When the Web Companion is installed, you can precisely select UI components for targeted feedback:

<Steps>
<Step title="Activate Selection Mode">
  Click the floating Vibe Kanban companion button in the bottom-right corner of your application to activate component selection mode.

  ![Component selection interface showing selectable elements highlighted](/images/vk-component-selection.png)
</Step>

<Step title="Choose Component Depth">
  When you click a component, Vibe Kanban shows a hierarchy of components from innermost to outermost. Select the appropriate level for your feedback:

  - **Inner components**: For specific UI elements (buttons, inputs)
  - **Outer components**: For broader sections (cards, layouts)

  ![Component depth selection showing hierarchy of selectable components](/images/preview-mode-component-depth.png)
</Step>

<Step title="Provide Targeted Feedback">
  After selecting a component, write your follow-up message. The coding agent will receive:
  - **Precise DOM selector** information
  - **Component hierarchy** and source file locations
  - **Your specific instructions** about what to change

  ::: tip ✓
  No need to describe "the button in the top right" - the agent knows exactly which component you mean!
  :::
</Step>
</Steps>

## Troubleshooting

If the preview doesn't load automatically, ensure your development server prints the URL to stdout/stderr for automatic detection.

Supported URL formats:
- `http://localhost:3000`
- `https://localhost:3000`
- `http://127.0.0.1:3000`
- `http://0.0.0.0:5173`

::: info
URLs using `0.0.0.0` or `::` are automatically converted to `localhost` for embedding.
:::

## Related Documentation

- [New Task Attempts](/core-features/new-task-attempts) - Learn about task attempt lifecycle
- [Reviewing Code Changes](/core-features/reviewing-code-changes) - Analyse and review code modifications
- [Configuration & Customisation](/configuration-customisation/global-settings) - Customise Vibe Kanban settings
