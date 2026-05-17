use accounts_repo_backend::{
    persistence::PersistentState,
    store::{
        AppStore, AuthenticatedActor, WorkspaceImportRequest, WorkspaceImportTrialBalanceLine,
    },
};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::{Connection, Executor, PgConnection, postgres::PgPoolOptions};
use uuid::Uuid;

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(database_url) => Some(database_url),
        Err(_) => {
            eprintln!("skipping Postgres integration test because DATABASE_URL is not set");
            None
        }
    }
}

async fn create_schema(database_url: &str) -> anyhow::Result<(PgConnection, String)> {
    let schema = format!("test_{}", Uuid::new_v4().simple());
    let mut connection = PgConnection::connect(database_url).await?;
    connection
        .execute(format!(r#"CREATE SCHEMA "{schema}""#).as_str())
        .await?;
    connection
        .execute(format!(r#"SET search_path TO "{schema}""#).as_str())
        .await?;
    Ok((connection, schema))
}

async fn drop_schema(mut connection: PgConnection, schema: &str) -> anyhow::Result<()> {
    connection
        .execute(format!(r#"DROP SCHEMA IF EXISTS "{schema}" CASCADE"#).as_str())
        .await?;
    Ok(())
}

async fn persistence_in_schema(
    database_url: &str,
    schema: &str,
) -> anyhow::Result<PersistentState> {
    let search_path = format!(r#"SET search_path TO "{schema}""#);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                connection.execute(search_path.as_str()).await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;

    PersistentState::from_pool(pool).await
}

fn import_request() -> WorkspaceImportRequest {
    WorkspaceImportRequest {
        entity_name: "Persistence Components Sdn Bhd".to_string(),
        registration_number: "202401010101 (1567890-X)".to_string(),
        jurisdiction: "Malaysia".to_string(),
        entity_type: "Sdn Bhd".to_string(),
        owner_name: "Hazli Johar".to_string(),
        owner_email: "hazli@client.test".to_string(),
        firm_name: "Amjad & Hazli Advisory".to_string(),
        preparer_name: "Aina Rahman".to_string(),
        preparer_email: "aina@ahadvisory.test".to_string(),
        reviewer_name: "Amjad Salleh".to_string(),
        reviewer_email: "amjad@ahadvisory.test".to_string(),
        client_signer_name: "Nur Sofia".to_string(),
        client_signer_email: "sofia@client.test".to_string(),
        branch_label: "FY2026 Year-End".to_string(),
        period_start: chrono::NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
        period_end: chrono::NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        source_label: "Real TB export 2026-06-30".to_string(),
        source_file_name: Some("real-tb.csv".to_string()),
        source_file_hash: Some("test-source-hash".to_string()),
        source_parser: Some("csv".to_string()),
        source_row_count: Some(2),
        trial_balance: vec![
            WorkspaceImportTrialBalanceLine {
                account_code: "1000".to_string(),
                account_name: "Cash at Bank".to_string(),
                account_type: accounts_repo_backend::domain::AccountType::Asset,
                amount: Decimal::new(100000, 2),
                fs_line: "Cash and Bank".to_string(),
                assertion: "Existence".to_string(),
            },
            WorkspaceImportTrialBalanceLine {
                account_code: "4000".to_string(),
                account_name: "Revenue".to_string(),
                account_type: accounts_repo_backend::domain::AccountType::Income,
                amount: Decimal::new(-100000, 2),
                fs_line: "Revenue".to_string(),
                assertion: "Completeness".to_string(),
            },
        ],
    }
}

fn actor() -> AuthenticatedActor {
    AuthenticatedActor {
        auth_user_id: "seed-preparer".to_string(),
        display_name: "Aina Rahman".to_string(),
        email: "aina@ahadvisory.test".to_string(),
    }
}

#[tokio::test]
async fn applies_initial_migration_to_empty_postgres_to_prevent_deploy_schema_breaks()
-> anyhow::Result<()> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let (mut connection, schema) = create_schema(&database_url).await?;

    sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
        .execute(&mut connection)
        .await?;
    let table_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables WHERE table_schema = $1 AND table_name IN ('review_packs', 'audit_events', 'signed_pack_exports')",
    )
    .bind(&schema)
    .fetch_one(&mut connection)
    .await?;

    assert_eq!(table_count, 3);
    drop_schema(connection, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn stores_signed_review_pack_in_normalized_tables_to_prevent_schema_contract_drift()
-> anyhow::Result<()> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let (mut connection, schema) = create_schema(&database_url).await?;

    sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
        .execute(&mut connection)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/0002_evidence_foundation.sql"))
        .execute(&mut connection)
        .await?;

    let organization_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let legal_entity_id = Uuid::new_v4();
    let branch_id = Uuid::new_v4();
    let account_id = Uuid::new_v4();
    let trial_balance_line_id = Uuid::new_v4();
    let mapping_id = Uuid::new_v4();
    let commit_id = Uuid::new_v4();
    let review_pack_id = Uuid::new_v4();
    let approval_id = Uuid::new_v4();
    let query_id = Uuid::new_v4();
    let audit_event_id = Uuid::new_v4();
    let export_id = Uuid::new_v4();

    sqlx::query(r#"INSERT INTO organizations (id, name) VALUES ($1::uuid, $2)"#)
        .bind(organization_id.to_string())
        .bind("Nusantara Precision Sdn Bhd")
        .execute(&mut connection)
        .await?;
    sqlx::query(
        r#"INSERT INTO users (id, auth_user_id, display_name, email) VALUES ($1::uuid, $2, $3, $4)"#,
    )
    .bind(user_id.to_string())
    .bind("seed-reviewer")
    .bind("Amjad Salleh")
    .bind("amjad@ahadvisory.test")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO legal_entities (id, owner_organization_id, name, registration_number, jurisdiction, entity_type)
        VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6)
        "#,
    )
    .bind(legal_entity_id.to_string())
    .bind(organization_id.to_string())
    .bind("Nusantara Precision Sdn Bhd")
    .bind("202001034561 (1390882-X)")
    .bind("Malaysia")
    .bind("Sdn Bhd")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"INSERT INTO repo_collaborators (legal_entity_id, user_id, role) VALUES ($1::uuid, $2::uuid, 'reviewer')"#,
    )
    .bind(legal_entity_id.to_string())
    .bind(user_id.to_string())
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO period_branches (id, legal_entity_id, label, period_start, period_end, status)
        VALUES ($1::uuid, $2::uuid, $3, $4::date, $5::date, 'frozen')
        "#,
    )
    .bind(branch_id.to_string())
    .bind(legal_entity_id.to_string())
    .bind("FY2026 Year-End")
    .bind("2025-07-01")
    .bind("2026-06-30")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO accounts (id, legal_entity_id, code, name, account_type)
        VALUES ($1::uuid, $2::uuid, '4000', 'Revenue', 'income')
        "#,
    )
    .bind(account_id.to_string())
    .bind(legal_entity_id.to_string())
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO trial_balance_lines (id, period_branch_id, account_id, amount, source_label)
        VALUES ($1::uuid, $2::uuid, $3::uuid, $4::numeric, $5)
        "#,
    )
    .bind(trial_balance_line_id.to_string())
    .bind(branch_id.to_string())
    .bind(account_id.to_string())
    .bind("-1350000.00")
    .bind("Real TB export 2026-06-30")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO mappings (id, legal_entity_id, account_code, fs_line, assertion)
        VALUES ($1::uuid, $2::uuid, '4000', 'Revenue', 'Completeness')
        "#,
    )
    .bind(mapping_id.to_string())
    .bind(legal_entity_id.to_string())
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO commits (id, period_branch_id, sequence_number, message, snapshot_hash, snapshot_json, created_by)
        VALUES ($1::uuid, $2::uuid, 1, $3, $4, $5::jsonb, $6)
        "#,
    )
    .bind(commit_id.to_string())
    .bind(branch_id.to_string())
    .bind("Imported trial balance")
    .bind("a".repeat(64))
    .bind(json!({"fs_lines": [{"fs_line": "Revenue", "amount": "-1350000.00"}]}))
    .bind("Aina Rahman")
    .execute(&mut connection)
    .await?;
    sqlx::query(r#"UPDATE period_branches SET head_commit_id = $1::uuid WHERE id = $2::uuid"#)
        .bind(commit_id.to_string())
        .bind(branch_id.to_string())
        .execute(&mut connection)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO review_packs (id, legal_entity_id, period_branch_id, commit_id, title, status, created_by)
        VALUES ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5, 'signed', $6)
        "#,
    )
    .bind(review_pack_id.to_string())
    .bind(legal_entity_id.to_string())
    .bind(branch_id.to_string())
    .bind(commit_id.to_string())
    .bind("FY2026 Sdn Bhd Year-End Review Pack")
    .bind("Aina Rahman")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO approvals (id, review_pack_id, commit_id, role, actor_user_id, actor_name, actor_email, snapshot_hash, approval_hash, note)
        VALUES ($1::uuid, $2::uuid, $3::uuid, 'reviewer', $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(approval_id.to_string())
    .bind(review_pack_id.to_string())
    .bind(commit_id.to_string())
    .bind("seed-reviewer")
    .bind("Amjad Salleh")
    .bind("amjad@ahadvisory.test")
    .bind("a".repeat(64))
    .bind("d".repeat(64))
    .bind("Reviewed")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO review_queries (id, review_pack_id, title, status, assigned_to, resolved_note, resolved_by, resolved_at)
        VALUES ($1::uuid, $2::uuid, $3, 'resolved', $4, $5, $6, now())
        "#,
    )
    .bind(query_id.to_string())
    .bind(review_pack_id.to_string())
    .bind("Confirm revenue cut-off support")
    .bind("Aina Rahman")
    .bind("Schedule reviewed")
    .bind("Amjad Salleh")
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO audit_events (id, legal_entity_id, sequence_number, actor_user_id, actor_name, actor_email, event_type, message, related_commit_id, event_hash)
        VALUES ($1::uuid, $2::uuid, 1, $3, $4, $5, 'client_signed', $6, $7::uuid, $8)
        "#,
    )
    .bind(audit_event_id.to_string())
    .bind(legal_entity_id.to_string())
    .bind("seed-reviewer")
    .bind("Amjad Salleh")
    .bind("amjad@ahadvisory.test")
    .bind("Client director signed the review pack")
    .bind(commit_id.to_string())
    .bind("b".repeat(64))
    .execute(&mut connection)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO signed_pack_exports (id, review_pack_id, commit_id, payload_json, payload_hash, exported_by, exported_by_user_id, exported_by_email)
        VALUES ($1::uuid, $2::uuid, $3::uuid, $4::jsonb, $5, $6, $7, $8)
        "#,
    )
    .bind(export_id.to_string())
    .bind(review_pack_id.to_string())
    .bind(commit_id.to_string())
    .bind(json!({"review_pack_id": review_pack_id.to_string(), "commit_id": commit_id.to_string()}))
    .bind("c".repeat(64))
    .bind("Amjad Salleh")
    .bind("seed-reviewer")
    .bind("amjad@ahadvisory.test")
    .execute(&mut connection)
    .await?;

    let signed_pack_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM signed_pack_exports export
        JOIN review_packs pack ON pack.id = export.review_pack_id
        JOIN commits commit ON commit.id = export.commit_id
        JOIN period_branches branch ON branch.id = pack.period_branch_id
        WHERE pack.status = 'signed' AND branch.status = 'frozen' AND commit.id = $1::uuid
        "#,
    )
    .bind(commit_id.to_string())
    .fetch_one(&mut connection)
    .await?;
    assert_eq!(signed_pack_count, 1);

    let rewrite_commit =
        sqlx::query(r#"UPDATE commits SET message = 'Rewritten history' WHERE id = $1::uuid"#)
            .bind(commit_id.to_string())
            .execute(&mut connection)
            .await;
    assert!(rewrite_commit.is_err());

    let duplicate_role = sqlx::query(
        r#"
        INSERT INTO approvals (id, review_pack_id, commit_id, role, actor_user_id, actor_name, actor_email, snapshot_hash, approval_hash)
        VALUES ($1::uuid, $2::uuid, $3::uuid, 'reviewer', 'seed-reviewer', 'Duplicate Reviewer', 'amjad@ahadvisory.test', $4, $5)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(review_pack_id.to_string())
    .bind(commit_id.to_string())
    .bind("a".repeat(64))
    .bind("e".repeat(64))
    .execute(&mut connection)
    .await;
    assert!(duplicate_role.is_err());

    let invalid_status = sqlx::query(
        r#"
        INSERT INTO review_packs (id, legal_entity_id, period_branch_id, commit_id, title, status, created_by)
        VALUES ($1::uuid, $2::uuid, $3::uuid, $4::uuid, 'Invalid Pack', 'approved', 'Aina Rahman')
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(legal_entity_id.to_string())
    .bind(branch_id.to_string())
    .bind(commit_id.to_string())
    .execute(&mut connection)
    .await;
    assert!(invalid_status.is_err());

    drop_schema(connection, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn persists_imported_workspace_across_fresh_state_load_to_prevent_restart_data_loss()
-> anyhow::Result<()> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let (connection, schema) = create_schema(&database_url).await?;
    let persistence = persistence_in_schema(&database_url, &schema).await?;
    let mut store = AppStore::empty();
    let imported_workspace = store.import_workspace(import_request(), &actor())?;

    persistence.save_store(&store).await?;
    let reloaded_store = persistence.load_store().await?;
    let reloaded_workspace = reloaded_store.repo_workspace(imported_workspace.repo.id)?;

    assert_eq!(
        reloaded_workspace.repo.name,
        "Persistence Components Sdn Bhd"
    );
    assert_eq!(reloaded_workspace.commits.len(), 2);
    assert_eq!(reloaded_workspace.import_sources.len(), 1);
    assert_eq!(
        reloaded_workspace.import_sources[0].file_hash,
        "test-source-hash"
    );
    assert_eq!(
        reloaded_workspace.review_pack.id,
        imported_workspace.review_pack.id
    );
    assert_eq!(
        reloaded_workspace.audit_events.len(),
        imported_workspace.audit_events.len()
    );
    assert_eq!(
        reloaded_workspace.repo.summary.revenue,
        Decimal::new(-100000, 2)
    );

    drop(persistence);
    drop_schema(connection, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn mirrors_commits_and_audit_events_into_normalized_tables_when_schema_exists()
-> anyhow::Result<()> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let (mut connection, schema) = create_schema(&database_url).await?;

    sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
        .execute(&mut connection)
        .await?;

    let persistence = persistence_in_schema(&database_url, &schema).await?;
    let mut store = AppStore::empty();
    let imported_workspace = store.import_workspace(import_request(), &actor())?;

    persistence.save_store(&store).await?;

    let commit_count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM commits WHERE period_branch_id = $1::uuid"#)
            .bind(imported_workspace.branch.id.to_string())
            .fetch_one(&mut connection)
            .await?;
    let audit_count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM audit_events WHERE legal_entity_id = $1::uuid"#)
            .bind(imported_workspace.repo.id.to_string())
            .fetch_one(&mut connection)
            .await?;
    let import_source_count: i64 = sqlx::query_scalar(
        r#"SELECT count(*) FROM import_sources WHERE period_branch_id = $1::uuid"#,
    )
    .bind(imported_workspace.branch.id.to_string())
    .fetch_one(&mut connection)
    .await?;
    let account_count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM accounts WHERE legal_entity_id = $1::uuid"#)
            .bind(imported_workspace.repo.id.to_string())
            .fetch_one(&mut connection)
            .await?;
    let tb_line_count: i64 = sqlx::query_scalar(
        r#"SELECT count(*) FROM trial_balance_lines WHERE period_branch_id = $1::uuid"#,
    )
    .bind(imported_workspace.branch.id.to_string())
    .fetch_one(&mut connection)
    .await?;
    let mapping_count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM mappings WHERE legal_entity_id = $1::uuid"#)
            .bind(imported_workspace.repo.id.to_string())
            .fetch_one(&mut connection)
            .await?;
    let head_commit_id: Option<String> = sqlx::query_scalar(
        r#"SELECT head_commit_id::text FROM period_branches WHERE id = $1::uuid"#,
    )
    .bind(imported_workspace.branch.id.to_string())
    .fetch_one(&mut connection)
    .await?;

    assert_eq!(commit_count, imported_workspace.commits.len() as i64);
    assert_eq!(audit_count, imported_workspace.audit_events.len() as i64);
    assert_eq!(import_source_count, 1);
    assert_eq!(account_count, 2);
    assert_eq!(tb_line_count, 2);
    assert_eq!(mapping_count, 2);
    assert_eq!(
        head_commit_id,
        Some(imported_workspace.branch.head_commit_id.to_string())
    );

    drop(persistence);
    drop_schema(connection, &schema).await?;
    Ok(())
}
