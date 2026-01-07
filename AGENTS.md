# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Rust workspace crates — `server` (API + bins), `db` (SQLx models/migrations), `executors`, `services`, `utils`, `deployment`, `local-deployment`, `remote`.
- `frontend/`: React + TypeScript app (Vite, Tailwind). Source in `frontend/src`.
- `frontend/src/components/dialogs`: Dialog components for the frontend.
- `remote-frontend/`: Remote deployment frontend.
- `shared/`: Generated TypeScript types (`shared/types.ts`). Do not edit directly.
- `assets/`, `dev_assets_seed/`, `dev_assets/`: Packaged and local dev assets.
- `npx-cli/`: Files published to the npm CLI package.
- `scripts/`: Dev helpers (ports, DB preparation).
- `docs/`: Documentation files.

## Managing Shared Types Between Rust and TypeScript

ts-rs allows you to derive TypeScript types from Rust structs/enums. By annotating your Rust types with #[derive(TS)] and related macros, ts-rs will generate .ts declaration files for those types.
Do not manually edit shared/types.ts, instead edit crates/server/src/bin/generate_types.rs

### Regenerating TypeScript Types

When you modify Rust structs that derive `TS` (like adding a field to `UpdateProject`), you must regenerate the TypeScript types:

```bash
# Requires DATABASE_URL to be set for SQLx query validation
DATABASE_URL="sqlite:///Users/the_dusky/code/emerge/vibe-kanban-sm/dev_assets/db.sqlite" pnpm run generate-types
```

**Common issues when regenerating types:**

1. **Missing DATABASE_URL**: SQLx macros need a database connection to validate queries. Set `DATABASE_URL` to point to your dev database.

2. **Missing fields in struct usages**: If you add a field to a struct like `UpdateProject`, you must also update all places that construct that struct. Search for usages:
   ```bash
   # Find all places constructing the struct
   grep -r "UpdateProject {" crates/
   ```
   Then add the new field (typically with `None` for optional fields) to each usage.

3. **SQLx cache outdated**: If queries changed, you may need to run `pnpm run prepare-db` first to update the SQLx offline cache.

4. **Use `null` not `undefined` for optional fields**: Rust `Option<T>` types are exported as `T | null` in TypeScript. Always use `null` (not `undefined`) when setting optional fields to empty. Example: `board_id: value || null` NOT `board_id: value || undefined`.

## Build, Test, and Development Commands
- Install: `pnpm i`
- Run dev (frontend + backend with ports auto-assigned): `pnpm run dev`
- Backend (watch): `pnpm run backend:dev:watch`
- Frontend (dev): `pnpm run frontend:dev`
- Type checks: `pnpm run check` (frontend) and `pnpm run backend:check` (Rust cargo check)
- Rust tests: `cargo test --workspace`
- Generate TS types from Rust: `pnpm run generate-types` (or `generate-types:check` in CI)
- Prepare SQLx (offline): `pnpm run prepare-db` — **ALWAYS use this command, never try manual cargo sqlx commands**
- Prepare SQLx (remote package, postgres): `pnpm run remote:prepare-db`
- Local NPX build: `pnpm run build:npx` then `pnpm pack` in `npx-cli/`

## Coding Style & Naming Conventions
- Rust: `rustfmt` enforced (`rustfmt.toml`); group imports by crate; snake_case modules, PascalCase types.
- TypeScript/React: ESLint + Prettier (2 spaces, single quotes, 80 cols). PascalCase components, camelCase vars/functions, kebab-case file names where practical.
- Keep functions small, add `Debug`/`Serialize`/`Deserialize` where useful.

## Testing Guidelines
- Rust: prefer unit tests alongside code (`#[cfg(test)]`), run `cargo test --workspace`. Add tests for new logic and edge cases.
- Frontend: ensure `pnpm run check` and `pnpm run lint` pass. If adding runtime logic, include lightweight tests (e.g., Vitest) in the same directory.

## Security & Config Tips
- Use `.env` for local overrides; never commit secrets. Key envs: `FRONTEND_PORT`, `BACKEND_PORT`, `HOST`
- Dev ports and assets are managed by `scripts/setup-dev-environment.js`.

## CRITICAL: Database Safety Rules

**NEVER touch `dev_assets/db.sqlite` without explicit user permission.** This file contains the user's development data.

- `pnpm run prepare-db` is SAFE — it creates a separate temp database at `crates/db/prepare_db.sqlite` for SQLx cache generation. It does NOT touch `dev_assets/db.sqlite`.
- **NEVER run `rm dev_assets/db.sqlite` or `cp dev_assets_seed/db.sqlite dev_assets/db.sqlite`** — this wipes all user data.
- **NEVER restore from seed** without asking the user first.
- If the API returns empty data but the database file has data, investigate the connection path — don't assume the database is broken.

## CRITICAL: Migration Testing Rules

**ALWAYS test migrations with `pnpm run prepare-db` BEFORE telling the user to restart the server.**

- `pnpm run prepare-db` runs all migrations against a fresh temp database — if it fails, the migration has bugs
- FK constraint errors (code 787) usually mean INSERT order is wrong or referenced rows don't exist
- Use `INSERT OR IGNORE` for seed data to handle re-runs gracefully
- Use direct `VALUES()` instead of `SELECT ... WHERE NOT EXISTS` for simpler FK-safe inserts
- If a migration fails on the user's dev database, the migration entry may not be in `_sqlx_migrations` — fix the migration and have the user restart
