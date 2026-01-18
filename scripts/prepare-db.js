#!/usr/bin/env node

const { execSync, spawnSync } = require('child_process');
const path = require('path');

const checkMode = process.argv.includes('--check');

console.log(checkMode ? 'Checking SQLx prepared queries...' : 'Preparing database for SQLx...');

// Change to backend directory
const backendDir = path.join(__dirname, '..', 'crates/db');
process.chdir(backendDir);

const CONTAINER_NAME = 'vibe-kanban-sqlx-prepare';
const DB_NAME = 'vibe_kanban_prepare';
const DB_USER = 'postgres';
const DB_PASS = 'postgres';
const DB_PORT = '5433'; // Use different port to avoid conflicts
const DATABASE_URL = `postgres://${DB_USER}:${DB_PASS}@localhost:${DB_PORT}/${DB_NAME}?sslmode=disable`;

function dockerCommand(...args) {
  const result = spawnSync('docker', args, { stdio: 'inherit' });
  return result.status === 0;
}

function cleanup() {
  console.log('Cleaning up temporary PostgreSQL container...');
  spawnSync('docker', ['rm', '-f', CONTAINER_NAME], { stdio: 'ignore' });
}

// Clean up any existing container
cleanup();

try {
  // Start PostgreSQL container
  console.log('Starting temporary PostgreSQL container...');
  const started = dockerCommand(
    'run', '-d',
    '--name', CONTAINER_NAME,
    '-e', `POSTGRES_USER=${DB_USER}`,
    '-e', `POSTGRES_PASSWORD=${DB_PASS}`,
    '-e', `POSTGRES_DB=${DB_NAME}`,
    '-p', `${DB_PORT}:5432`,
    'postgres:16-alpine'
  );

  if (!started) {
    console.error('Failed to start PostgreSQL container. Make sure Docker is running.');
    process.exit(1);
  }

  // Wait for PostgreSQL to be ready
  console.log('Waiting for PostgreSQL to be ready...');
  let ready = false;
  for (let i = 0; i < 30; i++) {
    const result = spawnSync('docker', ['exec', CONTAINER_NAME, 'pg_isready', '-U', DB_USER], { stdio: 'ignore' });
    if (result.status === 0) {
      ready = true;
      break;
    }
    spawnSync('sleep', ['1']);
  }

  if (!ready) {
    console.error('PostgreSQL failed to start in time.');
    cleanup();
    process.exit(1);
  }

  // Give the database a moment to be fully ready for connections
  spawnSync('sleep', ['2']);
  console.log(`Using database: ${DATABASE_URL}`);

  // Run migrations
  console.log('Running migrations...');
  execSync('cargo sqlx migrate run', {
    stdio: 'inherit',
    env: { ...process.env, DATABASE_URL }
  });

  // Prepare queries
  const sqlxCommand = checkMode ? 'cargo sqlx prepare --check' : 'cargo sqlx prepare';
  console.log(checkMode ? 'Checking prepared queries...' : 'Preparing queries...');
  execSync(sqlxCommand, {
    stdio: 'inherit',
    env: { ...process.env, DATABASE_URL }
  });

  console.log(checkMode ? 'SQLx check complete!' : 'Database preparation complete!');

} catch (error) {
  console.error('Error during database preparation:', error.message);
  cleanup();
  process.exit(1);
} finally {
  cleanup();
}
