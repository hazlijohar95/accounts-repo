use accounts_repo_backend::{
    AppState, app, auth::AuthConfig, domain::LegalEntityRepo, store::AppStore,
};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Arc;
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceExt;

const INTERNAL_AUTH_TOKEN: &str = "test-internal-token";

fn production_auth_app(auth_service_url: String) -> axum::Router {
    app(AppState {
        store: Arc::new(RwLock::new(AppStore::seeded())),
        auth: AuthConfig::better_auth(auth_service_url, INTERNAL_AUTH_TOKEN),
        persistence: None,
    })
}

async fn read_json<T: DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn spawn_auth_service() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new().route("/internal/session", get(internal_session));

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    format!("http://{address}")
}

async fn internal_session(headers: HeaderMap) -> Response {
    let valid_internal_token = headers
        .get("x-internal-auth-token")
        .and_then(|value| value.to_str().ok())
        == Some(INTERNAL_AUTH_TOKEN);
    if !valid_internal_token {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    let user = if cookie.contains("session=reviewer") {
        json!({"id": "seed-reviewer", "name": "Amjad Salleh", "email": "amjad@ahadvisory.test"})
    } else if cookie.contains("session=outsider") {
        json!({"id": "outsider", "name": "Outside User", "email": "outside@example.test"})
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "No active session"})),
        )
            .into_response();
    };

    (StatusCode::OK, Json(json!({"user": user}))).into_response()
}

#[tokio::test]
async fn rejects_dev_auth_headers_when_better_auth_is_enabled_to_prevent_header_spoofing() {
    let app = production_auth_app("http://127.0.0.1:9".to_string());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .header("x-dev-user-id", "seed-owner")
                .header("x-dev-user-name", "Hazli Johar")
                .header("x-dev-user-email", "hazli@nusantara.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let error: serde_json::Value = read_json(response).await;
    assert_eq!(error["error"], "Authentication required");
}

#[tokio::test]
async fn filters_repo_listing_by_better_auth_session_email_to_prevent_cross_client_disclosure() {
    let auth_service_url = spawn_auth_service().await;
    let app = production_auth_app(auth_service_url);

    let outsider_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .header(header::COOKIE, "session=outsider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(outsider_response.status(), StatusCode::OK);
    let outsider_repos: Vec<LegalEntityRepo> = read_json(outsider_response).await;
    assert!(outsider_repos.is_empty());

    let reviewer_response = app
        .oneshot(
            Request::builder()
                .uri("/api/repos")
                .header(header::COOKIE, "session=reviewer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(reviewer_response.status(), StatusCode::OK);
    let reviewer_repos: Vec<LegalEntityRepo> = read_json(reviewer_response).await;
    assert_eq!(reviewer_repos.len(), 1);
    assert_eq!(reviewer_repos[0].name, "Nusantara Precision Sdn Bhd");
}
