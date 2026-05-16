# GitHub-Depth Product Direction

## Product Standard

Accounts Repo should feel less like a dashboard and more like a financial source-control system. GitHub is the product reference because it makes complex state inspectable: repository, branch, commit, diff, review, approval, and audit history are all first-class concepts.

## What To Borrow

- Repository pages should make custody, current branch, head commit, status, collaborators, and source data visible without explaining the whole product.
- Review packs should behave like pull requests: one active decision, visible diff, clear checks, reviewer approval before merge/sign-off, and a timeline of events.
- Commits should be append-only financial snapshots with stable hashes, authors, messages, and previous history preserved.
- Diffs should be the main review surface, not a decorative summary. Financial statement impact comes first; trial balance and mapped lines remain inspectable evidence.
- Empty states should create real objects from real user data. Avoid fake walkthroughs that blur what is implemented.

## Real Data Rule

No default seeded workspace in local development. The app starts empty and asks for a mapped trial balance import. Test data can exist inside automated tests, but the running product should only show user-imported workspace data.

## Presentation Rule

The interface should be restrained, dense, and credible: strong hierarchy, compact tabs, clear status pills, audit-friendly copy, and no ornamental hero/demo treatment. If a feature is not implemented for real data, do not present it as if it works.
