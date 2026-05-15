use accounts_repo_backend::{AppState, app, store::AppStore};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    app(AppState {
        store: Arc::new(RwLock::new(AppStore::seeded())),
    })
}

async fn read_json<T: DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
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
