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

The local backend starts with an empty in-memory store. Import a mapped trial balance from the UI to create a real review workspace for the current session; this avoids confusing demo data with built product behavior. The schema is included for the production persistence layer.

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
