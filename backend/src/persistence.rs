use crate::{
    domain::{
        AccountType, Approval, ApprovalRole, AuditEvent, AuditEventType, BranchStatus,
        Collaborator, Commit, ImportSource, LegalEntityRepo, Organization, PeriodBranch,
        QueryStatus, RepoRole, RepoSummary, ReviewPack, ReviewQuery, ReviewStatus,
        TrialBalanceLine, User, repo_summary,
    },
    store::{AppStore, SignedPackExportRecord},
};
use anyhow::{Context, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct PersistentState {
    adapter: SnapshotStateStore,
}

#[derive(Clone)]
pub struct SnapshotStateStore {
    pool: PgPool,
}

pub(crate) trait StateStore {
    async fn load_store(&self) -> anyhow::Result<AppStore>;
    async fn save_store(&self, store: &AppStore) -> anyhow::Result<()>;
}

impl PersistentState {
    pub async fn from_env() -> anyhow::Result<Option<Self>> {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return Ok(None);
        };

        Ok(Some(Self::connect(&database_url).await?))
    }

    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Self::from_pool(pool).await
    }

    pub async fn from_pool(pool: PgPool) -> anyhow::Result<Self> {
        let adapter = SnapshotStateStore { pool };
        adapter.ensure_schema().await?;
        Ok(Self { adapter })
    }

    pub async fn load_store(&self) -> anyhow::Result<AppStore> {
        self.adapter.load_store().await
    }

    pub async fn save_store(&self, store: &AppStore) -> anyhow::Result<()> {
        self.adapter.save_store(store).await
    }
}

impl StateStore for SnapshotStateStore {
    async fn load_store(&self) -> anyhow::Result<AppStore> {
        if let Some(store) = self.load_normalized_store().await? {
            return Ok(store);
        }

        let row = sqlx::query_scalar::<_, Value>(
            "SELECT payload FROM app_state_snapshots WHERE key = 'default'",
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(payload) => Ok(serde_json::from_value(payload)?),
            None => Ok(AppStore::empty()),
        }
    }

    async fn save_store(&self, store: &AppStore) -> anyhow::Result<()> {
        self.save_snapshot(store).await?;
        self.sync_normalized(store).await?;
        Ok(())
    }
}

