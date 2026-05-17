use crate::{
    domain::{ApprovalRole, AuditEventType, BranchStatus, QueryStatus, RepoRole, ReviewStatus},
    store::AppStore,
};
use serde_json::Value;
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
            .connect(&database_url)
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
        self.sync_normalized_append_only(store).await?;
        Ok(())
    }
}

impl SnapshotStateStore {
    async fn ensure_schema(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_state_snapshots (
              key TEXT PRIMARY KEY,
              payload JSONB NOT NULL,
              updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )
            "#,
        )
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

    async fn sync_normalized_append_only(&self, store: &AppStore) -> anyhow::Result<()> {
        if !self.normalized_schema_exists().await? {
            return Ok(());
        }

        let mut db_user_ids = BTreeMap::new();

        for organization in store.organizations.values() {
            sqlx::query(
                r#"
                INSERT INTO organizations (id, name)
                VALUES ($1::uuid, $2)
                ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
                "#,
            )
            .bind(organization.id.to_string())
            .bind(&organization.name)
            .execute(&self.pool)
            .await?;
        }

        for user in store.users.values() {
            let db_user_id: String = sqlx::query_scalar(
                r#"
                INSERT INTO users (id, auth_user_id, display_name, email)
                VALUES ($1::uuid, $2, $3, $4)
                ON CONFLICT (email) DO UPDATE SET
                  auth_user_id = COALESCE(users.auth_user_id, EXCLUDED.auth_user_id),
                  display_name = EXCLUDED.display_name
                RETURNING id::text
                "#,
            )
            .bind(user.id.to_string())
            .bind(&user.auth_user_id)
            .bind(&user.display_name)
            .bind(&user.email)
            .fetch_one(&self.pool)
            .await?;
            db_user_ids.insert(user.id, Uuid::parse_str(&db_user_id)?);
        }

        for repo in store.repos.values() {
            sqlx::query(
                r#"
                INSERT INTO legal_entities (id, owner_organization_id, name, registration_number, jurisdiction, entity_type)
                VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6)
                ON CONFLICT (id) DO UPDATE SET
                  name = EXCLUDED.name,
                  registration_number = EXCLUDED.registration_number,
                  jurisdiction = EXCLUDED.jurisdiction,
                  entity_type = EXCLUDED.entity_type
                "#,
            )
            .bind(repo.id.to_string())
            .bind(repo.owner_organization_id.to_string())
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
                    VALUES ($1::uuid, $2::uuid, $3)
                    ON CONFLICT (legal_entity_id, user_id) DO UPDATE SET role = EXCLUDED.role
                    "#,
                )
                .bind(repo.id.to_string())
                .bind(user_id.to_string())
                .bind(repo_role(&collaborator.role))
                .execute(&self.pool)
                .await?;
            }
        }

        for branch in store.branches.values() {
            sqlx::query(
                r#"
                INSERT INTO period_branches (id, legal_entity_id, label, period_start, period_end, status, head_commit_id)
                VALUES ($1::uuid, $2::uuid, $3, $4::date, $5::date, $6, NULL)
                ON CONFLICT (id) DO UPDATE SET
                  label = EXCLUDED.label,
                  period_start = EXCLUDED.period_start,
                  period_end = EXCLUDED.period_end,
                  status = EXCLUDED.status
                "#,
            )
            .bind(branch.id.to_string())
            .bind(branch.legal_entity_id.to_string())
            .bind(&branch.label)
            .bind(branch.period_start.to_string())
            .bind(branch.period_end.to_string())
            .bind(branch_status(&branch.status))
            .execute(&self.pool)
            .await?;
        }

        for commits in store.commits_by_branch.values() {
            for commit in commits {
                sqlx::query(
                    r#"
                    INSERT INTO commits (id, period_branch_id, sequence_number, message, previous_hash, snapshot_hash, snapshot_json, created_by, created_at)
                    VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7::jsonb, $8, $9::timestamptz)
                    ON CONFLICT (id) DO NOTHING
                    "#,
                )
                .bind(commit.id.to_string())
                .bind(commit.branch_id.to_string())
                .bind(commit.sequence_number as i32)
                .bind(&commit.message)
                .bind(&commit.previous_hash)
                .bind(&commit.snapshot_hash)
                .bind(serde_json::to_value(&commit.snapshot)?)
                .bind(&commit.created_by)
                .bind(commit.created_at.to_rfc3339())
                .execute(&self.pool)
                .await?;
            }
        }

        for branch in store.branches.values() {
            sqlx::query(
                r#"
                UPDATE period_branches
                SET head_commit_id = $1::uuid, status = $2
                WHERE id = $3::uuid
                "#,
            )
            .bind(branch.head_commit_id.to_string())
            .bind(branch_status(&branch.status))
            .bind(branch.id.to_string())
            .execute(&self.pool)
            .await?;
        }

        for review_pack in store.review_packs.values() {
            sqlx::query(
                r#"
                INSERT INTO review_packs (id, legal_entity_id, period_branch_id, commit_id, title, status, created_by, created_at)
                VALUES ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5, $6, $7, $8::timestamptz)
                ON CONFLICT (id) DO UPDATE SET
                  commit_id = EXCLUDED.commit_id,
                  title = EXCLUDED.title,
                  status = EXCLUDED.status
                "#,
            )
            .bind(review_pack.id.to_string())
            .bind(review_pack.legal_entity_id.to_string())
            .bind(review_pack.period_branch_id.to_string())
            .bind(review_pack.commit_id.to_string())
            .bind(&review_pack.title)
            .bind(review_status(&review_pack.status))
            .bind(&review_pack.created_by)
            .bind(review_pack.created_at.to_rfc3339())
            .execute(&self.pool)
            .await?;

            for approval in &review_pack.approvals {
                sqlx::query(
                    r#"
                    INSERT INTO approvals (id, review_pack_id, role, actor_user_id, actor_name, actor_email, note, approved_at)
                    VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8::timestamptz)
                    ON CONFLICT (review_pack_id, role) DO UPDATE SET
                      actor_user_id = EXCLUDED.actor_user_id,
                      actor_name = EXCLUDED.actor_name,
                      actor_email = EXCLUDED.actor_email,
                      note = EXCLUDED.note,
                      approved_at = EXCLUDED.approved_at
                    "#,
                )
                .bind(approval.id.to_string())
                .bind(review_pack.id.to_string())
                .bind(approval_role(&approval.role))
                .bind(Option::<String>::None)
                .bind(&approval.actor_name)
                .bind(Option::<String>::None)
                .bind(&approval.note)
                .bind(approval.approved_at.to_rfc3339())
                .execute(&self.pool)
                .await?;
            }

            for query in &review_pack.open_queries {
                sqlx::query(
                    r#"
                    INSERT INTO review_queries (id, review_pack_id, title, status, assigned_to, resolved_note, resolved_by, resolved_at)
                    VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8::timestamptz)
                    ON CONFLICT (id) DO UPDATE SET
                      title = EXCLUDED.title,
                      status = EXCLUDED.status,
                      assigned_to = EXCLUDED.assigned_to,
                      resolved_note = EXCLUDED.resolved_note,
                      resolved_by = EXCLUDED.resolved_by,
                      resolved_at = EXCLUDED.resolved_at
                    "#,
                )
                .bind(query.id.to_string())
                .bind(review_pack.id.to_string())
                .bind(&query.title)
                .bind(query_status(&query.status))
                .bind(&query.assigned_to)
                .bind(&query.resolved_note)
                .bind(&query.resolved_by)
                .bind(query.resolved_at.map(|at| at.to_rfc3339()))
                .execute(&self.pool)
                .await?;
            }
        }

        for events in store.audit_events_by_repo.values() {
            for event in events {
                sqlx::query(
                    r#"
                    INSERT INTO audit_events (id, legal_entity_id, sequence_number, actor_user_id, actor_name, actor_email, event_type, message, occurred_at, related_commit_id, previous_hash, event_hash)
                    VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9::timestamptz, $10::uuid, $11, $12)
                    ON CONFLICT (legal_entity_id, sequence_number) DO NOTHING
                    "#,
                )
                .bind(event.id.to_string())
                .bind(event.legal_entity_id.to_string())
                .bind(event.sequence_number as i64)
                .bind(&event.actor_user_id)
                .bind(&event.actor_name)
                .bind(&event.actor_email)
                .bind(audit_event_type(&event.event_type))
                .bind(&event.message)
                .bind(event.occurred_at.to_rfc3339())
                .bind(event.related_commit_id.map(|id| id.to_string()))
                .bind(&event.previous_hash)
                .bind(&event.event_hash)
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

fn repo_role(role: &RepoRole) -> &'static str {
    match role {
        RepoRole::Owner => "owner",
        RepoRole::Preparer => "preparer",
        RepoRole::Reviewer => "reviewer",
        RepoRole::ClientSigner => "client_signer",
        RepoRole::Observer => "observer",
    }
}

fn branch_status(status: &BranchStatus) -> &'static str {
    match status {
        BranchStatus::Working => "working",
        BranchStatus::InReview => "in_review",
        BranchStatus::Frozen => "frozen",
    }
}

fn review_status(status: &ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::InReview => "in_review",
        ReviewStatus::ReviewerApproved => "reviewer_approved",
        ReviewStatus::Signed => "signed",
    }
}

fn approval_role(role: &ApprovalRole) -> &'static str {
    match role {
        ApprovalRole::Reviewer => "reviewer",
        ApprovalRole::ClientDirector => "client_director",
    }
}

fn query_status(status: &QueryStatus) -> &'static str {
    match status {
        QueryStatus::Open => "open",
        QueryStatus::Resolved => "resolved",
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
