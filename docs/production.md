# Production Launch Notes

Accounts Repo has three runtime services:

| Service | Purpose | Required external access |
| --- | --- | --- |
| Frontend | Static React/Vite app | Public HTTPS |
| Rust API | `/api/*` financial repository API | Behind frontend origin or reverse proxy |
| Auth service | `/api/auth/*` and `/internal/session` | Public `/api/auth/*`; private `/internal/session` to Rust API only |

Cloudflare deployment details live in `docs/cloudflare.md`.

## Required Environment

Production must set real values for:

```bash
DATABASE_URL=postgres://...
BETTER_AUTH_SECRET=<32+ character high entropy secret>
BETTER_AUTH_URL=https://accounts.cariakauntan.com
BETTER_AUTH_TRUSTED_ORIGINS=https://accounts.cariakauntan.com
AUTH_SERVICE_URL=http://127.0.0.1:8081
AUTH_INTERNAL_TOKEN=<32+ character shared internal token>
ACCOUNTS_REPO_PROXY_TOKEN=<32+ character shared proxy token>
ACCOUNTS_REPO_EMAIL_MODE=resend
RESEND_API_KEY=<resend-api-key>
ACCOUNTS_REPO_EMAIL_FROM="Accounts Repo <no-reply@your-domain>"
ACCOUNTS_REPO_BIND_ADDR=127.0.0.1:8080
CORS_ALLOWED_ORIGIN=https://accounts.cariakauntan.com
```

Optional:

```bash
ACCOUNTS_REPO_EMAIL_REPLY_TO=support@your-domain
AUTH_PORT=8081
```

Production startup fails if auth secrets, HTTPS auth URLs, trusted origins, or email delivery are not configured.

## Routing Contract

The browser calls relative URLs. Your public HTTPS host must route:

| Path | Upstream |
| --- | --- |
| `/` and static assets | Frontend build output |
| `/api/auth/*` | Auth service |
| `/api/*` | Rust API |

Keep `/internal/session` reachable only from the Rust API to the auth service. It requires `AUTH_INTERNAL_TOKEN`, but it should still not be exposed publicly.

## Current Cloudflare Hostnames

| Purpose | Hostname |
| --- | --- |
| Pages fallback app | `https://accounts-repo.pages.dev` |
| Intended custom app | `https://accounts.cariakauntan.com` |
| Rust API tunnel origin | `https://accounts-api.cariakauntan.com` |
| Auth tunnel origin | `https://accounts-auth.cariakauntan.com` |

`accounts.cariakauntan.com` is active and has been smoke-tested through the stable API/auth tunnel hostnames.

## Launch Checks

Before opening access to users:

```bash
pnpm verify
pnpm e2e
pnpm audit --prod --audit-level moderate
cargo build --release -p accounts-repo-backend
```

Run `pnpm --dir auth-service auth:migrate --yes` against the production database before first production start and after Better Auth plugin/schema changes.
