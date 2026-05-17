use accounts_repo_backend::{
    AppState, app,
    auth::AuthConfig,
    domain::{
        AuditEvent, AuditEventType, LegalEntityRepo, QueryStatus, ReviewPack, ReviewQuery,
        ReviewStatus,
    },
    store::{AppStore, RepoWorkspace},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, request::Builder},
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    app(AppState {
        store: Arc::new(RwLock::new(AppStore::seeded())),
        auth: AuthConfig::disabled_dev(),
        persistence: None,
    })
}

fn empty_test_app() -> axum::Router {
    app(AppState {
        store: Arc::new(RwLock::new(AppStore::empty())),
        auth: AuthConfig::disabled_dev(),
        persistence: None,
    })
}

async fn read_json<T: DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn reviewer_headers(builder: Builder) -> Builder {
    builder
        .header("x-dev-user-id", "seed-reviewer")
        .header("x-dev-user-name", "Amjad Salleh")
        .header("x-dev-user-email", "amjad@ahadvisory.test")
}

fn owner_headers(builder: Builder) -> Builder {
    builder
        .header("x-dev-user-id", "seed-owner")
        .header("x-dev-user-name", "Hazli Johar")
        .header("x-dev-user-email", "hazli@nusantara.test")
}

async fn seeded_repo_workspace(app: &axum::Router) -> (LegalEntityRepo, RepoWorkspace) {
    let repos_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let repos: Vec<LegalEntityRepo> = read_json(repos_response).await;
    let repo = repos[0].clone();

    let workspace_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{}", repo.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let workspace = read_json(workspace_response).await;

    (repo, workspace)
}

#[tokio::test]
async fn imports_real_trial_balance_through_http_to_prevent_seed_dependency() {
    let app = empty_test_app();
    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/imports/year-end-review-pack")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "entity_name": "Real Components Sdn Bhd",
                        "registration_number": "202401010101 (1567890-X)",
                        "jurisdiction": "Malaysia",
                        "entity_type": "Sdn Bhd",
                        "owner_name": "Hazli Johar",
                        "owner_email": "hazli@client.test",
                        "firm_name": "Amjad & Hazli Advisory",
                        "preparer_name": "Aina Rahman",
                        "preparer_email": "aina@ahadvisory.test",
                        "reviewer_name": "Amjad Salleh",
                        "reviewer_email": "amjad@ahadvisory.test",
                        "client_signer_name": "Nur Sofia",
                        "client_signer_email": "sofia@client.test",
                        "branch_label": "FY2026 Year-End",
                        "period_start": "2025-07-01",
                        "period_end": "2026-06-30",
                        "source_label": "Real TB export 2026-06-30",
                        "trial_balance": [
                            {
                                "account_code": "1000",
                                "account_name": "Cash at Bank",
                                "account_type": "asset",
                                "amount": "1000.00",
                                "fs_line": "Cash and Bank",
                                "assertion": "Existence"
                            },
                            {
                                "account_code": "4000",
                                "account_name": "Revenue",
                                "account_type": "income",
                                "amount": "-1000.00",
                                "fs_line": "Revenue",
                                "assertion": "Completeness"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(import_response.status(), StatusCode::CREATED);
    let workspace: accounts_repo_backend::store::RepoWorkspace = read_json(import_response).await;
    assert_eq!(workspace.repo.name, "Real Components Sdn Bhd");
    assert_eq!(workspace.commits.len(), 2);

    let repos_response = app
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let repos: Vec<accounts_repo_backend::domain::LegalEntityRepo> =
        read_json(repos_response).await;
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "Real Components Sdn Bhd");
}

#[tokio::test]
async fn exposes_complete_contract_metadata_for_known_api_routes() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/meta/contract")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let contract: serde_json::Value = read_json(response).await;
    let routes = contract["routes"].as_array().unwrap();
    let route_pairs = routes
        .iter()
        .map(|route| {
            (
                route["method"].as_str().unwrap(),
                route["path"].as_str().unwrap(),
            )
        })
        .collect::<Vec<_>>();

    for expected in [
        ("GET", "/api/meta/contract"),
        ("GET", "/api/repos"),
        ("POST", "/api/imports/year-end-review-pack"),
        ("GET", "/api/repos/{repo_id}"),
        ("GET", "/api/repos/{repo_id}/audit"),
        (
            "POST",
            "/api/repos/{repo_id}/branches/{branch_id}/correction-commits",
        ),
        ("GET", "/api/review-packs/{review_pack_id}"),
        (
            "POST",
            "/api/review-packs/{review_pack_id}/reviewer-approval",
        ),
        ("POST", "/api/review-packs/{review_pack_id}/client-signoff"),
        ("POST", "/api/review-packs/{review_pack_id}/queries"),
        (
            "POST",
            "/api/review-packs/{review_pack_id}/queries/{query_id}/resolve",
        ),
        ("POST", "/api/review-packs/{review_pack_id}/signed-export"),
    ] {
        assert!(
            route_pairs.contains(&expected),
            "missing route {expected:?}"
        );
    }

    let dto_interfaces = contract["dto_interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|interface| interface.as_str().unwrap())
        .collect::<Vec<_>>();

    for expected in ["ReviewPack", "AuditEvent", "SignedPackExport"] {
        assert!(
            dto_interfaces.contains(&expected),
            "missing DTO interface {expected}",
        );
    }
}

#[tokio::test]
async fn rejects_client_signoff_without_reviewer_approval_through_http() {
    let app = test_app();
    let repos_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let repos: Vec<accounts_repo_backend::domain::LegalEntityRepo> =
        read_json(repos_response).await;
    let repo_id = repos[0].id;

    let workspace_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{repo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let workspace: accounts_repo_backend::store::RepoWorkspace =
        read_json(workspace_response).await;

    let sign_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/client-signoff",
                    workspace.review_pack.id
                ))
                .header("x-dev-user-id", "seed-owner")
                .header("x-dev-user-name", "Hazli Johar")
                .header("x-dev-user-email", "hazli@nusantara.test")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Hazli Johar",
                        "note": "Trying to sign early"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(sign_response.status(), StatusCode::BAD_REQUEST);
    let error: serde_json::Value = read_json(sign_response).await;
    assert_eq!(
        error["error"],
        "reviewer approval is required before client sign-off"
    );
}

