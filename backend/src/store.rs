use crate::domain::{
    AccountType, Adjustment, AdjustmentLine, Approval, AuditEvent, AuditEventType, BranchStatus,
    Collaborator, Commit, DomainError, FinancialSnapshot, FsImpactDiff, LegalEntityRepo, Mapping,
    Organization, PeriodBranch, RepoRole, ReviewPack, ReviewQuery, ReviewStatus, TrialBalanceLine,
    User, compare_commits, create_commit, repo_summary, short_hash,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("resource not found")]
    NotFound,
    #[error("invalid import: {0}")]
    InvalidImport(String),
    #[error(transparent)]
    Domain(#[from] DomainError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoWorkspace {
    pub repo: LegalEntityRepo,
    pub branch: PeriodBranch,
    pub commits: Vec<Commit>,
    pub review_pack: ReviewPack,
    pub fs_impact_diff: FsImpactDiff,
    pub audit_events: Vec<AuditEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrectionCommitRequest {
    pub actor_name: String,
    pub message: String,
    pub reference: String,
    pub description: String,
    pub rationale: String,
    pub lines: Vec<AdjustmentLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub actor_name: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceImportRequest {
    pub entity_name: String,
    pub registration_number: String,
    pub jurisdiction: String,
    pub entity_type: String,
    pub owner_name: String,
    pub firm_name: String,
    pub preparer_name: String,
    pub reviewer_name: String,
    pub client_signer_name: String,
    pub branch_label: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub source_label: String,
    pub trial_balance: Vec<WorkspaceImportTrialBalanceLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceImportTrialBalanceLine {
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub fs_line: String,
    pub assertion: String,
}

#[derive(Clone, Debug)]
pub struct AppStore {
    users: BTreeMap<Uuid, User>,
    organizations: BTreeMap<Uuid, Organization>,
    repos: BTreeMap<Uuid, LegalEntityRepo>,
    branches: BTreeMap<Uuid, PeriodBranch>,
    branch_by_repo: BTreeMap<Uuid, Uuid>,
    commits_by_branch: BTreeMap<Uuid, Vec<Commit>>,
    review_packs: BTreeMap<Uuid, ReviewPack>,
    review_pack_by_repo: BTreeMap<Uuid, Uuid>,
    audit_events_by_repo: BTreeMap<Uuid, Vec<AuditEvent>>,
}

impl AppStore {
    pub fn empty() -> Self {
        Self {
            users: BTreeMap::new(),
            organizations: BTreeMap::new(),
            repos: BTreeMap::new(),
            branches: BTreeMap::new(),
            branch_by_repo: BTreeMap::new(),
            commits_by_branch: BTreeMap::new(),
            review_packs: BTreeMap::new(),
            review_pack_by_repo: BTreeMap::new(),
            audit_events_by_repo: BTreeMap::new(),
        }
    }

    pub fn seeded() -> Self {
        let client_org_id = Uuid::new_v4();
        let firm_org_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let preparer_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();
        let signer_id = Uuid::new_v4();
        let repo_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();

        let organizations = BTreeMap::from([
            (
                client_org_id,
                Organization {
                    id: client_org_id,
                    name: "Nusantara Precision Sdn Bhd".to_string(),
                },
            ),
            (
                firm_org_id,
                Organization {
                    id: firm_org_id,
                    name: "Amjad & Hazli Advisory".to_string(),
                },
            ),
        ]);

        let users = BTreeMap::from([
            (
                owner_id,
                User {
                    id: owner_id,
                    display_name: "Hazli Johar".to_string(),
                    email: "hazli@nusantara.test".to_string(),
                },
            ),
            (
                preparer_id,
                User {
                    id: preparer_id,
                    display_name: "Aina Rahman".to_string(),
                    email: "aina@ahadvisory.test".to_string(),
                },
            ),
            (
                reviewer_id,
                User {
                    id: reviewer_id,
                    display_name: "Amjad Salleh".to_string(),
                    email: "amjad@ahadvisory.test".to_string(),
                },
            ),
            (
                signer_id,
                User {
                    id: signer_id,
                    display_name: "Nur Sofia".to_string(),
                    email: "sofia@nusantara.test".to_string(),
                },
            ),
        ]);

        let collaborators = vec![
            Collaborator {
                user_id: owner_id,
                display_name: "Hazli Johar".to_string(),
                role: RepoRole::Owner,
            },
            Collaborator {
                user_id: preparer_id,
                display_name: "Aina Rahman".to_string(),
                role: RepoRole::Preparer,
            },
            Collaborator {
                user_id: reviewer_id,
                display_name: "Amjad Salleh".to_string(),
                role: RepoRole::Reviewer,
            },
            Collaborator {
                user_id: signer_id,
                display_name: "Nur Sofia".to_string(),
                role: RepoRole::ClientSigner,
            },
        ];

        let branch = PeriodBranch {
            id: branch_id,
            legal_entity_id: repo_id,
            label: "FY2026 Year-End".to_string(),
            period_start: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
            period_end: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            status: BranchStatus::InReview,
            head_commit_id: Uuid::nil(),
        };

        let tb = seed_trial_balance();
        let mappings = seed_mappings();
        let initial_snapshot =
            FinancialSnapshot::from_components(tb.clone(), mappings.clone(), vec![])
                .expect("seeded initial snapshot must balance");
        let commit_one = create_commit(
            branch_id,
            1,
            None,
            initial_snapshot,
            "Imported FY2026 trial balance from SQL Ledger export".to_string(),
            "Aina Rahman".to_string(),
        );

        let adjustments = seed_adjustments();
        let prepared_snapshot = FinancialSnapshot::from_components(tb, mappings, adjustments)
            .expect("seeded prepared snapshot must balance");
        let commit_two = create_commit(
            branch_id,
            2,
            Some(commit_one.snapshot_hash.clone()),
            prepared_snapshot,
            "Mapped accounts and posted year-end adjustments".to_string(),
            "Aina Rahman".to_string(),
        );

        let mut branch = branch;
        branch.head_commit_id = commit_two.id;

        let review_pack_id = Uuid::new_v4();
        let review_pack = ReviewPack {
            id: review_pack_id,
            legal_entity_id: repo_id,
            period_branch_id: branch_id,
            commit_id: commit_two.id,
            title: "FY2026 Sdn Bhd Year-End Review Pack".to_string(),
            status: ReviewStatus::InReview,
            approvals: vec![],
            open_queries: vec![ReviewQuery {
                id: Uuid::new_v4(),
                title: "Confirm director approval for professional fee accrual".to_string(),
                status: crate::domain::QueryStatus::Open,
                assigned_to: "Hazli Johar".to_string(),
            }],
            created_by: "Aina Rahman".to_string(),
            created_at: Utc::now(),
        };

        let placeholder_summary = repo_summary(&branch, &commit_two, review_pack.status.clone());
        let repo = LegalEntityRepo {
            id: repo_id,
            owner_organization_id: client_org_id,
            name: "Nusantara Precision Sdn Bhd".to_string(),
            registration_number: "202001034561 (1390882-X)".to_string(),
            jurisdiction: "Malaysia".to_string(),
            entity_type: "Sdn Bhd".to_string(),
            collaborators,
            summary: placeholder_summary,
        };

        let audit_events = vec![
            audit(
                repo_id,
                "Hazli Johar",
                AuditEventType::RepoCreated,
                "Client-owned legal entity repo created",
            ),
            audit(
                repo_id,
                "Aina Rahman",
                AuditEventType::BranchCreated,
                "FY2026 period branch opened",
            ),
            audit(
                repo_id,
                "Aina Rahman",
                AuditEventType::DataImported,
                "Trial balance import attached to branch evidence",
            ),
            audit(
                repo_id,
                "Aina Rahman",
                AuditEventType::CommitCreated,
                format!(
                    "Commit {} created: {}",
                    short_hash(&commit_one.snapshot_hash),
                    commit_one.message
                ),
            ),
            audit(
                repo_id,
                "Aina Rahman",
                AuditEventType::CommitCreated,
                format!(
                    "Commit {} created: {}",
                    short_hash(&commit_two.snapshot_hash),
                    commit_two.message
                ),
            ),
            audit(
                repo_id,
                "Aina Rahman",
                AuditEventType::ReviewPackOpened,
                "FY2026 year-end review pack opened",
            ),
        ];

        Self {
            users,
            organizations,
            repos: BTreeMap::from([(repo_id, repo)]),
            branches: BTreeMap::from([(branch_id, branch)]),
            branch_by_repo: BTreeMap::from([(repo_id, branch_id)]),
            commits_by_branch: BTreeMap::from([(branch_id, vec![commit_one, commit_two])]),
            review_packs: BTreeMap::from([(review_pack_id, review_pack)]),
            review_pack_by_repo: BTreeMap::from([(repo_id, review_pack_id)]),
            audit_events_by_repo: BTreeMap::from([(repo_id, audit_events)]),
        }
    }

    pub fn import_workspace(
        &mut self,
        request: WorkspaceImportRequest,
    ) -> Result<RepoWorkspace, StoreError> {
        validate_import_request(&request)?;

        let client_org_id = Uuid::new_v4();
        let firm_org_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let preparer_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();
        let signer_id = Uuid::new_v4();
        let repo_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();

        self.organizations.insert(
            client_org_id,
            Organization {
                id: client_org_id,
                name: request.entity_name.clone(),
            },
        );
        self.organizations.insert(
            firm_org_id,
            Organization {
                id: firm_org_id,
                name: request.firm_name.clone(),
            },
        );

        self.users.insert(
            owner_id,
            User {
                id: owner_id,
                display_name: request.owner_name.clone(),
                email: user_email(&request.owner_name, "client.local"),
            },
        );
        self.users.insert(
            preparer_id,
            User {
                id: preparer_id,
                display_name: request.preparer_name.clone(),
                email: user_email(&request.preparer_name, "firm.local"),
            },
        );
        self.users.insert(
            reviewer_id,
            User {
                id: reviewer_id,
                display_name: request.reviewer_name.clone(),
                email: user_email(&request.reviewer_name, "firm.local"),
            },
        );
        self.users.insert(
            signer_id,
            User {
                id: signer_id,
                display_name: request.client_signer_name.clone(),
                email: user_email(&request.client_signer_name, "client.local"),
            },
        );

        let collaborators = vec![
            Collaborator {
                user_id: owner_id,
                display_name: request.owner_name.clone(),
                role: RepoRole::Owner,
            },
            Collaborator {
                user_id: preparer_id,
                display_name: request.preparer_name.clone(),
                role: RepoRole::Preparer,
            },
            Collaborator {
                user_id: reviewer_id,
                display_name: request.reviewer_name.clone(),
                role: RepoRole::Reviewer,
            },
            Collaborator {
                user_id: signer_id,
                display_name: request.client_signer_name.clone(),
                role: RepoRole::ClientSigner,
            },
        ];

        let baseline_snapshot = FinancialSnapshot::from_components(vec![], vec![], vec![])?;
        let baseline_commit = create_commit(
            branch_id,
            1,
            None,
            baseline_snapshot,
            format!("Opened {} period branch", request.branch_label),
            request.preparer_name.clone(),
        );

        let trial_balance = request
            .trial_balance
            .iter()
            .map(|line| TrialBalanceLine {
                account_code: line.account_code.trim().to_string(),
                account_name: line.account_name.trim().to_string(),
                account_type: line.account_type.clone(),
                amount: line.amount,
                source_label: request.source_label.trim().to_string(),
            })
            .collect::<Vec<_>>();
        let mappings = request
            .trial_balance
            .iter()
            .map(|line| Mapping {
                account_code: line.account_code.trim().to_string(),
                fs_line: line.fs_line.trim().to_string(),
                assertion: line.assertion.trim().to_string(),
            })
            .collect::<Vec<_>>();
        let imported_snapshot =
            FinancialSnapshot::from_components(trial_balance, mappings, vec![])?;
        let import_commit = create_commit(
            branch_id,
            2,
            Some(baseline_commit.snapshot_hash.clone()),
            imported_snapshot,
            format!(
                "Imported trial balance from {}",
                request.source_label.trim()
            ),
            request.preparer_name.clone(),
        );

        let branch = PeriodBranch {
            id: branch_id,
            legal_entity_id: repo_id,
            label: request.branch_label.clone(),
            period_start: request.period_start,
            period_end: request.period_end,
            status: BranchStatus::InReview,
            head_commit_id: import_commit.id,
        };
        let review_pack_id = Uuid::new_v4();
        let review_pack = ReviewPack {
            id: review_pack_id,
            legal_entity_id: repo_id,
            period_branch_id: branch_id,
            commit_id: import_commit.id,
            title: format!("{} Review Pack", request.branch_label),
            status: ReviewStatus::InReview,
            approvals: vec![],
            open_queries: vec![],
            created_by: request.preparer_name.clone(),
            created_at: Utc::now(),
        };
        let repo = LegalEntityRepo {
            id: repo_id,
            owner_organization_id: client_org_id,
            name: request.entity_name.clone(),
            registration_number: request.registration_number.clone(),
            jurisdiction: request.jurisdiction.clone(),
            entity_type: request.entity_type.clone(),
            collaborators,
            summary: repo_summary(&branch, &import_commit, review_pack.status.clone()),
        };
        let audit_events = vec![
            audit(
                repo_id,
                request.owner_name.clone(),
                AuditEventType::RepoCreated,
                "Client-owned legal entity repo created from imported source data",
            ),
            audit(
                repo_id,
                request.preparer_name.clone(),
                AuditEventType::BranchCreated,
                format!("{} period branch opened", request.branch_label),
            ),
            audit(
                repo_id,
                request.preparer_name.clone(),
                AuditEventType::DataImported,
                format!(
                    "Trial balance imported from {}",
                    request.source_label.trim()
                ),
            ),
            audit(
                repo_id,
                request.preparer_name.clone(),
                AuditEventType::CommitCreated,
                format!(
                    "Commit {} created: {}",
                    short_hash(&baseline_commit.snapshot_hash),
                    baseline_commit.message
                ),
            ),
            audit(
                repo_id,
                request.preparer_name.clone(),
                AuditEventType::CommitCreated,
                format!(
                    "Commit {} created: {}",
                    short_hash(&import_commit.snapshot_hash),
                    import_commit.message
                ),
            ),
            audit(
                repo_id,
                request.preparer_name,
                AuditEventType::ReviewPackOpened,
                "Year-end review pack opened from imported trial balance",
            ),
        ];

        self.repos.insert(repo_id, repo);
        self.branches.insert(branch_id, branch);
        self.branch_by_repo.insert(repo_id, branch_id);
        self.commits_by_branch
            .insert(branch_id, vec![baseline_commit, import_commit]);
        self.review_packs.insert(review_pack_id, review_pack);
        self.review_pack_by_repo.insert(repo_id, review_pack_id);
        self.audit_events_by_repo.insert(repo_id, audit_events);

        self.repo_workspace(repo_id)
    }

    pub fn list_repos(&self) -> Result<Vec<LegalEntityRepo>, StoreError> {
        self.repos
            .keys()
            .map(|repo_id| self.repo_view(*repo_id))
            .collect()
    }

    pub fn repo_workspace(&self, repo_id: Uuid) -> Result<RepoWorkspace, StoreError> {
        let repo = self.repo_view(repo_id)?;
        let branch_id = *self
            .branch_by_repo
            .get(&repo_id)
            .ok_or(StoreError::NotFound)?;
        let branch = self
            .branches
            .get(&branch_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let commits = self
            .commits_by_branch
            .get(&branch_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let first_commit = commits.first().ok_or(StoreError::NotFound)?;
        let head_commit = commits
            .iter()
            .find(|commit| commit.id == branch.head_commit_id)
            .ok_or(StoreError::NotFound)?;
        let review_pack_id = *self
            .review_pack_by_repo
            .get(&repo_id)
            .ok_or(StoreError::NotFound)?;
        let review_pack = self
            .review_packs
            .get(&review_pack_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let audit_events = self
            .audit_events_by_repo
            .get(&repo_id)
            .cloned()
            .unwrap_or_default();

        Ok(RepoWorkspace {
            repo,
            branch,
            fs_impact_diff: compare_commits(first_commit, head_commit),
            commits,
            review_pack,
            audit_events,
        })
    }

    pub fn review_pack(&self, review_pack_id: Uuid) -> Result<ReviewPack, StoreError> {
        self.review_packs
            .get(&review_pack_id)
            .cloned()
            .ok_or(StoreError::NotFound)
    }

    pub fn approve_reviewer(
        &mut self,
        review_pack_id: Uuid,
        request: ApprovalRequest,
    ) -> Result<Approval, StoreError> {
        let (approval, legal_entity_id) = {
            let review_pack = self
                .review_packs
                .get_mut(&review_pack_id)
                .ok_or(StoreError::NotFound)?;
            let approval =
                review_pack.approve_reviewer(request.actor_name.clone(), request.note)?;
            (approval, review_pack.legal_entity_id)
        };

        self.push_audit(
            legal_entity_id,
            request.actor_name,
            AuditEventType::ReviewerApproved,
            "Reviewer approved the frozen review pack snapshot".to_string(),
        );

        Ok(approval)
    }

    pub fn sign_client(
        &mut self,
        review_pack_id: Uuid,
        request: ApprovalRequest,
    ) -> Result<Approval, StoreError> {
        let (approval, legal_entity_id, branch_id) = {
            let review_pack = self
                .review_packs
                .get_mut(&review_pack_id)
                .ok_or(StoreError::NotFound)?;
            let approval = review_pack.sign_client(request.actor_name.clone(), request.note)?;
            (
                approval,
                review_pack.legal_entity_id,
                review_pack.period_branch_id,
            )
        };

        if let Some(branch) = self.branches.get_mut(&branch_id) {
            branch.status = BranchStatus::Frozen;
        }

        self.push_audit(
            legal_entity_id,
            request.actor_name,
            AuditEventType::ClientSigned,
            "Client director signed the review pack and froze the period branch".to_string(),
        );

        Ok(approval)
    }

    pub fn commit_correction(
        &mut self,
        repo_id: Uuid,
        branch_id: Uuid,
        request: CorrectionCommitRequest,
    ) -> Result<Commit, StoreError> {
        let branch = self
            .branches
            .get(&branch_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if branch.legal_entity_id != repo_id {
            return Err(StoreError::NotFound);
        }
        if branch.status == BranchStatus::Frozen {
            return Err(StoreError::Domain(DomainError::FrozenBranch));
        }

        let review_pack_id = *self
            .review_pack_by_repo
            .get(&repo_id)
            .ok_or(StoreError::NotFound)?;
        let review_pack = self
            .review_packs
            .get(&review_pack_id)
            .ok_or(StoreError::NotFound)?;
        if review_pack.status == ReviewStatus::Signed {
            return Err(StoreError::Domain(DomainError::FrozenBranch));
        }

        let commits = self
            .commits_by_branch
            .get(&branch_id)
            .ok_or(StoreError::NotFound)?;
        let next_sequence_number = commits.len() as u32 + 1;
        let head_commit = commits
            .iter()
            .find(|commit| commit.id == branch.head_commit_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;

        let correction = Adjustment {
            id: Uuid::new_v4(),
            reference: request.reference,
            description: request.description,
            rationale: request.rationale,
            lines: request.lines,
            created_by: request.actor_name.clone(),
            created_at: Utc::now(),
        };
        correction.validate()?;

        let mut adjustments = head_commit.snapshot.adjustments.clone();
        adjustments.push(correction);
        let snapshot = FinancialSnapshot::from_components(
            head_commit.snapshot.trial_balance.clone(),
            head_commit.snapshot.mappings.clone(),
            adjustments,
        )?;

        let commit = create_commit(
            branch_id,
            next_sequence_number,
            Some(head_commit.snapshot_hash.clone()),
            snapshot,
            request.message,
            request.actor_name.clone(),
        );

        self.commits_by_branch
            .get_mut(&branch_id)
            .ok_or(StoreError::NotFound)?
            .push(commit.clone());
        if let Some(branch) = self.branches.get_mut(&branch_id) {
            branch.head_commit_id = commit.id;
            branch.status = BranchStatus::Working;
        }
        if let Some(review_pack) = self.review_packs.get_mut(&review_pack_id) {
            review_pack.commit_id = commit.id;
            review_pack.status = ReviewStatus::InReview;
            review_pack.approvals.clear();
        }

        self.push_audit(
            repo_id,
            request.actor_name,
            AuditEventType::CorrectionCommitted,
            format!(
                "Correction commit {} appended; previous commits preserved",
                short_hash(&commit.snapshot_hash)
            ),
        );

        Ok(commit)
    }

    pub fn audit_events(&self, repo_id: Uuid) -> Result<Vec<AuditEvent>, StoreError> {
        if !self.repos.contains_key(&repo_id) {
            return Err(StoreError::NotFound);
        }
        Ok(self
            .audit_events_by_repo
            .get(&repo_id)
            .cloned()
            .unwrap_or_default())
    }

    pub fn organization_count(&self) -> usize {
        self.organizations.len()
    }

    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    fn repo_view(&self, repo_id: Uuid) -> Result<LegalEntityRepo, StoreError> {
        let mut repo = self
            .repos
            .get(&repo_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let branch_id = *self
            .branch_by_repo
            .get(&repo_id)
            .ok_or(StoreError::NotFound)?;
        let branch = self.branches.get(&branch_id).ok_or(StoreError::NotFound)?;
        let head_commit = self
            .commits_by_branch
            .get(&branch_id)
            .and_then(|commits| {
                commits
                    .iter()
                    .find(|commit| commit.id == branch.head_commit_id)
            })
            .ok_or(StoreError::NotFound)?;
        let review_pack_id = *self
            .review_pack_by_repo
            .get(&repo_id)
            .ok_or(StoreError::NotFound)?;
        let review_status = self
            .review_packs
            .get(&review_pack_id)
            .map(|pack| pack.status.clone())
            .ok_or(StoreError::NotFound)?;

        repo.summary = repo_summary(branch, head_commit, review_status);
        Ok(repo)
    }

    fn push_audit(
        &mut self,
        legal_entity_id: Uuid,
        actor_name: String,
        event_type: AuditEventType,
        message: String,
    ) {
        self.audit_events_by_repo
            .entry(legal_entity_id)
            .or_default()
            .push(audit(legal_entity_id, actor_name, event_type, message));
    }
}

fn validate_import_request(request: &WorkspaceImportRequest) -> Result<(), StoreError> {
    let required = [
        ("entity_name", request.entity_name.as_str()),
        ("registration_number", request.registration_number.as_str()),
        ("jurisdiction", request.jurisdiction.as_str()),
        ("entity_type", request.entity_type.as_str()),
        ("owner_name", request.owner_name.as_str()),
        ("firm_name", request.firm_name.as_str()),
        ("preparer_name", request.preparer_name.as_str()),
        ("reviewer_name", request.reviewer_name.as_str()),
        ("client_signer_name", request.client_signer_name.as_str()),
        ("branch_label", request.branch_label.as_str()),
        ("source_label", request.source_label.as_str()),
    ];

    for (field, value) in required {
        if value.trim().is_empty() {
            return Err(StoreError::InvalidImport(format!("{field} is required")));
        }
    }

    if request.period_start > request.period_end {
        return Err(StoreError::InvalidImport(
            "period_start must be before period_end".to_string(),
        ));
    }

    if request.trial_balance.is_empty() {
        return Err(StoreError::InvalidImport(
            "trial_balance must include at least one account".to_string(),
        ));
    }

    let mut account_codes = BTreeSet::new();
    for line in &request.trial_balance {
        let required_line = [
            ("account_code", line.account_code.as_str()),
            ("account_name", line.account_name.as_str()),
            ("fs_line", line.fs_line.as_str()),
            ("assertion", line.assertion.as_str()),
        ];
        for (field, value) in required_line {
            if value.trim().is_empty() {
                return Err(StoreError::InvalidImport(format!(
                    "{field} is required for every trial balance line"
                )));
            }
        }

        if !account_codes.insert(line.account_code.trim().to_string()) {
            return Err(StoreError::InvalidImport(format!(
                "duplicate account code {}",
                line.account_code.trim()
            )));
        }
    }

    let total = request
        .trial_balance
        .iter()
        .map(|line| line.amount)
        .sum::<Decimal>();
    if !total.is_zero() {
        return Err(StoreError::InvalidImport(
            "trial_balance must balance to zero".to_string(),
        ));
    }

    Ok(())
}

fn user_email(name: &str, domain: &str) -> String {
    let local = name
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(".");

    format!("{}@{}", local, domain)
}

fn audit(
    legal_entity_id: Uuid,
    actor_name: impl Into<String>,
    event_type: AuditEventType,
    message: impl Into<String>,
) -> AuditEvent {
    AuditEvent {
        id: Uuid::new_v4(),
        legal_entity_id,
        actor_name: actor_name.into(),
        event_type,
        message: message.into(),
        occurred_at: Utc::now(),
    }
}

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).expect("seed decimal must be valid")
}

fn seed_trial_balance() -> Vec<TrialBalanceLine> {
    vec![
        tb("1000", "Cash at Bank", AccountType::Asset, "245000.00"),
        tb("1100", "Trade Receivables", AccountType::Asset, "183500.00"),
        tb("1200", "Inventories", AccountType::Asset, "92000.00"),
        tb(
            "1500",
            "Plant and Equipment",
            AccountType::Asset,
            "380000.00",
        ),
        tb(
            "1600",
            "Accumulated Depreciation",
            AccountType::Asset,
            "-152000.00",
        ),
        tb(
            "2000",
            "Trade Payables",
            AccountType::Liability,
            "-121000.00",
        ),
        tb("2100", "Accruals", AccountType::Liability, "-68000.00"),
        tb("2200", "Tax Payable", AccountType::Liability, "-34000.00"),
        tb("3000", "Share Capital", AccountType::Equity, "-250000.00"),
        tb(
            "3100",
            "Retained Earnings",
            AccountType::Equity,
            "-175400.00",
        ),
        tb("4000", "Revenue", AccountType::Income, "-1350000.00"),
        tb("5000", "Cost of Sales", AccountType::Expense, "702000.00"),
        tb("6000", "Salaries", AccountType::Expense, "286000.00"),
        tb("6100", "Rent", AccountType::Expense, "84000.00"),
        tb(
            "6200",
            "Professional Fees",
            AccountType::Expense,
            "42000.00",
        ),
        tb(
            "6300",
            "Depreciation Expense",
            AccountType::Expense,
            "76000.00",
        ),
        tb("6400", "Bank Charges", AccountType::Expense, "3900.00"),
        tb("6500", "Tax Expense", AccountType::Expense, "56000.00"),
    ]
}

fn tb(code: &str, name: &str, account_type: AccountType, amount: &str) -> TrialBalanceLine {
    TrialBalanceLine {
        account_code: code.to_string(),
        account_name: name.to_string(),
        account_type,
        amount: dec(amount),
        source_label: "SQL Ledger TB export 2026-06-30".to_string(),
    }
}

fn seed_mappings() -> Vec<Mapping> {
    vec![
        mapping("1000", "Cash and Bank", "Existence"),
        mapping("1100", "Trade Receivables", "Recoverability"),
        mapping("1200", "Inventories", "Valuation"),
        mapping("1500", "Property, Plant and Equipment", "Existence"),
        mapping("1600", "Accumulated Depreciation", "Valuation"),
        mapping("2000", "Trade Payables", "Completeness"),
        mapping("2100", "Accruals", "Completeness"),
        mapping("2200", "Tax Payable", "Accuracy"),
        mapping("3000", "Share Capital", "Rights and obligations"),
        mapping("3100", "Retained Earnings", "Accuracy"),
        mapping("4000", "Revenue", "Completeness"),
        mapping("5000", "Cost of Sales", "Cut-off"),
        mapping("6000", "Administrative Expenses", "Accuracy"),
        mapping("6100", "Administrative Expenses", "Cut-off"),
        mapping("6200", "Administrative Expenses", "Cut-off"),
        mapping("6300", "Depreciation", "Accuracy"),
        mapping("6400", "Finance Costs", "Accuracy"),
        mapping("6500", "Tax Expense", "Accuracy"),
    ]
}

fn mapping(account_code: &str, fs_line: &str, assertion: &str) -> Mapping {
    Mapping {
        account_code: account_code.to_string(),
        fs_line: fs_line.to_string(),
        assertion: assertion.to_string(),
    }
}

fn seed_adjustments() -> Vec<Adjustment> {
    vec![
        Adjustment {
            id: Uuid::new_v4(),
            reference: "AJ-001".to_string(),
            description: "Accrue professional fees incurred before year end".to_string(),
            rationale:
                "Invoice received after 30 June relates to FY2026 audit and secretarial work"
                    .to_string(),
            lines: vec![
                AdjustmentLine {
                    account_code: "6200".to_string(),
                    amount: dec("12000.00"),
                },
                AdjustmentLine {
                    account_code: "2100".to_string(),
                    amount: dec("-12000.00"),
                },
            ],
            created_by: "Aina Rahman".to_string(),
            created_at: Utc::now(),
        },
        Adjustment {
            id: Uuid::new_v4(),
            reference: "AJ-002".to_string(),
            description: "Post depreciation true-up for new production equipment".to_string(),
            rationale: "Asset register depreciation schedule exceeded ledger charge by RM18,000"
                .to_string(),
            lines: vec![
                AdjustmentLine {
                    account_code: "6300".to_string(),
                    amount: dec("18000.00"),
                },
                AdjustmentLine {
                    account_code: "1600".to_string(),
                    amount: dec("-18000.00"),
                },
            ],
            created_by: "Aina Rahman".to_string(),
            created_at: Utc::now(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import_request() -> WorkspaceImportRequest {
        WorkspaceImportRequest {
            entity_name: "Real Components Sdn Bhd".to_string(),
            registration_number: "202401010101 (1567890-X)".to_string(),
            jurisdiction: "Malaysia".to_string(),
            entity_type: "Sdn Bhd".to_string(),
            owner_name: "Hazli Johar".to_string(),
            firm_name: "Amjad & Hazli Advisory".to_string(),
            preparer_name: "Aina Rahman".to_string(),
            reviewer_name: "Amjad Salleh".to_string(),
            client_signer_name: "Hazli Johar".to_string(),
            branch_label: "FY2026 Year-End".to_string(),
            period_start: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
            period_end: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            source_label: "Real TB export 2026-06-30".to_string(),
            trial_balance: vec![
                WorkspaceImportTrialBalanceLine {
                    account_code: "1000".to_string(),
                    account_name: "Cash at Bank".to_string(),
                    account_type: AccountType::Asset,
                    amount: dec("1000.00"),
                    fs_line: "Cash and Bank".to_string(),
                    assertion: "Existence".to_string(),
                },
                WorkspaceImportTrialBalanceLine {
                    account_code: "4000".to_string(),
                    account_name: "Revenue".to_string(),
                    account_type: AccountType::Income,
                    amount: dec("-1000.00"),
                    fs_line: "Revenue".to_string(),
                    assertion: "Completeness".to_string(),
                },
            ],
        }
    }

    #[test]
    fn imports_real_trial_balance_to_prevent_demo_workspace_confusion() {
        let mut store = AppStore::empty();

        let workspace = store.import_workspace(import_request()).unwrap();

        assert_eq!(workspace.repo.name, "Real Components Sdn Bhd");
        assert_eq!(workspace.commits.len(), 2);
        assert_eq!(workspace.branch.head_commit_id, workspace.commits[1].id);
        assert_eq!(workspace.review_pack.commit_id, workspace.commits[1].id);
        assert_eq!(workspace.commits[1].snapshot.trial_balance.len(), 2);
        assert_eq!(workspace.fs_impact_diff.changed_fs_lines.len(), 2);
        assert_eq!(store.list_repos().unwrap().len(), 1);
    }

    #[test]
    fn rejects_unbalanced_real_trial_balance_to_prevent_invalid_review_pack() {
        let mut request = import_request();
        request.trial_balance[0].amount = dec("999.99");
        let mut store = AppStore::empty();

        let result = store.import_workspace(request);

        assert!(matches!(result, Err(StoreError::InvalidImport(_))));
        assert!(store.list_repos().unwrap().is_empty());
    }

    #[test]
    fn maintains_append_only_history_when_correction_commit_is_added() {
        let mut store = AppStore::seeded();
        let repo_id = *store.repos.keys().next().unwrap();
        let branch_id = *store.branch_by_repo.get(&repo_id).unwrap();
        let before_workspace = store.repo_workspace(repo_id).unwrap();
        let before_commit_ids = before_workspace
            .commits
            .iter()
            .map(|commit| commit.id)
            .collect::<Vec<_>>();

        let correction = CorrectionCommitRequest {
            actor_name: "Aina Rahman".to_string(),
            message: "Append correction for bank charge reclass".to_string(),
            reference: "AJ-003".to_string(),
            description: "Reclass bank charges to administrative expenses".to_string(),
            rationale: "Reviewer requested presentation under admin expenses".to_string(),
            lines: vec![
                AdjustmentLine {
                    account_code: "6000".to_string(),
                    amount: dec("3900.00"),
                },
                AdjustmentLine {
                    account_code: "6400".to_string(),
                    amount: dec("-3900.00"),
                },
            ],
        };

        let new_commit = store
            .commit_correction(repo_id, branch_id, correction)
            .unwrap();
        let after_workspace = store.repo_workspace(repo_id).unwrap();

        assert_eq!(
            after_workspace.commits.len(),
            before_workspace.commits.len() + 1
        );
        assert_eq!(after_workspace.branch.head_commit_id, new_commit.id);
        assert_eq!(
            after_workspace
                .commits
                .iter()
                .take(before_commit_ids.len())
                .map(|commit| commit.id)
                .collect::<Vec<_>>(),
            before_commit_ids
        );
    }

    #[test]
    fn rejects_client_signoff_before_reviewer_approval_in_store_workflow() {
        let mut store = AppStore::seeded();
        let review_pack_id = *store.review_packs.keys().next().unwrap();

        let result = store.sign_client(
            review_pack_id,
            ApprovalRequest {
                actor_name: "Hazli Johar".to_string(),
                note: Some("Approved".to_string()),
            },
        );

        assert!(matches!(
            result,
            Err(StoreError::Domain(DomainError::ReviewerApprovalRequired))
        ));
    }

    #[test]
    fn reopens_review_pack_when_correction_changes_reviewed_snapshot() {
        let mut store = AppStore::seeded();
        let repo_id = *store.repos.keys().next().unwrap();
        let branch_id = *store.branch_by_repo.get(&repo_id).unwrap();
        let review_pack_id = *store.review_packs.keys().next().unwrap();

        store
            .approve_reviewer(
                review_pack_id,
                ApprovalRequest {
                    actor_name: "Amjad Salleh".to_string(),
                    note: Some("Reviewed".to_string()),
                },
            )
            .unwrap();

        let new_commit = store
            .commit_correction(
                repo_id,
                branch_id,
                CorrectionCommitRequest {
                    actor_name: "Aina Rahman".to_string(),
                    message: "Append correction after reviewer note".to_string(),
                    reference: "AJ-003".to_string(),
                    description: "Reclass bank charges to administrative expenses".to_string(),
                    rationale: "Reviewer requested presentation under admin expenses".to_string(),
                    lines: vec![
                        AdjustmentLine {
                            account_code: "6000".to_string(),
                            amount: dec("3900.00"),
                        },
                        AdjustmentLine {
                            account_code: "6400".to_string(),
                            amount: dec("-3900.00"),
                        },
                    ],
                },
            )
            .unwrap();
        let workspace = store.repo_workspace(repo_id).unwrap();

        assert_eq!(workspace.review_pack.status, ReviewStatus::InReview);
        assert_eq!(workspace.review_pack.commit_id, new_commit.id);
        assert!(workspace.review_pack.approvals.is_empty());
    }

    #[test]
    fn rejects_correction_commit_after_client_signoff_to_keep_signed_branch_immutable() {
        let mut store = AppStore::seeded();
        let repo_id = *store.repos.keys().next().unwrap();
        let branch_id = *store.branch_by_repo.get(&repo_id).unwrap();
        let review_pack_id = *store.review_packs.keys().next().unwrap();

        store
            .approve_reviewer(
                review_pack_id,
                ApprovalRequest {
                    actor_name: "Amjad Salleh".to_string(),
                    note: Some("Reviewed".to_string()),
                },
            )
            .unwrap();
        store
            .sign_client(
                review_pack_id,
                ApprovalRequest {
                    actor_name: "Hazli Johar".to_string(),
                    note: Some("Signed".to_string()),
                },
            )
            .unwrap();
        let before = store.repo_workspace(repo_id).unwrap();

        let result = store.commit_correction(
            repo_id,
            branch_id,
            CorrectionCommitRequest {
                actor_name: "Aina Rahman".to_string(),
                message: "Attempt correction after sign-off".to_string(),
                reference: "AJ-003".to_string(),
                description: "Reclass bank charges to administrative expenses".to_string(),
                rationale: "Reviewer requested presentation under admin expenses".to_string(),
                lines: vec![
                    AdjustmentLine {
                        account_code: "6000".to_string(),
                        amount: dec("3900.00"),
                    },
                    AdjustmentLine {
                        account_code: "6400".to_string(),
                        amount: dec("-3900.00"),
                    },
                ],
            },
        );
        let after = store.repo_workspace(repo_id).unwrap();

        assert!(matches!(
            result,
            Err(StoreError::Domain(DomainError::FrozenBranch))
        ));
        assert_eq!(after.commits.len(), before.commits.len());
        assert_eq!(after.branch.status, BranchStatus::Frozen);
    }
}
