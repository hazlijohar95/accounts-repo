use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("reviewer approval is required before client sign-off")]
    ReviewerApprovalRequired,
    #[error("review pack has already been signed")]
    AlreadySigned,
    #[error("approval role has already approved this review pack")]
    DuplicateApproval,
    #[error("period branch is frozen after client sign-off")]
    FrozenBranch,
    #[error("adjustment must balance to zero")]
    UnbalancedAdjustment,
    #[error("adjustment must include at least two lines")]
    EmptyAdjustment,
    #[error("review pack has open queries")]
    BlockingQueriesOpen,
    #[error("adjustment references unknown account {0}")]
    UnknownAdjustmentAccount(String),
    #[error("adjustment references unmapped account {0}")]
    UnmappedAdjustmentAccount(String),
    #[error("adjustment reference already exists: {0}")]
    DuplicateAdjustmentReference(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub auth_user_id: Option<String>,
    pub display_name: String,
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegalEntityRepo {
    pub id: Uuid,
    pub owner_organization_id: Uuid,
    pub name: String,
    pub registration_number: String,
    pub jurisdiction: String,
    pub entity_type: String,
    pub collaborators: Vec<Collaborator>,
    pub summary: RepoSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoSummary {
    pub active_branch_label: String,
    pub head_commit_hash: String,
    pub review_pack_status: ReviewStatus,
    #[serde(with = "rust_decimal::serde::str")]
    pub revenue: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub profit_before_tax: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_assets: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Collaborator {
    pub user_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub role: RepoRole,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepoRole {
    Owner,
    Preparer,
    Reviewer,
    ClientSigner,
    Observer,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeriodBranch {
    pub id: Uuid,
    pub legal_entity_id: Uuid,
    pub label: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: BranchStatus,
    pub head_commit_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchStatus {
    Working,
    InReview,
    Frozen,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrialBalanceLine {
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub source_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountCode(String);

impl AccountCode {
    pub fn parse(value: &str) -> Result<Self, AccountCodeError> {
        let code = value.trim();
        if code.is_empty() {
            return Err(AccountCodeError::Empty);
        }
        if code.chars().any(char::is_whitespace) {
            return Err(AccountCodeError::ContainsWhitespace);
        }
        Ok(Self(code.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AccountCodeError {
    #[error("account code is required")]
    Empty,
    #[error("account code must not contain whitespace")]
    ContainsWhitespace,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mapping {
    pub account_code: String,
    pub fs_line: String,
    pub assertion: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Adjustment {
    pub id: Uuid,
    pub reference: String,
    pub description: String,
    pub rationale: String,
    pub lines: Vec<AdjustmentLine>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl Adjustment {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.lines.is_empty() {
            return Err(DomainError::EmptyAdjustment);
        }

        let total = self.lines.iter().map(|line| line.amount).sum::<Decimal>();

        if total.is_zero() {
            Ok(())
        } else {
            Err(DomainError::UnbalancedAdjustment)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdjustmentLine {
    pub account_code: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinancialSnapshot {
    pub trial_balance: Vec<TrialBalanceLine>,
    pub mappings: Vec<Mapping>,
    pub adjustments: Vec<Adjustment>,
    pub fs_lines: Vec<FinancialStatementLine>,
}

impl FinancialSnapshot {
    pub fn from_components(
        trial_balance: Vec<TrialBalanceLine>,
        mappings: Vec<Mapping>,
        adjustments: Vec<Adjustment>,
    ) -> Result<Self, DomainError> {
        for adjustment in &adjustments {
            adjustment.validate()?;
        }

        let fs_lines = compute_fs_lines(&trial_balance, &mappings, &adjustments);

        Ok(Self {
            trial_balance,
            mappings,
            adjustments,
            fs_lines,
        })
    }

    pub fn snapshot_hash(&self, previous_hash: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        if let Some(previous_hash) = previous_hash {
            hasher.update(previous_hash.as_bytes());
        }
        let payload =
            serde_json::to_vec(self).expect("snapshot serialization must be deterministic");
        hasher.update(payload);
        format!("{:x}", hasher.finalize())
    }

    pub fn adjusted_account_amounts(&self) -> BTreeMap<String, Decimal> {
        adjusted_account_amounts(&self.trial_balance, &self.adjustments)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinancialStatementLine {
    pub fs_line: String,
    pub account_codes: Vec<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Commit {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub sequence_number: u32,
    pub message: String,
    pub previous_hash: Option<String>,
    pub snapshot_hash: String,
    pub snapshot: FinancialSnapshot,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsImpactDiff {
    pub from_commit_id: Uuid,
    pub to_commit_id: Uuid,
    pub changed_accounts: Vec<AccountDiff>,
    pub changed_fs_lines: Vec<FsLineDiff>,
    pub adjustment_changes: Vec<AdjustmentDiff>,
    pub headline: DiffHeadline,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountDiff {
    pub account_code: String,
    pub account_name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub before: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub after: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub change: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsLineDiff {
    pub fs_line: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub before: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub after: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub change: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdjustmentDiff {
    pub reference: String,
    pub description: String,
    pub change_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffHeadline {
    #[serde(with = "rust_decimal::serde::str")]
    pub revenue_change: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub profit_before_tax_change: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_assets_change: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewPack {
    pub id: Uuid,
    pub legal_entity_id: Uuid,
    pub period_branch_id: Uuid,
    pub commit_id: Uuid,
    pub title: String,
    pub status: ReviewStatus,
    pub approvals: Vec<Approval>,
    pub open_queries: Vec<ReviewQuery>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl ReviewPack {
    pub fn approve_reviewer(
        &mut self,
        actor_name: String,
        note: Option<String>,
    ) -> Result<Approval, DomainError> {
        if self.status == ReviewStatus::Signed {
            return Err(DomainError::AlreadySigned);
        }
        if self.has_open_queries() {
            return Err(DomainError::BlockingQueriesOpen);
        }
        if self
            .approvals
            .iter()
            .any(|approval| approval.role == ApprovalRole::Reviewer)
        {
            return Err(DomainError::DuplicateApproval);
        }

        let approval = Approval {
            id: Uuid::new_v4(),
            role: ApprovalRole::Reviewer,
            actor_name,
            note,
            approved_at: Utc::now(),
        };
        self.approvals.push(approval.clone());
        self.status = ReviewStatus::ReviewerApproved;
        Ok(approval)
    }

    pub fn sign_client(
        &mut self,
        actor_name: String,
        note: Option<String>,
    ) -> Result<Approval, DomainError> {
        if self.status == ReviewStatus::Signed {
            return Err(DomainError::AlreadySigned);
        }
        if self.has_open_queries() {
            return Err(DomainError::BlockingQueriesOpen);
        }
        if !self
            .approvals
            .iter()
            .any(|approval| approval.role == ApprovalRole::Reviewer)
        {
            return Err(DomainError::ReviewerApprovalRequired);
        }
        if self
            .approvals
            .iter()
            .any(|approval| approval.role == ApprovalRole::ClientDirector)
        {
            return Err(DomainError::DuplicateApproval);
        }

        let approval = Approval {
            id: Uuid::new_v4(),
            role: ApprovalRole::ClientDirector,
            actor_name,
            note,
            approved_at: Utc::now(),
        };
        self.approvals.push(approval.clone());
        self.status = ReviewStatus::Signed;
        Ok(approval)
    }

    fn has_open_queries(&self) -> bool {
        self.open_queries
            .iter()
            .any(|query| query.status == QueryStatus::Open)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    InReview,
    ReviewerApproved,
    Signed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
    pub id: Uuid,
    pub role: ApprovalRole,
    pub actor_name: String,
    pub note: Option<String>,
    pub approved_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRole {
    Reviewer,
    ClientDirector,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewQuery {
    pub id: Uuid,
    pub title: String,
    pub status: QueryStatus,
    pub assigned_to: String,
    pub resolved_note: Option<String>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryStatus {
    Open,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub legal_entity_id: Uuid,
    pub sequence_number: u64,
    pub actor_user_id: Option<String>,
    pub actor_name: String,
    pub actor_email: String,
    pub event_type: AuditEventType,
    pub message: String,
    pub occurred_at: DateTime<Utc>,
    pub related_commit_id: Option<Uuid>,
    pub previous_hash: Option<String>,
    pub event_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    RepoCreated,
    BranchCreated,
    DataImported,
    CommitCreated,
    ReviewPackOpened,
    ReviewerApproved,
    ClientSigned,
    CorrectionCommitted,
    ReviewQueryOpened,
    ReviewQueryResolved,
    SignedPackExported,
}

pub fn create_commit(
    branch_id: Uuid,
    sequence_number: u32,
    previous_hash: Option<String>,
    snapshot: FinancialSnapshot,
    message: String,
    created_by: String,
) -> Commit {
    let snapshot_hash = snapshot.snapshot_hash(previous_hash.as_deref());
    Commit {
        id: Uuid::new_v4(),
        branch_id,
        sequence_number,
        message,
        previous_hash,
        snapshot_hash,
        snapshot,
        created_by,
        created_at: Utc::now(),
    }
}

pub fn compare_commits(from: &Commit, to: &Commit) -> FsImpactDiff {
    let before_accounts = from.snapshot.adjusted_account_amounts();
    let after_accounts = to.snapshot.adjusted_account_amounts();
    let account_names = to
        .snapshot
        .trial_balance
        .iter()
        .chain(from.snapshot.trial_balance.iter())
        .map(|line| (line.account_code.clone(), line.account_name.clone()))
        .collect::<BTreeMap<_, _>>();

    let account_codes = before_accounts
        .keys()
        .chain(after_accounts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let changed_accounts = account_codes
        .into_iter()
        .filter_map(|account_code| {
            let before = before_accounts
                .get(&account_code)
                .copied()
                .unwrap_or_default();
            let after = after_accounts
                .get(&account_code)
                .copied()
                .unwrap_or_default();
            let change = after - before;
            (!change.is_zero()).then(|| AccountDiff {
                account_name: account_names
                    .get(&account_code)
                    .cloned()
                    .unwrap_or_else(|| "Unknown account".to_string()),
                account_code,
                before,
                after,
                change,
            })
        })
        .collect::<Vec<_>>();

    let before_fs = fs_line_amounts(&from.snapshot.fs_lines);
    let after_fs = fs_line_amounts(&to.snapshot.fs_lines);
    let fs_line_names = before_fs
        .keys()
        .chain(after_fs.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let changed_fs_lines = fs_line_names
        .into_iter()
        .filter_map(|fs_line| {
            let before = before_fs.get(&fs_line).copied().unwrap_or_default();
            let after = after_fs.get(&fs_line).copied().unwrap_or_default();
            let change = after - before;
            (!change.is_zero()).then_some(FsLineDiff {
                fs_line,
                before,
                after,
                change,
            })
        })
        .collect::<Vec<_>>();

    let before_adjustments = from
        .snapshot
        .adjustments
        .iter()
        .map(|adjustment| adjustment.reference.clone())
        .collect::<BTreeSet<_>>();
    let adjustment_changes = to
        .snapshot
        .adjustments
        .iter()
        .filter(|adjustment| !before_adjustments.contains(&adjustment.reference))
        .map(|adjustment| AdjustmentDiff {
            reference: adjustment.reference.clone(),
            description: adjustment.description.clone(),
            change_type: "added".to_string(),
        })
        .collect::<Vec<_>>();

    FsImpactDiff {
        from_commit_id: from.id,
        to_commit_id: to.id,
        headline: DiffHeadline {
            revenue_change: fs_change(&changed_fs_lines, "Revenue"),
            profit_before_tax_change: profit_before_tax(&to.snapshot.fs_lines)
                - profit_before_tax(&from.snapshot.fs_lines),
            net_assets_change: net_assets(&to.snapshot.fs_lines)
                - net_assets(&from.snapshot.fs_lines),
        },
        changed_accounts,
        changed_fs_lines,
        adjustment_changes,
    }
}

pub fn repo_summary(
    branch: &PeriodBranch,
    head_commit: &Commit,
    review_status: ReviewStatus,
) -> RepoSummary {
    RepoSummary {
        active_branch_label: branch.label.clone(),
        head_commit_hash: short_hash(&head_commit.snapshot_hash),
        review_pack_status: review_status,
        revenue: fs_amount(&head_commit.snapshot.fs_lines, "Revenue"),
        profit_before_tax: profit_before_tax(&head_commit.snapshot.fs_lines),
        net_assets: net_assets(&head_commit.snapshot.fs_lines),
    }
}

pub fn short_hash(hash: &str) -> String {
    hash.chars().take(10).collect()
}

fn adjusted_account_amounts(
    trial_balance: &[TrialBalanceLine],
    adjustments: &[Adjustment],
) -> BTreeMap<String, Decimal> {
    let mut amounts = trial_balance
        .iter()
        .map(|line| (line.account_code.clone(), line.amount))
        .collect::<BTreeMap<_, _>>();

    for adjustment in adjustments {
        for line in &adjustment.lines {
            *amounts.entry(line.account_code.clone()).or_default() += line.amount;
        }
    }

    amounts
}

fn compute_fs_lines(
    trial_balance: &[TrialBalanceLine],
    mappings: &[Mapping],
    adjustments: &[Adjustment],
) -> Vec<FinancialStatementLine> {
    let adjusted_accounts = adjusted_account_amounts(trial_balance, adjustments);
    let account_lookup = trial_balance
        .iter()
        .map(|line| line.account_code.clone())
        .collect::<BTreeSet<_>>();

    let mut fs_lines: BTreeMap<String, FinancialStatementLine> = BTreeMap::new();
    for mapping in mappings {
        if !account_lookup.contains(&mapping.account_code) {
            continue;
        }
        let entry =
            fs_lines
                .entry(mapping.fs_line.clone())
                .or_insert_with(|| FinancialStatementLine {
                    fs_line: mapping.fs_line.clone(),
                    account_codes: Vec::new(),
                    amount: Decimal::ZERO,
                });
        entry.account_codes.push(mapping.account_code.clone());
        entry.amount += adjusted_accounts
            .get(&mapping.account_code)
            .copied()
            .unwrap_or_default();
    }

    fs_lines.into_values().collect()
}

fn fs_line_amounts(lines: &[FinancialStatementLine]) -> BTreeMap<String, Decimal> {
    lines
        .iter()
        .map(|line| (line.fs_line.clone(), line.amount))
        .collect()
}

fn fs_amount(lines: &[FinancialStatementLine], fs_line: &str) -> Decimal {
    lines
        .iter()
        .find(|line| line.fs_line == fs_line)
        .map(|line| line.amount)
        .unwrap_or_default()
}

fn fs_change(lines: &[FsLineDiff], fs_line: &str) -> Decimal {
    lines
        .iter()
        .find(|line| line.fs_line == fs_line)
        .map(|line| line.change)
        .unwrap_or_default()
}

fn profit_before_tax(lines: &[FinancialStatementLine]) -> Decimal {
    fs_amount(lines, "Revenue")
        + fs_amount(lines, "Cost of Sales")
        + fs_amount(lines, "Administrative Expenses")
        + fs_amount(lines, "Depreciation")
        + fs_amount(lines, "Finance Costs")
}

fn net_assets(lines: &[FinancialStatementLine]) -> Decimal {
    fs_amount(lines, "Cash and Bank")
        + fs_amount(lines, "Trade Receivables")
        + fs_amount(lines, "Inventories")
        + fs_amount(lines, "Property, Plant and Equipment")
        + fs_amount(lines, "Accumulated Depreciation")
        + fs_amount(lines, "Trade Payables")
        + fs_amount(lines, "Accruals")
        + fs_amount(lines, "Tax Payable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    #[test]
    fn rejects_unbalanced_adjustment_to_prevent_money_appearing_in_snapshot() {
        let adjustment = Adjustment {
            id: Uuid::new_v4(),
            reference: "AJ-BAD".to_string(),
            description: "Unbalanced entry".to_string(),
            rationale: "Test".to_string(),
            lines: vec![AdjustmentLine {
                account_code: "6000".to_string(),
                amount: dec("100.00"),
            }],
            created_by: "Test".to_string(),
            created_at: Utc::now(),
        };

        assert_eq!(
            adjustment.validate(),
            Err(DomainError::UnbalancedAdjustment)
        );
    }

    #[test]
    fn rejects_client_signoff_before_reviewer_approval_to_prevent_unreviewed_accounts() {
        let mut review_pack = ReviewPack {
            id: Uuid::new_v4(),
            legal_entity_id: Uuid::new_v4(),
            period_branch_id: Uuid::new_v4(),
            commit_id: Uuid::new_v4(),
            title: "FY2026 Review Pack".to_string(),
            status: ReviewStatus::InReview,
            approvals: vec![],
            open_queries: vec![],
            created_by: "Aina".to_string(),
            created_at: Utc::now(),
        };

        let result = review_pack.sign_client("Hazli".to_string(), None);

        assert_eq!(result, Err(DomainError::ReviewerApprovalRequired));
    }

    #[test]
    fn ensures_fs_diff_exposes_adjustment_impact_for_reviewers() {
        let branch_id = Uuid::new_v4();
        let tb = vec![
            TrialBalanceLine {
                account_code: "4000".to_string(),
                account_name: "Revenue".to_string(),
                account_type: AccountType::Income,
                amount: dec("-1000.00"),
                source_label: "TB".to_string(),
            },
            TrialBalanceLine {
                account_code: "6100".to_string(),
                account_name: "Professional Fees".to_string(),
                account_type: AccountType::Expense,
                amount: dec("100.00"),
                source_label: "TB".to_string(),
            },
            TrialBalanceLine {
                account_code: "2100".to_string(),
                account_name: "Accruals".to_string(),
                account_type: AccountType::Liability,
                amount: dec("-100.00"),
                source_label: "TB".to_string(),
            },
        ];
        let mappings = vec![
            Mapping {
                account_code: "4000".to_string(),
                fs_line: "Revenue".to_string(),
                assertion: "Completeness".to_string(),
            },
            Mapping {
                account_code: "6100".to_string(),
                fs_line: "Administrative Expenses".to_string(),
                assertion: "Cut-off".to_string(),
            },
            Mapping {
                account_code: "2100".to_string(),
                fs_line: "Accruals".to_string(),
                assertion: "Completeness".to_string(),
            },
        ];
        let before_snapshot =
            FinancialSnapshot::from_components(tb.clone(), mappings.clone(), vec![]).unwrap();
        let adjustment = Adjustment {
            id: Uuid::new_v4(),
            reference: "AJ-001".to_string(),
            description: "Accrue audit fee".to_string(),
            rationale: "Invoice received after year end".to_string(),
            lines: vec![
                AdjustmentLine {
                    account_code: "6100".to_string(),
                    amount: dec("50.00"),
                },
                AdjustmentLine {
                    account_code: "2100".to_string(),
                    amount: dec("-50.00"),
                },
            ],
            created_by: "Aina".to_string(),
            created_at: Utc::now(),
        };
        let after_snapshot =
            FinancialSnapshot::from_components(tb, mappings, vec![adjustment]).unwrap();
        let before = create_commit(
            branch_id,
            1,
            None,
            before_snapshot,
            "Import TB".to_string(),
            "Aina".to_string(),
        );
        let after = create_commit(
            branch_id,
            2,
            Some(before.snapshot_hash.clone()),
            after_snapshot,
            "Post accrual".to_string(),
            "Aina".to_string(),
        );

        let diff = compare_commits(&before, &after);

        assert_eq!(diff.adjustment_changes.len(), 1);
        assert!(diff.changed_fs_lines.iter().any(|line| {
            line.fs_line == "Administrative Expenses" && line.change == dec("50.00")
        }));
        assert_eq!(diff.headline.profit_before_tax_change, dec("50.00"));
    }
}