impl SnapshotStateStore {
    async fn ensure_schema(&self) -> anyhow::Result<()> {
        sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::raw_sql(include_str!("../migrations/0002_evidence_foundation.sql"))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn save_snapshot(&self, store: &AppStore) -> anyhow::Result<()> {
        let payload = serde_json::to_value(store)?;
        sqlx::query(
            r#"
            INSERT INTO app_state_snapshots (key, payload, updated_at)
            VALUES ('default', $1, now())
            ON CONFLICT (key)
            DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()
            "#,
        )
        .bind(payload)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_normalized_store(&self) -> anyhow::Result<Option<AppStore>> {
        if !self.normalized_schema_exists().await? {
            return Ok(None);
        }

        let repo_count: i64 = sqlx::query_scalar("SELECT count(*) FROM legal_entities")
            .fetch_one(&self.pool)
            .await?;
        if repo_count == 0 {
            return Ok(None);
        }

        let organizations = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, name FROM organizations ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id, name)| (id, Organization { id, name }))
        .collect::<BTreeMap<_, _>>();

        let users = sqlx::query_as::<_, (Uuid, Option<String>, String, String)>(
            "SELECT id, auth_user_id, display_name, email FROM users ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id, auth_user_id, display_name, email)| {
            (
                id,
                User {
                    id,
                    auth_user_id,
                    display_name,
                    email,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

        let mut collaborators_by_repo: BTreeMap<Uuid, Vec<Collaborator>> = BTreeMap::new();
        for (legal_entity_id, user_id, display_name, email, role) in sqlx::query_as::<
            _,
            (Uuid, Uuid, String, String, String),
        >(
            r#"
            SELECT collaborator.legal_entity_id, user_account.id, user_account.display_name, user_account.email, collaborator.role
            FROM repo_collaborators collaborator
            JOIN users user_account ON user_account.id = collaborator.user_id
            ORDER BY collaborator.created_at, user_account.display_name
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        {
            collaborators_by_repo
                .entry(legal_entity_id)
                .or_default()
                .push(Collaborator {
                    user_id,
                    display_name,
                    email,
                    role: repo_role_from_str(&role)?,
                });
        }

        let mut branches = BTreeMap::new();
        let mut branch_by_repo = BTreeMap::new();
        for (id, legal_entity_id, label, period_start, period_end, status, head_commit_id) in
            sqlx::query_as::<
                _,
                (
                    Uuid,
                    Uuid,
                    String,
                    NaiveDate,
                    NaiveDate,
                    String,
                    Option<Uuid>,
                ),
            >(
                r#"
                SELECT id, legal_entity_id, label, period_start, period_end, status, head_commit_id
                FROM period_branches
                ORDER BY created_at, id
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        {
            let branch = PeriodBranch {
                id,
                legal_entity_id,
                label,
                period_start,
                period_end,
                status: branch_status_from_str(&status)?,
                head_commit_id: head_commit_id.unwrap_or_else(Uuid::nil),
            };
            branch_by_repo.insert(legal_entity_id, id);
            branches.insert(id, branch);
        }

        let mut commits_by_branch: BTreeMap<Uuid, Vec<Commit>> = BTreeMap::new();
        for (
            id,
            period_branch_id,
            sequence_number,
            message,
            previous_hash,
            snapshot_hash,
            snapshot_json,
            created_by,
            created_at,
        ) in sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                i32,
                String,
                Option<String>,
                String,
                Value,
                String,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, period_branch_id, sequence_number, message, previous_hash, snapshot_hash, snapshot_json, created_by, created_at
            FROM commits
            ORDER BY period_branch_id, sequence_number
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        {
            commits_by_branch
                .entry(period_branch_id)
                .or_default()
                .push(Commit {
                    id,
                    branch_id: period_branch_id,
                    sequence_number: sequence_number as u32,
                    message,
                    previous_hash,
                    snapshot_hash,
                    snapshot: serde_json::from_value(snapshot_json)?,
                    created_by,
                    created_at,
                });
        }

        let mut import_sources_by_branch: BTreeMap<Uuid, Vec<ImportSource>> = BTreeMap::new();
        for (
            id,
            legal_entity_id,
            period_branch_id,
            label,
            file_name,
            file_hash,
            parser,
            row_count,
            uploaded_by_user_id,
            uploaded_by_name,
            uploaded_by_email,
            uploaded_at,
        ) in sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                String,
                Option<String>,
                String,
                String,
                i32,
                String,
                String,
                String,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, legal_entity_id, period_branch_id, label, file_name, file_hash, parser, row_count,
                   uploaded_by_user_id, uploaded_by_name, uploaded_by_email, uploaded_at
            FROM import_sources
            ORDER BY uploaded_at, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        {
            import_sources_by_branch
                .entry(period_branch_id)
                .or_default()
                .push(ImportSource {
                    id,
                    legal_entity_id,
                    period_branch_id,
                    label,
                    file_name,
                    file_hash,
                    parser,
                    row_count: row_count as u32,
                    uploaded_by_user_id,
                    uploaded_by_name,
                    uploaded_by_email,
                    uploaded_at,
                });
        }

        let mut review_packs = BTreeMap::new();
        let mut review_pack_by_repo = BTreeMap::new();
        for (id, legal_entity_id, period_branch_id, commit_id, title, status, created_by, created_at) in
            sqlx::query_as::<_, (Uuid, Uuid, Uuid, Uuid, String, String, String, DateTime<Utc>)>(
                r#"
                SELECT id, legal_entity_id, period_branch_id, commit_id, title, status, created_by, created_at
                FROM review_packs
                ORDER BY created_at, id
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        {
            review_pack_by_repo.insert(legal_entity_id, id);
            review_packs.insert(
                id,
                ReviewPack {
                    id,
                    legal_entity_id,
                    period_branch_id,
                    commit_id,
                    title,
                    status: review_status_from_str(&status)?,
                    approvals: vec![],
                    open_queries: vec![],
                    created_by,
                    created_at,
                },
            );
        }

        for (
            id,
            review_pack_id,
            commit_id,
            role,
            actor_user_id,
            actor_name,
            actor_email,
            snapshot_hash,
            approval_hash,
            note,
            approved_at,
        ) in sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, review_pack_id, commit_id, role, actor_user_id, actor_name, actor_email,
                   snapshot_hash, approval_hash, note, approved_at
            FROM approvals
            ORDER BY approved_at, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        {
            if let Some(pack) = review_packs.get_mut(&review_pack_id) {
                if commit_id == pack.commit_id {
                    pack.approvals.push(Approval {
                        id,
                        review_pack_id,
                        commit_id,
                        role: approval_role_from_str(&role)?,
                        actor_user_id,
                        actor_name,
                        actor_email,
                        snapshot_hash,
                        approval_hash,
                        note,
                        approved_at,
                    });
                }
            }
        }

        for (id, review_pack_id, title, status, assigned_to, resolved_note, resolved_by, resolved_at) in
            sqlx::query_as::<
                _,
                (
                    Uuid,
                    Uuid,
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<DateTime<Utc>>,
                ),
            >(
                r#"
                SELECT id, review_pack_id, title, status, assigned_to, resolved_note, resolved_by, resolved_at
                FROM review_queries
                ORDER BY created_at, id
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        {
            if let Some(pack) = review_packs.get_mut(&review_pack_id) {
                pack.open_queries.push(ReviewQuery {
                    id,
                    title,
                    status: query_status_from_str(&status)?,
                    assigned_to,
                    resolved_note,
                    resolved_by,
                    resolved_at,
                });
            }
        }

        let mut repos = BTreeMap::new();
        for (id, owner_organization_id, name, registration_number, jurisdiction, entity_type) in
            sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String)>(
                r#"
                SELECT id, owner_organization_id, name, registration_number, jurisdiction, entity_type
                FROM legal_entities
                ORDER BY created_at, id
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        {
            let summary = branch_by_repo
                .get(&id)
                .and_then(|branch_id| branches.get(branch_id))
                .and_then(|branch| {
                    commits_by_branch
                        .get(&branch.id)
                        .and_then(|commits| commits.iter().find(|commit| commit.id == branch.head_commit_id))
                        .map(|commit| (branch, commit))
                })
                .map(|(branch, commit)| {
                    let review_status = review_pack_by_repo
                        .get(&id)
                        .and_then(|pack_id| review_packs.get(pack_id))
                        .map(|pack| pack.status.clone())
                        .unwrap_or(ReviewStatus::InReview);
                    repo_summary(branch, commit, review_status)
                })
                .unwrap_or_else(|| RepoSummary {
                    active_branch_label: "No active branch".to_string(),
                    head_commit_hash: String::new(),
                    review_pack_status: ReviewStatus::InReview,
                    revenue: Decimal::ZERO,
                    profit_before_tax: Decimal::ZERO,
                    net_assets: Decimal::ZERO,
                });

            repos.insert(
                id,
                LegalEntityRepo {
                    id,
                    owner_organization_id,
                    name,
                    registration_number,
                    jurisdiction,
                    entity_type,
                    collaborators: collaborators_by_repo.remove(&id).unwrap_or_default(),
                    summary,
                },
            );
        }

        let mut audit_events_by_repo: BTreeMap<Uuid, Vec<AuditEvent>> = BTreeMap::new();
        for (
            id,
            legal_entity_id,
            sequence_number,
            actor_user_id,
            actor_name,
            actor_email,
            event_type,
            message,
            occurred_at,
            related_commit_id,
            previous_hash,
            event_hash,
        ) in sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                i64,
                Option<String>,
                String,
                String,
                String,
                String,
                DateTime<Utc>,
                Option<Uuid>,
                Option<String>,
                String,
            ),
        >(
            r#"
            SELECT id, legal_entity_id, sequence_number, actor_user_id, actor_name, actor_email,
                   event_type, message, occurred_at, related_commit_id, previous_hash, event_hash
            FROM audit_events
            ORDER BY legal_entity_id, sequence_number
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        {
            audit_events_by_repo
                .entry(legal_entity_id)
                .or_default()
                .push(AuditEvent {
                    id,
                    legal_entity_id,
                    sequence_number: sequence_number as u64,
                    actor_user_id,
                    actor_name,
                    actor_email,
                    event_type: audit_event_type_from_str(&event_type)?,
                    message,
                    occurred_at,
                    related_commit_id,
                    previous_hash,
                    event_hash,
                });
        }

        let mut signed_exports_by_pack: BTreeMap<Uuid, Vec<SignedPackExportRecord>> =
            BTreeMap::new();
        for (
            id,
            review_pack_id,
            commit_id,
            payload_json,
            payload_hash,
            exported_by,
            exported_by_user_id,
            exported_by_email,
            exported_at,
        ) in sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                Value,
                String,
                String,
                String,
                String,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, review_pack_id, commit_id, payload_json, payload_hash, exported_by,
                   exported_by_user_id, exported_by_email, exported_at
            FROM signed_pack_exports
            ORDER BY exported_at, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        {
            signed_exports_by_pack
                .entry(review_pack_id)
                .or_default()
                .push(SignedPackExportRecord {
                    id,
                    review_pack_id,
                    commit_id,
                    payload_json,
                    payload_hash,
                    exported_by,
                    exported_by_user_id,
                    exported_by_email,
                    exported_at,
                });
        }

        Ok(Some(AppStore {
            users,
            organizations,
            repos,
            branches,
            branch_by_repo,
            commits_by_branch,
            import_sources_by_branch,
            review_packs,
            review_pack_by_repo,
            audit_events_by_repo,
            signed_exports_by_pack,
        }))
    }

    async fn sync_normalized(&self, store: &AppStore) -> anyhow::Result<()> {
        if !self.normalized_schema_exists().await? {
            return Ok(());
        }

        let mut db_user_ids = BTreeMap::new();

        for organization in store.organizations.values() {
            sqlx::query(
                r#"
                INSERT INTO organizations (id, name)
                VALUES ($1, $2)
                ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
                "#,
            )
            .bind(organization.id)
            .bind(&organization.name)
            .execute(&self.pool)
            .await?;
        }

        for user in store.users.values() {
            let db_user_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO users (id, auth_user_id, display_name, email)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (email) DO UPDATE SET
                  auth_user_id = COALESCE(users.auth_user_id, EXCLUDED.auth_user_id),
                  display_name = EXCLUDED.display_name
                RETURNING id
                "#,
            )
            .bind(user.id)
            .bind(&user.auth_user_id)
            .bind(&user.display_name)
            .bind(&user.email)
            .fetch_one(&self.pool)
            .await?;
            db_user_ids.insert(user.id, db_user_id);
        }

        for repo in store.repos.values() {
            sqlx::query(
                r#"
                INSERT INTO legal_entities (id, owner_organization_id, name, registration_number, jurisdiction, entity_type)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (id) DO UPDATE SET
                  name = EXCLUDED.name,
                  registration_number = EXCLUDED.registration_number,
                  jurisdiction = EXCLUDED.jurisdiction,
                  entity_type = EXCLUDED.entity_type
                "#,
            )
            .bind(repo.id)
            .bind(repo.owner_organization_id)
            .bind(&repo.name)
            .bind(&repo.registration_number)
            .bind(&repo.jurisdiction)
            .bind(&repo.entity_type)
            .execute(&self.pool)
            .await?;

            for collaborator in &repo.collaborators {
                let user_id = db_user_ids
                    .get(&collaborator.user_id)
                    .copied()
                    .unwrap_or(collaborator.user_id);
                sqlx::query(
                    r#"
                    INSERT INTO repo_collaborators (legal_entity_id, user_id, role)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (legal_entity_id, user_id) DO UPDATE SET role = EXCLUDED.role
                    "#,
                )
                .bind(repo.id)
                .bind(user_id)
                .bind(repo_role(&collaborator.role))
                .execute(&self.pool)
                .await?;
            }
        }

        for branch in store.branches.values() {
            sqlx::query(
                r#"
                INSERT INTO period_branches (id, legal_entity_id, label, period_start, period_end, status, head_commit_id)
                VALUES ($1, $2, $3, $4, $5, $6, NULL)
                ON CONFLICT (id) DO UPDATE SET
                  label = EXCLUDED.label,
                  period_start = EXCLUDED.period_start,
                  period_end = EXCLUDED.period_end,
                  status = EXCLUDED.status
                "#,
            )
            .bind(branch.id)
            .bind(branch.legal_entity_id)
            .bind(&branch.label)
            .bind(branch.period_start)
            .bind(branch.period_end)
            .bind(branch_status(&branch.status))
            .execute(&self.pool)
            .await?;
        }

        self.sync_import_sources(store).await?;
        self.sync_source_detail(store).await?;
        self.sync_commits(store).await?;

        for branch in store.branches.values() {
            sqlx::query(
                r#"
                UPDATE period_branches
                SET head_commit_id = $1, status = $2
                WHERE id = $3
                "#,
            )
            .bind(branch.head_commit_id)
            .bind(branch_status(&branch.status))
            .bind(branch.id)
            .execute(&self.pool)
            .await?;
        }

        self.sync_review_packs(store).await?;
        self.sync_audit_events(store).await?;
        self.sync_signed_exports(store).await?;

        Ok(())
    }

    async fn sync_import_sources(&self, store: &AppStore) -> anyhow::Result<()> {
        for sources in store.import_sources_by_branch.values() {
            for source in sources {
                sqlx::query(
                    r#"
                    INSERT INTO import_sources (id, legal_entity_id, period_branch_id, label, file_name, file_hash, parser, row_count,
                                                uploaded_by_user_id, uploaded_by_name, uploaded_by_email, uploaded_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT (period_branch_id, file_hash) DO UPDATE SET
                      label = EXCLUDED.label,
                      file_name = EXCLUDED.file_name,
                      parser = EXCLUDED.parser,
                      row_count = EXCLUDED.row_count
                    "#,
                )
                .bind(source.id)
                .bind(source.legal_entity_id)
                .bind(source.period_branch_id)
                .bind(&source.label)
                .bind(&source.file_name)
                .bind(&source.file_hash)
                .bind(&source.parser)
                .bind(source.row_count as i32)
                .bind(&source.uploaded_by_user_id)
                .bind(&source.uploaded_by_name)
                .bind(&source.uploaded_by_email)
                .bind(source.uploaded_at)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn sync_source_detail(&self, store: &AppStore) -> anyhow::Result<()> {
        for (branch_id, commits) in &store.commits_by_branch {
            let branch = store
                .branches
                .get(branch_id)
                .with_context(|| format!("missing branch {branch_id}"))?;
            let Some(snapshot) = commits
                .iter()
                .rev()
                .find(|commit| !commit.snapshot.trial_balance.is_empty())
                .map(|commit| &commit.snapshot)
            else {
                continue;
            };

            for line in &snapshot.trial_balance {
                let account_id = self.upsert_account(branch.legal_entity_id, line).await?;
                let trial_balance_line_id = stable_uuid(&[
                    "trial_balance_line",
                    &branch.id.to_string(),
                    &account_id.to_string(),
                ]);
                sqlx::query(
                    r#"
                    INSERT INTO trial_balance_lines (id, period_branch_id, account_id, amount, source_label, source_id)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (period_branch_id, account_id) DO UPDATE SET
                      amount = EXCLUDED.amount,
                      source_label = EXCLUDED.source_label,
                      source_id = EXCLUDED.source_id
                    "#,
                )
                .bind(trial_balance_line_id)
                .bind(branch.id)
                .bind(account_id)
                .bind(line.amount)
                .bind(&line.source_label)
                .bind(line.source_id)
                .execute(&self.pool)
                .await?;
            }

            for mapping in &snapshot.mappings {
                let mapping_id = stable_uuid(&[
                    "mapping",
                    &branch.legal_entity_id.to_string(),
                    &mapping.account_code,
                ]);
                sqlx::query(
                    r#"
                    INSERT INTO mappings (id, legal_entity_id, account_code, fs_line, assertion)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (legal_entity_id, account_code) DO UPDATE SET
                      fs_line = EXCLUDED.fs_line,
                      assertion = EXCLUDED.assertion
                    "#,
                )
                .bind(mapping_id)
                .bind(branch.legal_entity_id)
                .bind(&mapping.account_code)
                .bind(&mapping.fs_line)
                .bind(&mapping.assertion)
                .execute(&self.pool)
                .await?;
            }

            for adjustment in &snapshot.adjustments {
                let adjustment_id: Uuid = sqlx::query_scalar(
                    r#"
                    INSERT INTO adjustments (id, period_branch_id, reference, description, rationale, created_by, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (period_branch_id, reference) DO UPDATE SET
                      description = EXCLUDED.description,
                      rationale = EXCLUDED.rationale
                    RETURNING id
                    "#,
                )
                .bind(adjustment.id)
                .bind(branch.id)
                .bind(&adjustment.reference)
                .bind(&adjustment.description)
                .bind(&adjustment.rationale)
                .bind(&adjustment.created_by)
                .bind(adjustment.created_at)
                .fetch_one(&self.pool)
                .await?;

                for (index, line) in adjustment.lines.iter().enumerate() {
                    let line_id = stable_uuid(&[
                        "adjustment_line",
                        &adjustment_id.to_string(),
                        &index.to_string(),
                    ]);
                    sqlx::query(
                        r#"
                        INSERT INTO adjustment_lines (id, adjustment_id, account_code, amount)
                        VALUES ($1, $2, $3, $4)
                        ON CONFLICT (id) DO UPDATE SET
                          account_code = EXCLUDED.account_code,
                          amount = EXCLUDED.amount
                        "#,
                    )
                    .bind(line_id)
                    .bind(adjustment_id)
                    .bind(&line.account_code)
                    .bind(line.amount)
                    .execute(&self.pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    async fn upsert_account(
        &self,
        legal_entity_id: Uuid,
        line: &TrialBalanceLine,
    ) -> anyhow::Result<Uuid> {
        let account_id =
            stable_uuid(&["account", &legal_entity_id.to_string(), &line.account_code]);
        Ok(sqlx::query_scalar(
            r#"
            INSERT INTO accounts (id, legal_entity_id, code, name, account_type)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (legal_entity_id, code) DO UPDATE SET
              name = EXCLUDED.name,
              account_type = EXCLUDED.account_type
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(legal_entity_id)
        .bind(&line.account_code)
        .bind(&line.account_name)
        .bind(account_type(&line.account_type))
        .fetch_one(&self.pool)
        .await?)
    }

    async fn sync_commits(&self, store: &AppStore) -> anyhow::Result<()> {
        for commits in store.commits_by_branch.values() {
            for commit in commits {
                sqlx::query(
                    r#"
                    INSERT INTO commits (id, period_branch_id, sequence_number, message, previous_hash, snapshot_hash, snapshot_json, created_by, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (id) DO NOTHING
                    "#,
                )
                .bind(commit.id)
                .bind(commit.branch_id)
                .bind(commit.sequence_number as i32)
                .bind(&commit.message)
                .bind(&commit.previous_hash)
                .bind(&commit.snapshot_hash)
                .bind(serde_json::to_value(&commit.snapshot)?)
                .bind(&commit.created_by)
                .bind(commit.created_at)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn sync_review_packs(&self, store: &AppStore) -> anyhow::Result<()> {
        for review_pack in store.review_packs.values() {
            sqlx::query(
                r#"
                INSERT INTO review_packs (id, legal_entity_id, period_branch_id, commit_id, title, status, created_by, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO UPDATE SET
                  commit_id = EXCLUDED.commit_id,
                  title = EXCLUDED.title,
                  status = EXCLUDED.status
                "#,
            )
            .bind(review_pack.id)
            .bind(review_pack.legal_entity_id)
            .bind(review_pack.period_branch_id)
            .bind(review_pack.commit_id)
            .bind(&review_pack.title)
            .bind(review_status(&review_pack.status))
            .bind(&review_pack.created_by)
            .bind(review_pack.created_at)
            .execute(&self.pool)
            .await?;

            for approval in &review_pack.approvals {
                sqlx::query(
                    r#"
                    INSERT INTO approvals (id, review_pack_id, commit_id, role, actor_user_id, actor_name, actor_email,
                                           snapshot_hash, approval_hash, note, approved_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                    ON CONFLICT (review_pack_id, commit_id, role) DO NOTHING
                    "#,
                )
                .bind(approval.id)
                .bind(approval.review_pack_id)
                .bind(approval.commit_id)
                .bind(approval_role(&approval.role))
                .bind(&approval.actor_user_id)
                .bind(&approval.actor_name)
                .bind(&approval.actor_email)
                .bind(&approval.snapshot_hash)
                .bind(&approval.approval_hash)
                .bind(&approval.note)
                .bind(approval.approved_at)
                .execute(&self.pool)
                .await?;
            }

            for query in &review_pack.open_queries {
                sqlx::query(
                    r#"
                    INSERT INTO review_queries (id, review_pack_id, title, status, assigned_to, resolved_note, resolved_by, resolved_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (id) DO UPDATE SET
                      title = EXCLUDED.title,
                      status = EXCLUDED.status,
                      assigned_to = EXCLUDED.assigned_to,
                      resolved_note = EXCLUDED.resolved_note,
                      resolved_by = EXCLUDED.resolved_by,
                      resolved_at = EXCLUDED.resolved_at
                    "#,
                )
                .bind(query.id)
                .bind(review_pack.id)
                .bind(&query.title)
                .bind(query_status(&query.status))
                .bind(&query.assigned_to)
                .bind(&query.resolved_note)
                .bind(&query.resolved_by)
                .bind(query.resolved_at)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn sync_audit_events(&self, store: &AppStore) -> anyhow::Result<()> {
        for events in store.audit_events_by_repo.values() {
            for event in events {
                sqlx::query(
                    r#"
                    INSERT INTO audit_events (id, legal_entity_id, sequence_number, actor_user_id, actor_name, actor_email, event_type, message,
                                              occurred_at, related_commit_id, previous_hash, event_hash)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT (legal_entity_id, sequence_number) DO NOTHING
                    "#,
                )
                .bind(event.id)
                .bind(event.legal_entity_id)
                .bind(event.sequence_number as i64)
                .bind(&event.actor_user_id)
                .bind(&event.actor_name)
                .bind(&event.actor_email)
                .bind(audit_event_type(&event.event_type))
                .bind(&event.message)
                .bind(event.occurred_at)
                .bind(event.related_commit_id)
                .bind(&event.previous_hash)
                .bind(&event.event_hash)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn sync_signed_exports(&self, store: &AppStore) -> anyhow::Result<()> {
        for records in store.signed_exports_by_pack.values() {
            for record in records {
                sqlx::query(
                    r#"
                    INSERT INTO signed_pack_exports (id, review_pack_id, commit_id, payload_json, payload_hash, exported_by,
                                                     exported_by_user_id, exported_by_email, exported_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (review_pack_id, payload_hash) DO NOTHING
                    "#,
                )
                .bind(record.id)
                .bind(record.review_pack_id)
                .bind(record.commit_id)
                .bind(&record.payload_json)
                .bind(&record.payload_hash)
                .bind(&record.exported_by)
                .bind(&record.exported_by_user_id)
                .bind(&record.exported_by_email)
                .bind(record.exported_at)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn normalized_schema_exists(&self) -> anyhow::Result<bool> {
        Ok(
            sqlx::query_scalar("SELECT to_regclass('legal_entities') IS NOT NULL")
                .fetch_one(&self.pool)
                .await?,
        )
    }
}

fn stable_uuid(parts: &[&str]) -> Uuid {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let hash = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    Uuid::from_bytes(bytes)
}

fn repo_role(role: &RepoRole) -> &'static str {
    match role {
        RepoRole::Owner => "owner",
        RepoRole::Preparer => "preparer",
        RepoRole::Reviewer => "reviewer",
        RepoRole::ClientSigner => "client_signer",
        RepoRole::Observer => "observer",
    }
}

fn repo_role_from_str(value: &str) -> anyhow::Result<RepoRole> {
    match value {
        "owner" => Ok(RepoRole::Owner),
        "preparer" => Ok(RepoRole::Preparer),
        "reviewer" => Ok(RepoRole::Reviewer),
        "client_signer" => Ok(RepoRole::ClientSigner),
        "observer" => Ok(RepoRole::Observer),
        other => Err(anyhow!("unknown repo role {other}")),
    }
}

fn branch_status(status: &BranchStatus) -> &'static str {
    match status {
        BranchStatus::Working => "working",
        BranchStatus::InReview => "in_review",
        BranchStatus::Frozen => "frozen",
    }
}

fn branch_status_from_str(value: &str) -> anyhow::Result<BranchStatus> {
    match value {
        "working" => Ok(BranchStatus::Working),
        "in_review" => Ok(BranchStatus::InReview),
        "frozen" => Ok(BranchStatus::Frozen),
        other => Err(anyhow!("unknown branch status {other}")),
    }
}

fn review_status(status: &ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::InReview => "in_review",
        ReviewStatus::ReviewerApproved => "reviewer_approved",
        ReviewStatus::Signed => "signed",
    }
}

fn review_status_from_str(value: &str) -> anyhow::Result<ReviewStatus> {
    match value {
        "in_review" => Ok(ReviewStatus::InReview),
        "reviewer_approved" => Ok(ReviewStatus::ReviewerApproved),
        "signed" => Ok(ReviewStatus::Signed),
        other => Err(anyhow!("unknown review status {other}")),
    }
}

fn approval_role(role: &ApprovalRole) -> &'static str {
    match role {
        ApprovalRole::Reviewer => "reviewer",
        ApprovalRole::ClientDirector => "client_director",
    }
}

fn approval_role_from_str(value: &str) -> anyhow::Result<ApprovalRole> {
    match value {
        "reviewer" => Ok(ApprovalRole::Reviewer),
        "client_director" => Ok(ApprovalRole::ClientDirector),
        other => Err(anyhow!("unknown approval role {other}")),
    }
}

fn query_status(status: &QueryStatus) -> &'static str {
    match status {
        QueryStatus::Open => "open",
        QueryStatus::Resolved => "resolved",
    }
}

fn query_status_from_str(value: &str) -> anyhow::Result<QueryStatus> {
    match value {
        "open" => Ok(QueryStatus::Open),
        "resolved" => Ok(QueryStatus::Resolved),
        other => Err(anyhow!("unknown query status {other}")),
    }
}

fn account_type(account_type: &AccountType) -> &'static str {
    match account_type {
        AccountType::Asset => "asset",
        AccountType::Liability => "liability",
        AccountType::Equity => "equity",
        AccountType::Income => "income",
        AccountType::Expense => "expense",
    }
}

#[allow(dead_code)]
fn account_type_from_str(value: &str) -> anyhow::Result<AccountType> {
    match value {
        "asset" => Ok(AccountType::Asset),
        "liability" => Ok(AccountType::Liability),
        "equity" => Ok(AccountType::Equity),
        "income" => Ok(AccountType::Income),
        "expense" => Ok(AccountType::Expense),
        other => Err(anyhow!("unknown account type {other}")),
    }
}

fn audit_event_type(event_type: &AuditEventType) -> &'static str {
    match event_type {
        AuditEventType::RepoCreated => "repo_created",
        AuditEventType::BranchCreated => "branch_created",
        AuditEventType::DataImported => "data_imported",
        AuditEventType::CommitCreated => "commit_created",
        AuditEventType::ReviewPackOpened => "review_pack_opened",
        AuditEventType::ReviewerApproved => "reviewer_approved",
        AuditEventType::ClientSigned => "client_signed",
        AuditEventType::CorrectionCommitted => "correction_committed",
        AuditEventType::ReviewQueryOpened => "review_query_opened",
        AuditEventType::ReviewQueryResolved => "review_query_resolved",
        AuditEventType::SignedPackExported => "signed_pack_exported",
    }
}

fn audit_event_type_from_str(value: &str) -> anyhow::Result<AuditEventType> {
    match value {
        "repo_created" => Ok(AuditEventType::RepoCreated),
        "branch_created" => Ok(AuditEventType::BranchCreated),
        "data_imported" => Ok(AuditEventType::DataImported),
        "commit_created" => Ok(AuditEventType::CommitCreated),
        "review_pack_opened" => Ok(AuditEventType::ReviewPackOpened),
        "reviewer_approved" => Ok(AuditEventType::ReviewerApproved),
        "client_signed" => Ok(AuditEventType::ClientSigned),
        "correction_committed" => Ok(AuditEventType::CorrectionCommitted),
        "review_query_opened" => Ok(AuditEventType::ReviewQueryOpened),
        "review_query_resolved" => Ok(AuditEventType::ReviewQueryResolved),
        "signed_pack_exported" => Ok(AuditEventType::SignedPackExported),
        other => Err(anyhow!("unknown audit event type {other}")),
    }
}
