# Accounts Repo

Accounts Repo is a GitHub-inspired financial data platform for client-owned accounting repositories. The first production slice is a Malaysia Sdn Bhd year-end review pack workflow: trial balance to mappings, adjustments, financial-statement impact diff, reviewer approval, client sign-off, and immutable audit trail.

## Local Development

```bash
pnpm install
cargo test
docker compose up -d postgres
pnpm --dir auth-service auth:migrate
pnpm dev:auth
DATABASE_URL=postgres://accounts_repo:accounts_repo@127.0.0.1:5432/accounts_repo cargo run -p accounts-repo-backend
pnpm dev
```

The auth service runs on `http://127.0.0.1:8081`, the backend runs on `http://127.0.0.1:8080`, and the frontend runs on `http://127.0.0.1:5173`.

If another local Postgres is already bound to port `5432`, start the compose database on an alternate host port and use that port in `DATABASE_URL`:

```bash
POSTGRES_HOST_PORT=5433 docker compose up -d postgres
DATABASE_URL=postgres://accounts_repo:accounts_repo@127.0.0.1:5433/accounts_repo pnpm --dir auth-service auth:migrate --yes
```

## Database

Postgres schema migrations are in `backend/migrations`. Start Postgres with:

```bash
docker compose up -d postgres
```

With `DATABASE_URL` set, the backend persists the app state to Postgres in `app_state_snapshots` and keeps commit snapshots plus audit hash-chain data in the domain payload. Without `DATABASE_URL`, the backend runs in empty in-memory mode for local smoke testing.

## Authentication

Better Auth is provided by `auth-service/` with email/password sessions and organization support. Production Rust API calls validate the Better Auth session through the auth service internal session endpoint. Local E2E uses explicit dev-auth headers only when `ACCOUNTS_REPO_AUTH_DISABLED_DEV=1` and `VITE_DEV_AUTH_EMAIL` are set.

Required production environment variables:

```bash
DATABASE_URL=postgres://accounts_repo:accounts_repo@127.0.0.1:5432/accounts_repo
BETTER_AUTH_SECRET=<32+ char high entropy secret>
BETTER_AUTH_URL=http://127.0.0.1:5173
AUTH_SERVICE_URL=http://127.0.0.1:8081
AUTH_INTERNAL_TOKEN=<shared internal token>
```

## Verification

```bash
cargo test
pnpm test:auth
pnpm test
pnpm build
pnpm e2e
```

## Real Data Import

Run the backend and frontend, then import a CSV with these columns:

```csv
account_code,account_name,account_type,amount,fs_line,assertion
1000,Cash at Bank,asset,1000.00,Cash and Bank,Existence
4000,Revenue,income,-1000.00,Revenue,Completeness
```

`account_type` must be one of `asset`, `liability`, `equity`, `income`, or `expense`. Amounts must balance to zero before the backend will create the review pack.

## Product Model

- Repo: one client-owned legal entity.
- Branch: one financial period, such as FY2026 year-end.
- Commit: append-only financial snapshot.
- Review pack: accountant-native pull request.
- Diff: financial statement impact diff.
- Audit trail: immutable event stream.
- Signed export: JSON evidence pack after client sign-off.
