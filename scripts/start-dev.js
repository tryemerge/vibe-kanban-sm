#!/usr/bin/env node

const { spawn, execSync } = require("child_process");
const path = require("path");
const net = require("net");

const ROOT_DIR = path.join(__dirname, "..");
const POSTGRES_PORT = 5432;
const POSTGRES_USER = "postgres";
const POSTGRES_PASSWORD = "postgres";
const POSTGRES_DB = "vibe_kanban";

// Colors for console output
const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  red: "\x1b[31m",
  cyan: "\x1b[36m",
};

function log(message, color = colors.reset) {
  console.log(`${color}${message}${colors.reset}`);
}

function logStep(step, message) {
  console.log(
    `${colors.cyan}[${step}]${colors.reset} ${colors.bright}${message}${colors.reset}`
  );
}

function logSuccess(message) {
  console.log(`${colors.green}✓${colors.reset} ${message}`);
}

function logError(message) {
  console.error(`${colors.red}✗${colors.reset} ${message}`);
}

/**
 * Check if Docker is running
 */
function isDockerRunning() {
  try {
    execSync("docker info", { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

/**
 * Check if PostgreSQL container is running
 */
function isPostgresContainerRunning() {
  try {
    const result = execSync(
      'docker ps --filter "name=vibe-kanban-postgres" --format "{{.Status}}"',
      { encoding: "utf8" }
    ).trim();
    return result.includes("Up");
  } catch {
    return false;
  }
}

/**
 * Check if a port is accepting connections
 */
function checkPort(port, host = "localhost") {
  return new Promise((resolve) => {
    const socket = new net.Socket();
    socket.setTimeout(1000);

    socket.on("connect", () => {
      socket.destroy();
      resolve(true);
    });

    socket.on("timeout", () => {
      socket.destroy();
      resolve(false);
    });

    socket.on("error", () => {
      socket.destroy();
      resolve(false);
    });

    socket.connect(port, host);
  });
}

/**
 * Wait for PostgreSQL to be ready
 */
async function waitForPostgres(maxAttempts = 30) {
  for (let i = 0; i < maxAttempts; i++) {
    const isReady = await checkPort(POSTGRES_PORT);
    if (isReady) {
      // Additional check: try to connect with psql via docker
      try {
        execSync(
          `docker exec vibe-kanban-postgres pg_isready -U ${POSTGRES_USER}`,
          { stdio: "ignore" }
        );
        return true;
      } catch {
        // Container is up but postgres isn't ready yet
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
    process.stdout.write(".");
  }
  return false;
}

/**
 * Start PostgreSQL via Docker Compose
 */
async function startPostgres() {
  logStep("1/4", "Starting PostgreSQL...");

  if (!isDockerRunning()) {
    logError("Docker is not running. Please start Docker Desktop and try again.");
    process.exit(1);
  }

  if (isPostgresContainerRunning()) {
    logSuccess("PostgreSQL container already running");
    return;
  }

  // Start docker compose in detached mode
  try {
    execSync("docker compose up -d", {
      cwd: ROOT_DIR,
      stdio: "inherit",
    });
  } catch (error) {
    logError("Failed to start PostgreSQL container");
    console.error(error.message);
    process.exit(1);
  }

  // Wait for PostgreSQL to be ready
  process.stdout.write("  Waiting for PostgreSQL to be ready");
  const ready = await waitForPostgres();
  console.log(); // New line after dots

  if (!ready) {
    logError("PostgreSQL failed to start within timeout");
    process.exit(1);
  }

  logSuccess("PostgreSQL is ready");
}

/**
 * Get dev ports from setup script
 */
async function getDevPorts() {
  logStep("2/4", "Setting up dev environment...");

  const { getPorts } = require("./setup-dev-environment.js");
  const ports = await getPorts();

  logSuccess(`Frontend port: ${ports.frontend}`);
  logSuccess(`Backend port: ${ports.backend}`);

  return ports;
}

/**
 * Build DATABASE_URL
 */
function getDatabaseUrl() {
  return `postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT}/${POSTGRES_DB}`;
}

/**
 * Run database migrations
 */
function runMigrations() {
  logStep("3/4", "Running database migrations...");

  const databaseUrl = getDatabaseUrl();

  try {
    // Run the server briefly to apply migrations (it does this on startup)
    // Or we could add a dedicated migration command
    // For now, migrations are applied automatically when the server starts
    logSuccess("Migrations will be applied on server startup");
  } catch (error) {
    logError("Failed to run migrations");
    console.error(error.message);
    process.exit(1);
  }
}

/**
 * Start the development servers
 */
function startDevServers(ports) {
  logStep("4/4", "Starting development servers...");

  const databaseUrl = getDatabaseUrl();

  const env = {
    ...process.env,
    DATABASE_URL: databaseUrl,
    FRONTEND_PORT: String(ports.frontend),
    BACKEND_PORT: String(ports.backend),
  };

  log("\n" + "=".repeat(60), colors.cyan);
  log("Development servers starting...", colors.bright);
  log("=".repeat(60), colors.cyan);
  log(`  Frontend: http://localhost:${ports.frontend}`, colors.green);
  log(`  Backend:  http://localhost:${ports.backend}`, colors.green);
  log(`  Database: ${databaseUrl}`, colors.blue);
  log("=".repeat(60) + "\n", colors.cyan);

  // Use concurrently to run both servers
  const concurrently = spawn(
    "npx",
    [
      "concurrently",
      "--names",
      "backend,frontend",
      "--prefix-colors",
      "blue,green",
      "npm run backend:dev:watch",
      "npm run frontend:dev",
    ],
    {
      cwd: ROOT_DIR,
      env,
      stdio: "inherit",
      shell: false,
    }
  );

  concurrently.on("error", (error) => {
    logError(`Failed to start servers: ${error.message}`);
    process.exit(1);
  });

  concurrently.on("exit", (code) => {
    process.exit(code || 0);
  });

  // Handle Ctrl+C gracefully
  process.on("SIGINT", () => {
    log("\nShutting down...", colors.yellow);
    concurrently.kill("SIGINT");
  });

  process.on("SIGTERM", () => {
    concurrently.kill("SIGTERM");
  });
}

/**
 * Main entry point
 */
async function main() {
  console.log();
  log("=".repeat(60), colors.cyan);
  log("  Vibe Kanban Development Environment", colors.bright);
  log("=".repeat(60), colors.cyan);
  console.log();

  await startPostgres();
  const ports = await getDevPorts();
  runMigrations();
  startDevServers(ports);
}

main().catch((error) => {
  logError(error.message);
  process.exit(1);
});
