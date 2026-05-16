export type ReviewStatus = "in_review" | "reviewer_approved" | "signed";
export type BranchStatus = "working" | "in_review" | "frozen";
export type RepoRole = "owner" | "preparer" | "reviewer" | "client_signer" | "observer";

export interface RepoSummary {
  active_branch_label: string;
  head_commit_hash: string;
  review_pack_status: ReviewStatus;
  revenue: string;
  profit_before_tax: string;
  net_assets: string;
}

export interface Collaborator {
  user_id: string;
  display_name: string;
  role: RepoRole;
}

export interface LegalEntityRepo {
  id: string;
  owner_organization_id: string;
  name: string;
  registration_number: string;
  jurisdiction: string;
  entity_type: string;
  collaborators: Collaborator[];
  summary: RepoSummary;
}

export interface PeriodBranch {
  id: string;
  legal_entity_id: string;
  label: string;
  period_start: string;
  period_end: string;
  status: BranchStatus;
  head_commit_id: string;
}

export interface TrialBalanceLine {
  account_code: string;
  account_name: string;
  account_type: "asset" | "liability" | "equity" | "income" | "expense";
  amount: string;
  source_label: string;
}

export interface Mapping {
  account_code: string;
  fs_line: string;
  assertion: string;
}

export interface AdjustmentLine {
  account_code: string;
  amount: string;
}

export interface Adjustment {
  id: string;
  reference: string;
  description: string;
  rationale: string;
  lines: AdjustmentLine[];
  created_by: string;
  created_at: string;
}

export interface FinancialStatementLine {
  fs_line: string;
  account_codes: string[];
  amount: string;
}

export interface FinancialSnapshot {
  trial_balance: TrialBalanceLine[];
  mappings: Mapping[];
  adjustments: Adjustment[];
  fs_lines: FinancialStatementLine[];
}

export interface Commit {
  id: string;
  branch_id: string;
  sequence_number: number;
  message: string;
  previous_hash: string | null;
  snapshot_hash: string;
  snapshot: FinancialSnapshot;
  created_by: string;
  created_at: string;
}

export interface AccountDiff {
  account_code: string;
  account_name: string;
  before: string;
  after: string;
  change: string;
}

export interface FsLineDiff {
  fs_line: string;
  before: string;
  after: string;
  change: string;
}

export interface AdjustmentDiff {
  reference: string;
  description: string;
  change_type: string;
}

export interface DiffHeadline {
  revenue_change: string;
  profit_before_tax_change: string;
  net_assets_change: string;
}

export interface FsImpactDiff {
  from_commit_id: string;
  to_commit_id: string;
  changed_accounts: AccountDiff[];
  changed_fs_lines: FsLineDiff[];
  adjustment_changes: AdjustmentDiff[];
  headline: DiffHeadline;
}

export interface Approval {
  id: string;
  role: "reviewer" | "client_director";
  actor_name: string;
  note: string | null;
  approved_at: string;
}

export interface ReviewQuery {
  id: string;
  title: string;
  status: "open" | "resolved";
  assigned_to: string;
}

export interface ReviewPack {
  id: string;
  legal_entity_id: string;
  period_branch_id: string;
  commit_id: string;
  title: string;
  status: ReviewStatus;
  approvals: Approval[];
  open_queries: ReviewQuery[];
  created_by: string;
  created_at: string;
}

export interface AuditEvent {
  id: string;
  legal_entity_id: string;
  actor_name: string;
  event_type: string;
  message: string;
  occurred_at: string;
}

export interface RepoWorkspace {
  repo: LegalEntityRepo;
  branch: PeriodBranch;
  commits: Commit[];
  review_pack: ReviewPack;
  fs_impact_diff: FsImpactDiff;
  audit_events: AuditEvent[];
}

export interface ApprovalPayload {
  actor_name: string;
  note?: string;
}

export interface CorrectionCommitPayload {
  actor_name: string;
  message: string;
  reference: string;
  description: string;
  rationale: string;
  lines: AdjustmentLine[];
}

export interface ImportTrialBalanceLine {
  account_code: string;
  account_name: string;
  account_type: TrialBalanceLine["account_type"];
  amount: string;
  fs_line: string;
  assertion: string;
}

export interface ImportWorkspacePayload {
  entity_name: string;
  registration_number: string;
  jurisdiction: string;
  entity_type: string;
  owner_name: string;
  firm_name: string;
  preparer_name: string;
  reviewer_name: string;
  client_signer_name: string;
  branch_label: string;
  period_start: string;
  period_end: string;
  source_label: string;
  trial_balance: ImportTrialBalanceLine[];
}
