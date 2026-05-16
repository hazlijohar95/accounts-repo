# Design Simplification Notes

## Removed Complexity

- Removed the always-visible year-end workflow strip because it explained the product instead of helping the current review decision. The active workflow is now expressed through review status and the next available action.
- Replaced always-visible supporting panels with native disclosure sections for commit chain, mapped FS lines, trial balance, and audit trail. Evidence remains accessible, but the first screen now focuses on diff and approval.
- Hid unavailable future actions instead of showing disabled competing buttons. Reviewer approval appears first; client sign-off appears only after reviewer approval.
- Removed decorative gradients, large hero treatment, heavy shadows, and card-on-card surfaces. The interface now relies on spacing, borders, and compact hierarchy.

## Capability Preserved

- Correction commits remain available from the FS impact diff until the branch is frozen.
- Reviewer approval and client sign-off remain enforced in order.
- Audit trail, trial balance, mapped FS lines, and commit history remain available through disclosure sections.
- Mobile readability remains covered by the Playwright mobile viewport test.

## Feedback To Monitor

- Whether reviewers need any evidence section open by default.
- Whether client directors understand that hidden sign-off means reviewer approval is still pending.
- Whether the reduced visual hierarchy still surfaces material financial-statement changes quickly enough.
