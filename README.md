# Accounts Repo

Accounts Repo is a GitHub-inspired financial data platform for client-owned accounting repositories. The first production slice is a Malaysia Sdn Bhd year-end review pack workflow: trial balance to mappings, adjustments, financial-statement impact diff, reviewer approval, client sign-off, and immutable audit trail.

## Local Development

```bash
pnpm install
cargo test
cargo run -p accounts-repo-backend
pnpm dev
```

The backend runs on `http://127.0.0.1:8080` and the frontend runs on `http://127.0.0.1:5173`.

## Database

Postgres schema migrations are in `backend/migrations`. Start Postgres with:

```bash
docker compose up -d postgres
```

The current local backend uses a seeded in-memory store so the full workflow runs immediately. The schema is included for the production persistence layer.

## Product Model

- Repo: one client-owned legal entity.
- Branch: one financial period, such as FY2026 year-end.
- Commit: append-only financial snapshot.
- Review pack: accountant-native pull request.
- Diff: financial statement impact diff.
- Audit trail: immutable event stream.
