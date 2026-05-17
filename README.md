# Accounts Repo

Accounts Repo is a GitHub-inspired financial data platform for client-owned accounting repositories. The first production slice is a Malaysia Sdn Bhd year-end review pack workflow: balanced trial balance import, mappings, adjustments, financial statement impact diff, reviewer approval, client sign-off, and immutable audit trail.

## Stack

| Area | Technology | Location |
| --- | --- | --- |
| Frontend | React, Vite, Vitest, Playwright | `frontend/`, `e2e/` |
| API | Rust, Axum, SQLx, Postgres | `backend/` |
| Auth | Better Auth, Hono, Postgres | `auth-service/` |
| Workspace | pnpm, Cargo, Docker Compose | repo root |

## Prerequisites

- Node.js 22 or newer.
- pnpm 10.11.0. The repo declares this in `packageManager`.
- Rust stable with Cargo.
- Docker for local Postgres.
- Playwright browsers for E2E: `pnpm --dir frontend exec playwright install chromium`.

## Quick Start

Install dependencies:

```bash
pnpm install
```

Create and export local environment variables:

```bash
cp .env.example .env
# Replace AUTH_INTERNAL_TOKEN and BETTER_AUTH_SECRET with local random values before sourcing.
set -a; source .env; set +a
```

Start Postgres and run Better Auth migrations:

```bash
docker compose up -d postgres
pnpm --dir auth-service auth:migrate --yes
```

Run the three app processes in separate terminals. Export `.env` in each terminal first with `set -a; source .env; set +a`.

```bash
pnpm dev:auth
pnpm dev:backend
pnpm dev
```

Local URLs:

| Service | URL |
| --- | --- |
| Frontend | `http://127.0.0.1:5173` |
| Rust API | `http://127.0.0.1:8080` |
| Auth service | `http://127.0.0.1:8081` |
| Postgres | `127.0.0.1:5432` |

If another local Postgres already uses port `5432`, choose another host port and update `DATABASE_URL` before running migrations:

```bash
POSTGRES_HOST_PORT=5433 docker compose up -d postgres
DATABASE_URL=postgres://accounts_repo:accounts_repo@127.0.0.1:5433/accounts_repo pnpm --dir auth-service auth:migrate --yes
```

## Daily Commands

| Command | Purpose | Notes |
| --- | --- | --- |
| `pnpm dev` | Start the frontend | Proxies `/api` and `/api/auth` to local services. |
| `pnpm dev:auth` | Start Better Auth service | Requires `DATABASE_URL` and auth env. |
| `pnpm dev:backend` | Start Rust API | Uses Postgres when `DATABASE_URL` is set; otherwise starts in empty in-memory mode. |
| `pnpm test:rust` | Run Rust tests | Postgres integration tests require `DATABASE_URL`. |
| `pnpm test:auth` | Typecheck and test auth service | Requires `DATABASE_URL`. |
| `pnpm test:frontend` | Run frontend tests once | Uses Vitest and jsdom. |
| `pnpm build:all` | Build auth service and frontend | Useful before deployment changes. |
| `pnpm verify` | Run Rust, auth, frontend tests, then builds | Does not run browser E2E. |
| `pnpm e2e` | Run Playwright E2E | Starts isolated backend/frontend test servers with explicit dev auth. |
| `pnpm clean` | Remove generated local build/test artifacts | Does not remove `node_modules` or Cargo `target`. |

## Environment

Use `.env.example` as the source of truth for local variables. The scripts do not auto-load `.env`, so export it in each shell before running services.

Production must provide these values with real secrets and deployment URLs:

```bash
DATABASE_URL=postgres://...
BETTER_AUTH_SECRET=<32+ character high entropy secret>
BETTER_AUTH_URL=https://<frontend-host>
BETTER_AUTH_TRUSTED_ORIGINS=https://<frontend-host>
AUTH_SERVICE_URL=https://<auth-service-host>
AUTH_INTERNAL_TOKEN=<shared internal token>
ACCOUNTS_REPO_PROXY_TOKEN=<shared Pages/Tunnel proxy token>
ACCOUNTS_REPO_EMAIL_MODE=resend
RESEND_API_KEY=<resend-api-key>
ACCOUNTS_REPO_EMAIL_FROM="Accounts Repo <no-reply@your-domain>"
ACCOUNTS_REPO_BIND_ADDR=0.0.0.0:8080
CORS_ALLOWED_ORIGIN=https://<frontend-host>
```

`ACCOUNTS_REPO_AUTH_DISABLED_DEV=1` is only for explicit local or E2E dev-auth flows. Do not enable it in production.

See `docs/production.md` for launch routing and `docs/cloudflare.md` for Cloudflare Pages + Tunnel deployment.

## Database

Postgres starts from `docker-compose.yml`. Backend migrations live in `backend/migrations`.

The current runtime persistence adapter stores serialized app state in `app_state_snapshots`. The initial migration also defines the normalized target schema for the financial repository model. See `docs/architecture.md` before changing persistence so the transitional design stays clear.

## Troubleshooting

If Postgres tests fail with `role "accounts_repo" does not exist`, your local Docker volume was probably created before this compose file used the current role/database names. Docker only applies `POSTGRES_USER`, `POSTGRES_PASSWORD`, and `POSTGRES_DB` when the volume is first initialized.

To keep local data, point `DATABASE_URL` at a working Postgres database that already has the `accounts_repo` role and database. To reset only this local development database, stop the app first, then run:

```bash
docker compose down -v
docker compose up -d postgres
pnpm --dir auth-service auth:migrate --yes
```

`docker compose down -v` deletes the local compose database volume.

## Real Data Import

Run the backend and frontend, then import a CSV export. XLSX imports are disabled for launch until a dependency with a clean security posture is selected. The source file hash, parser, row count, and uploader are preserved with the review pack evidence.

The imported table must have these columns:

```csv
account_code,account_name,account_type,amount,fs_line,assertion
1000,Cash at Bank,asset,1000.00,Cash and Bank,Existence
4000,Revenue,income,-1000.00,Revenue,Completeness
```

`account_type` must be one of `asset`, `liability`, `equity`, `income`, or `expense`. Amounts must balance to zero before the backend creates the review pack.

## Product Model

| Concept | Meaning |
| --- | --- |
| Repo | One client-owned legal entity. |
| Branch | One financial period, such as FY2026 year-end. |
| Commit | Append-only financial snapshot. |
| Review pack | Accountant-native pull request. |
| Diff | Financial statement impact diff. |
| Audit trail | Immutable event stream. |
| Signed export | JSON evidence pack after client sign-off. |

## Docs

- `docs/product.md` - product thesis, first slice, user, owner, and trust rules.
- `docs/architecture.md` - runtime modules, seams, persistence transition, and quality bar.
- `docs/github-depth-product-direction.md` - broader product direction.
- `docs/design-simplification.md` - design simplification notes.
