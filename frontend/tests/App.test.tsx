import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../src/App";
import type { LegalEntityRepo, RepoWorkspace } from "../src/types";

const repo: LegalEntityRepo = {
  id: "repo-1",
  owner_organization_id: "org-1",
  name: "Nusantara Precision Sdn Bhd",
  registration_number: "202001034561 (1390882-X)",
  jurisdiction: "Malaysia",
  entity_type: "Sdn Bhd",
  collaborators: [
    { user_id: "u1", display_name: "Hazli Johar", role: "owner" },
    { user_id: "u2", display_name: "Aina Rahman", role: "preparer" },
  ],
  summary: {
    active_branch_label: "FY2026 Year-End",
    head_commit_hash: "abc123def4",
    review_pack_status: "in_review",
    revenue: "-1350000.00",
    profit_before_tax: "-186100.00",
    net_assets: "718500.00",
  },
};

const workspace: RepoWorkspace = {
  repo,
  branch: {
    id: "branch-1",
    legal_entity_id: "repo-1",
    label: "FY2026 Year-End",
    period_start: "2025-07-01",
    period_end: "2026-06-30",
    status: "in_review",
    head_commit_id: "commit-2",
  },
  commits: [
    {
      id: "commit-1",
      branch_id: "branch-1",
      sequence_number: 1,
      message: "Imported trial balance",
      previous_hash: null,
      snapshot_hash: "abc123def456",
      created_by: "Aina Rahman",
      created_at: "2026-05-16T00:00:00Z",
      snapshot: {
        trial_balance: [],
        mappings: [],
        adjustments: [],
        fs_lines: [],
      },
    },
    {
      id: "commit-2",
      branch_id: "branch-1",
      sequence_number: 2,
      message: "Prepared review pack",
      previous_hash: "abc123def456",
      snapshot_hash: "def456abc123",
      created_by: "Aina Rahman",
      created_at: "2026-05-16T00:00:00Z",
      snapshot: {
        trial_balance: [
          {
            account_code: "4000",
            account_name: "Revenue",
            account_type: "income",
            amount: "-1350000.00",
            source_label: "TB",
          },
        ],
        mappings: [],
        adjustments: [],
        fs_lines: [{ fs_line: "Revenue", account_codes: ["4000"], amount: "-1350000.00" }],
      },
    },
  ],
  review_pack: {
    id: "pack-1",
    legal_entity_id: "repo-1",
    period_branch_id: "branch-1",
    commit_id: "commit-2",
    title: "FY2026 Sdn Bhd Year-End Review Pack",
    status: "in_review",
    approvals: [],
    open_queries: [
      {
        id: "query-1",
        title: "Confirm professional fee accrual",
        status: "open",
        assigned_to: "Hazli Johar",
      },
    ],
    created_by: "Aina Rahman",
    created_at: "2026-05-16T00:00:00Z",
  },
  fs_impact_diff: {
    from_commit_id: "commit-1",
    to_commit_id: "commit-2",
    changed_accounts: [],
    changed_fs_lines: [
      { fs_line: "Revenue", before: "0.00", after: "-1350000.00", change: "-1350000.00" },
    ],
    adjustment_changes: [],
    headline: {
      revenue_change: "-1350000.00",
      profit_before_tax_change: "-186100.00",
      net_assets_change: "718500.00",
    },
  },
  audit_events: [],
};

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("review pack workflow", () => {
  it("prevents client signoff button before reviewer approval is recorded", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        const payload = url.endsWith("/api/repos") ? [repo] : workspace;

        return new Response(JSON.stringify(payload), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }) satisfies typeof fetch,
    );

    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Nusantara Precision Sdn Bhd" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Approve as reviewer" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Sign as client" })).toBeDisabled();
  });
});