#[tokio::test]
async fn creates_correction_commit_without_rewriting_existing_history_through_http() {
    let app = test_app();
    let repos_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let repos: Vec<accounts_repo_backend::domain::LegalEntityRepo> =
        read_json(repos_response).await;
    let repo_id = repos[0].id;

    let before_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{repo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let before: accounts_repo_backend::store::RepoWorkspace = read_json(before_response).await;
    let before_commit_ids = before
        .commits
        .iter()
        .map(|commit| commit.id)
        .collect::<Vec<_>>();

    let correction_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/repos/{}/branches/{}/correction-commits",
                    repo_id, before.branch.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Aina Rahman",
                        "message": "Append correction for bank charge presentation",
                        "reference": "AJ-003",
                        "description": "Reclass bank charges into administrative expenses",
                        "rationale": "Reviewer requested presentation under administrative expenses",
                        "lines": [
                            {"account_code": "6000", "amount": "3900.00"},
                            {"account_code": "6400", "amount": "-3900.00"}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(correction_response.status(), StatusCode::CREATED);

    let after_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{repo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after: accounts_repo_backend::store::RepoWorkspace = read_json(after_response).await;

    assert_eq!(after.commits.len(), before.commits.len() + 1);
    assert_eq!(
        after
            .commits
            .iter()
            .take(before_commit_ids.len())
            .map(|commit| commit.id)
            .collect::<Vec<_>>(),
        before_commit_ids
    );
}

#[tokio::test]
async fn rejects_correction_commit_after_client_signoff_through_http() {
    let app = test_app();
    let repos_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let repos: Vec<accounts_repo_backend::domain::LegalEntityRepo> =
        read_json(repos_response).await;
    let repo_id = repos[0].id;

    let before_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{repo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let before: accounts_repo_backend::store::RepoWorkspace = read_json(before_response).await;

    let approve_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/reviewer-approval",
                    before.review_pack.id
                ))
                .header("x-dev-user-id", "seed-reviewer")
                .header("x-dev-user-name", "Amjad Salleh")
                .header("x-dev-user-email", "amjad@ahadvisory.test")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Amjad Salleh",
                        "note": "Reviewed"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approve_response.status(), StatusCode::CREATED);

    let sign_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/client-signoff",
                    before.review_pack.id
                ))
                .header("x-dev-user-id", "seed-owner")
                .header("x-dev-user-name", "Hazli Johar")
                .header("x-dev-user-email", "hazli@nusantara.test")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Hazli Johar",
                        "note": "Signed"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sign_response.status(), StatusCode::CREATED);

    let correction_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/repos/{}/branches/{}/correction-commits",
                    repo_id, before.branch.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Aina Rahman",
                        "message": "Attempt correction after sign-off",
                        "reference": "AJ-003",
                        "description": "Reclass bank charges into administrative expenses",
                        "rationale": "Reviewer requested presentation under administrative expenses",
                        "lines": [
                            {"account_code": "6000", "amount": "3900.00"},
                            {"account_code": "6400", "amount": "-3900.00"}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(correction_response.status(), StatusCode::CONFLICT);
    let error: serde_json::Value = read_json(correction_response).await;
    assert_eq!(
        error["error"],
        "period branch is frozen after client sign-off"
    );

    let after_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{repo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after: accounts_repo_backend::store::RepoWorkspace = read_json(after_response).await;

    assert_eq!(after.commits.len(), before.commits.len());
    assert_eq!(
        after.branch.status,
        accounts_repo_backend::domain::BranchStatus::Frozen
    );
}

#[tokio::test]
async fn rejects_preparer_reviewer_approval_to_prevent_self_review_through_http() {
    let app = test_app();
    let (_, workspace) = seeded_repo_workspace(&app).await;

    let approve_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/reviewer-approval",
                    workspace.review_pack.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Amjad Salleh",
                        "note": "Preparer tries to approve their own work"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(approve_response.status(), StatusCode::FORBIDDEN);
    let error: serde_json::Value = read_json(approve_response).await;
    assert_eq!(
        error["error"],
        "authenticated user does not have the required repo role"
    );

    let review_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/review-packs/{}", workspace.review_pack.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let review_pack: ReviewPack = read_json(review_response).await;

    assert_eq!(review_pack.status, ReviewStatus::InReview);
    assert!(review_pack.approvals.is_empty());
}

#[tokio::test]
async fn rejects_reviewer_correction_commit_to_prevent_approval_bypass_through_http() {
    let app = test_app();
    let (repo, before) = seeded_repo_workspace(&app).await;

    let correction_response = app
        .clone()
        .oneshot(
            reviewer_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/repos/{}/branches/{}/correction-commits",
                    repo.id, before.branch.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "actor_name": "Amjad Salleh",
                        "message": "Reviewer attempts to alter reviewed numbers",
                        "reference": "AJ-003",
                        "description": "Move bank charges after review",
                        "rationale": "This would bypass preparer controls",
                        "lines": [
                            {"account_code": "6000", "amount": "3900.00"},
                            {"account_code": "6400", "amount": "-3900.00"}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(correction_response.status(), StatusCode::FORBIDDEN);
    let error: serde_json::Value = read_json(correction_response).await;
    assert_eq!(
        error["error"],
        "authenticated user does not have the required repo role"
    );

    let after_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{}", repo.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after: RepoWorkspace = read_json(after_response).await;

    assert_eq!(after.commits.len(), before.commits.len());
    assert_eq!(after.branch.head_commit_id, before.branch.head_commit_id);
}

#[tokio::test]
async fn rejects_reviewer_approval_while_queries_open_to_prevent_unresolved_matters_through_http() {
    let app = test_app();
    let (_, workspace) = seeded_repo_workspace(&app).await;

    let query_response = app
        .clone()
        .oneshot(
            reviewer_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/queries",
                    workspace.review_pack.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "Confirm revenue cut-off support before approval",
                        "assigned_to": "Aina Rahman"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(query_response.status(), StatusCode::CREATED);
    let query: ReviewQuery = read_json(query_response).await;
    assert_eq!(query.status, QueryStatus::Open);

    let blocked_approval_response = app
        .clone()
        .oneshot(
            reviewer_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/reviewer-approval",
                    workspace.review_pack.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"note": "Approving despite unresolved query"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked_approval_response.status(), StatusCode::CONFLICT);
    let error: serde_json::Value = read_json(blocked_approval_response).await;
    assert_eq!(error["error"], "review pack has open queries");

    let resolve_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/queries/{}/resolve",
                    workspace.review_pack.id, query.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"note": "Revenue cut-off schedule attached and reviewed"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resolve_response.status(), StatusCode::OK);
    let resolved_query: ReviewQuery = read_json(resolve_response).await;
    assert_eq!(resolved_query.status, QueryStatus::Resolved);

    let approval_response = app
        .clone()
        .oneshot(
            reviewer_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/reviewer-approval",
                    workspace.review_pack.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"note": "Resolved and approved"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approval_response.status(), StatusCode::CREATED);

    let review_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/review-packs/{}", workspace.review_pack.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let review_pack: ReviewPack = read_json(review_response).await;
    assert_eq!(review_pack.status, ReviewStatus::ReviewerApproved);
}

#[tokio::test]
async fn rejects_signed_export_before_client_signoff_to_prevent_unsigned_evidence_pack_through_http()
 {
    let app = test_app();
    let (_, workspace) = seeded_repo_workspace(&app).await;

    let export_response = app
        .oneshot(
            reviewer_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/signed-export",
                    workspace.review_pack.id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(export_response.status(), StatusCode::CONFLICT);
    let error: serde_json::Value = read_json(export_response).await;
    assert_eq!(error["error"], "review pack must be signed before export");
}

#[tokio::test]
async fn maintains_audit_hash_chain_when_signed_pack_is_exported_through_http() {
    let app = test_app();
    let (repo, workspace) = seeded_repo_workspace(&app).await;

    let approve_response = app
        .clone()
        .oneshot(
            reviewer_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/reviewer-approval",
                    workspace.review_pack.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(json!({"note": "Reviewed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approve_response.status(), StatusCode::CREATED);

    let sign_response = app
        .clone()
        .oneshot(
            owner_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/client-signoff",
                    workspace.review_pack.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(json!({"note": "Signed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sign_response.status(), StatusCode::CREATED);

    let export_response = app
        .clone()
        .oneshot(
            owner_headers(Request::builder())
                .method("POST")
                .uri(format!(
                    "/api/review-packs/{}/signed-export",
                    workspace.review_pack.id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_response.status(), StatusCode::OK);
    let export_payload: serde_json::Value = read_json(export_response).await;
    assert_eq!(export_payload["review_pack"]["status"], "signed");
    assert_eq!(
        export_payload["commit"]["id"],
        workspace.review_pack.commit_id.to_string()
    );
    assert_eq!(
        export_payload["audit_events"]
            .as_array()
            .and_then(|events| events.last())
            .and_then(|event| event["event_type"].as_str()),
        Some("signed_pack_exported")
    );

    let audit_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/repos/{}/audit", repo.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let audit_events: Vec<AuditEvent> = read_json(audit_response).await;

    assert_eq!(
        audit_events.last().map(|event| &event.event_type),
        Some(&AuditEventType::SignedPackExported)
    );
    for (index, event) in audit_events.iter().enumerate() {
        assert_eq!(event.sequence_number, index as u64 + 1);
        assert_eq!(event.event_hash.len(), 64);

        if index == 0 {
            assert!(event.previous_hash.is_none());
        } else {
            assert_eq!(
                event.previous_hash.as_deref(),
                Some(audit_events[index - 1].event_hash.as_str())
            );
        }
    }
}
