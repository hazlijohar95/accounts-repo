# Architecture Notes

Accounts Repo is a client-owned financial repository platform. The first production slice is a Malaysia Sdn Bhd year-end review pack: import a balanced trial balance, map it to financial statement lines, propose adjustments, review the financial statement impact, approve, sign, and preserve the evidence trail.

## Runtime Modules

| Module | Location | Responsibility |
| --- | --- | --- |
| Frontend | `frontend/` | React/Vite reviewer workspace, import flow, review pack screens, and signed export UI. |
| Auth service | `auth-service/` | Better Auth email/password sessions, organization support, CORS for the frontend, and the internal session endpoint. |
| Rust API | `backend/` | Domain workflows, API routes, authorization checks, persistence, and audit hash-chain behavior. |
| E2E harness | `e2e/`, `playwright.config.ts` | Browser coverage for the core review-pack journey with explicit local dev-auth headers. |

## Main Seams

### Authentication

Production requests authenticate through Better Auth. The Rust API never trusts browser-provided dev headers unless `ACCOUNTS_REPO_AUTH_DISABLED_DEV=1` is set. In production mode it forwards the session cookie to `AUTH_SERVICE_URL/internal/session` with `AUTH_INTERNAL_TOKEN`.

This keeps the session adapter in `auth-service/` and the authorization decisions in `backend/`.

### Persistence

`backend/src/persistence.rs` currently persists the serialized `AppStore` in `app_state_snapshots`. This is intentional for the early product slice because the domain workflow is still changing quickly.

`backend/migrations/0001_initial.sql` also defines the normalized target schema for legal entities, period branches, accounts, commits, review packs, approvals, queries, audit events, and signed exports. Integration tests guard that schema so future persistence work has a stable contract.

Do not assume every normalized table is populated by runtime code yet. The migration is the target shape; `app_state_snapshots` is the current runtime adapter.

### Frontend API Access

The frontend calls relative `/api` and `/api/auth` paths. Vite proxies those to the Rust API and auth service using `BACKEND_URL` and `AUTH_SERVICE_URL`. This keeps browser code environment-light and lets local, CI, and deployed environments swap adapters at the proxy layer.

## Quality Bar

Every change should preserve these properties:

- Financial figures must be traceable to imported source data.
- Trial balances must balance before a review pack is created.
- Commits and audit events are append-only.
- Reviewer approval must precede client sign-off.
- Dev-auth is local-only and must never be enabled silently in production.
- New behavior should be covered at the module seam closest to the risk: Rust domain/API tests for workflow rules, auth-service tests for session behavior, frontend tests for UI behavior, and Playwright tests for critical journeys.

## Scaling Direction

The next persistence deepening step is to move runtime writes from the snapshot adapter into the normalized tables while keeping the public API and domain workflow stable. That should be done as a narrow migration path, not as a broad rewrite.

When adding integrations, prefer a small interface at the seam and one concrete adapter only when it has immediate leverage. Avoid pass-through modules that only rename another module without concentrating behavior.
