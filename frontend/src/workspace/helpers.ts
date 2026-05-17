import type { RepoRole, RepoWorkspace, ReviewStatus, TrialBalanceLine } from "../types";

export function currentUserRoles(workspace: RepoWorkspace, email: string): RepoRole[] {
  return workspace.repo.collaborators
    .filter((collaborator) => collaborator.email.toLowerCase() === email.toLowerCase())
    .map((collaborator) => collaborator.role);
}

export function hasAnyRole(actual: RepoRole[], allowed: RepoRole[]) {
  return actual.some((role) => allowed.includes(role));
}

export function reviewActionMessage({
  branchFrozen,
  canApprove,
  canSign,
  status,
}: {
  branchFrozen: boolean;
  canApprove: boolean;
  canSign: boolean;
  status: ReviewStatus;
}) {
  if (branchFrozen) return "Signed branches are immutable.";
  if (status === "in_review" && !canApprove) return "Reviewer approval is waiting for an assigned reviewer.";
  if (status === "reviewer_approved" && !canSign) return "Client sign-off is waiting for the owner or signer.";
  return "Review steps are complete.";
}

export function workspaceSourceLabel(trialBalance: TrialBalanceLine[]) {
  const labels = Array.from(new Set(trialBalance.map((line) => line.source_label).filter(Boolean)));

  if (labels.length === 0) return "No imported trial balance source is attached.";
  if (labels.length === 1) return labels[0];
  return `${labels.length} imported sources`;
}
