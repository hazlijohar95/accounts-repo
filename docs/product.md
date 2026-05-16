# Product Brief

## Thesis

Build the financial-data equivalent of a source-control platform: client-owned legal entity repos where accountants prepare, review, sign, and preserve authoritative financial snapshots.

## First Slice

Malaysia Sdn Bhd year-end review pack.

## Primary User

Accounting firm preparer.

## Owner

Finance-literate client/director owns the repo, controls access, and can inspect the full accounting detail.

## Core Trust Rules

- Commits are append-only.
- Audit events are append-only.
- Corrections are new commits, never silent rewrites.
- Reviewer approval must happen before client sign-off.
- AI can suggest later, but never authorizes a commit or approval.
- Local product testing uses imported real workspace data, not preloaded demo workspaces.

## Product Reference

See `docs/github-depth-product-direction.md` for the GitHub-depth product and presentation direction.
