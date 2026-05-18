export type WorkspaceTab = "review" | "commits" | "statements" | "trial-balance" | "audit";

export const WORKSPACE_TABS: Array<{ label: string; tab: WorkspaceTab }> = [
  { label: "Review", tab: "review" },
  { label: "Commits", tab: "commits" },
  { label: "Statements", tab: "statements" },
  { label: "Trial balance", tab: "trial-balance" },
  { label: "Audit", tab: "audit" },
];
