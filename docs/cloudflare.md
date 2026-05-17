# Cloudflare Deployment

Accounts Repo uses Cloudflare Pages for the public web app and Cloudflare Tunnel for the existing Rust API and Better Auth services. This keeps the current production code path intact while putting the public entrypoint, HTTPS, security headers, and routing on Cloudflare.

## Architecture

| Public path | Cloudflare component | Upstream |
| --- | --- | --- |
| `/` | Pages static assets from `frontend/dist` | none |
| `/api/auth/*` | Pages Function proxy | Auth service through Cloudflare Tunnel |
| `/api/*` | Pages Function proxy | Rust API through Cloudflare Tunnel |

The Pages Function at `functions/api/[[path]].js` adds `x-accounts-repo-proxy-token` before forwarding. Set the same token on both origin services so direct requests to the tunnel hostnames are rejected.

## Cloudflare Pages Project

Current Pages project:

```text
https://accounts-repo.pages.dev
```

Latest smoke deployment created during setup:

```text
https://f2350c68.accounts-repo.pages.dev
```

The intended custom app hostname is:

```text
https://accounts.cariakauntan.com
```

`accounts.cariakauntan.com` is active on the `accounts-repo` Pages project. The `cariakauntan.com` zone contains this DNS record:

| Type | Name | Target | Proxy |
| --- | --- | --- | --- |
| `CNAME` | `accounts` | `accounts-repo.pages.dev` | Proxied |

The `cariakauntan.com` apex is already attached to the separate `cariakauntan` Pages project, so do not repoint the apex for Accounts Repo unless that project is being retired.

Use these Pages settings:

| Setting | Value |
| --- | --- |
| Project name | `accounts-repo` |
| Build command | `pnpm build:frontend` |
| Build output directory | `frontend/dist` |
| Functions directory | `functions` |
| Node version | `22` |

Set these Pages environment variables:

```bash
ACCOUNTS_REPO_API_ORIGIN=https://accounts-api.cariakauntan.com
ACCOUNTS_REPO_AUTH_ORIGIN=https://accounts-auth.cariakauntan.com
ACCOUNTS_REPO_PROXY_TOKEN=<32+ character shared proxy token>
```

Local/manual deploy command after Cloudflare login:

```bash
pnpm deploy:cloudflare:pages
```

`wrangler` requires Node 22+. The repo includes `.node-version` for version managers and Cloudflare Pages.
The local deploy script uses a temporary Node 22 runner because this workstation currently has Node 20 as its default.

## Temporary Demo Bypass

The current smoke deployment is temporarily built with a fixed demo user so testers can enter the app without signing up or verifying email:

```bash
VITE_DEV_AUTH_EMAIL=demo@accounts.cariakauntan.com \
VITE_DEV_AUTH_NAME='Accounts Repo Demo' \
VITE_DEV_AUTH_ID=demo-preparer \
pnpm run deploy:cloudflare:pages
```

The loopback API origin is also temporarily started with:

```bash
ACCOUNTS_REPO_AUTH_DISABLED_DEV=1
```

This is only acceptable while the API is bound to `127.0.0.1` behind the Pages proxy and Cloudflare Tunnel. Turn it off before production use by redeploying without the `VITE_DEV_AUTH_*` variables and restarting the API without `ACCOUNTS_REPO_AUTH_DISABLED_DEV=1`.

## Current Tunnel Hostnames

Current named tunnel:

```text
accounts-repo-prod
edce55d5-c4e9-496a-85af-8c5f04c7e210
```

Current stable origin hostnames:

```text
https://accounts-api.cariakauntan.com
https://accounts-auth.cariakauntan.com
```

Current local smoke ingress config lives at `/Users/hazlijohar/.cloudflared/config.yml` and routes those hostnames to local ports `18080` and `18081`. For production, move the same tunnel credentials or a newly created tunnel to the persistent origin server and use service ports `8080` and `8081`.

## Pages.dev Fallback

Cloudflare Pages can run on `accounts-repo.pages.dev`, but named Cloudflare Tunnel public hostnames require a DNS hostname in a Cloudflare-managed zone. With `pages.dev` only, there is no stable hostname for:

```text
ACCOUNTS_REPO_API_ORIGIN
ACCOUNTS_REPO_AUTH_ORIGIN
```

For a temporary smoke test, run two quick tunnels and set the Pages variables to their generated `trycloudflare.com` URLs. These URLs change whenever the tunnel restarts and are not suitable for production.

The current deployment no longer depends on quick tunnels. The Pages project points to the stable `cariakauntan.com` tunnel hostnames above.

## Tunnel Origin Server

Run the Rust API and auth service on an origin server reachable by Cloudflare Tunnel. Keep both services bound to loopback; Cloudflare Tunnel is the public ingress.

Example `cloudflared` ingress config:

```yaml
tunnel: edce55d5-c4e9-496a-85af-8c5f04c7e210
credentials-file: /etc/cloudflared/edce55d5-c4e9-496a-85af-8c5f04c7e210.json

ingress:
  - hostname: accounts-api.cariakauntan.com
    service: http://127.0.0.1:8080
  - hostname: accounts-auth.cariakauntan.com
    service: http://127.0.0.1:8081
  - service: http_status:404
```

Create DNS routes:

```bash
cloudflared tunnel create accounts-repo-prod
cloudflared tunnel route dns accounts-repo-prod accounts-api.cariakauntan.com
cloudflared tunnel route dns accounts-repo-prod accounts-auth.cariakauntan.com
```

## Origin Environment

Rust API:

```bash
DATABASE_URL=postgres://...
AUTH_SERVICE_URL=http://127.0.0.1:8081
AUTH_INTERNAL_TOKEN=<32+ character shared internal token>
ACCOUNTS_REPO_PROXY_TOKEN=<same token as Pages>
ACCOUNTS_REPO_BIND_ADDR=127.0.0.1:8080
CORS_ALLOWED_ORIGIN=https://accounts.cariakauntan.com
```

Auth service:

```bash
NODE_ENV=production
DATABASE_URL=postgres://...
BETTER_AUTH_SECRET=<32+ character high entropy secret>
BETTER_AUTH_URL=https://accounts.cariakauntan.com
BETTER_AUTH_TRUSTED_ORIGINS=https://accounts.cariakauntan.com,https://accounts-repo.pages.dev
AUTH_INTERNAL_TOKEN=<same internal token as Rust API>
ACCOUNTS_REPO_PROXY_TOKEN=<same token as Pages>
ACCOUNTS_REPO_EMAIL_MODE=resend
RESEND_API_KEY=<resend-api-key>
ACCOUNTS_REPO_EMAIL_FROM="Accounts Repo <no-reply@your-domain>"
AUTH_PORT=8081
```

Run Better Auth migrations against the production database before first start:

```bash
pnpm --dir auth-service auth:migrate --yes
```

## Pre-Deploy Checks

```bash
pnpm verify
pnpm e2e
pnpm audit --audit-level moderate
```

The current local environment uses Node 20, so `wrangler` cannot run here until Node is switched to 22. Cloudflare Pages should use Node 22 from `.node-version`.
