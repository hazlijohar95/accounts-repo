use crate::store::{AppStore, ApprovalRequest, CorrectionCommitRequest, StoreError};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<AppStore>>,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/repos", get(list_repos))
        .route("/api/repos/{repo_id}", get(repo_workspace))
        .route("/api/repos/{repo_id}/audit", get(audit_events))
        .route(
            "/api/repos/{repo_id}/branches/{branch_id}/correction-commits",
            post(commit_correction),
        )
        .route("/api/review-packs/{review_pack_id}", get(review_pack))
        .route(
            "/api/review-packs/{review_pack_id}/reviewer-approval",
            post(approve_reviewer),
        )
        .route(
            "/api/review-packs/{review_pack_id}/client-signoff",
            post(sign_client),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let store = state.store.read().await;
    Json(HealthResponse {
        status: "ok".to_string(),
        organizations: store.organization_count(),
        users: store.user_count(),
    })
}

async fn list_repos(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.list_repos()?))
}

async fn repo_workspace(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.repo_workspace(repo_id)?))
}

async fn audit_events(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.audit_events(repo_id)?))
}

async fn review_pack(
    State(state): State<AppState>,
    Path(review_pack_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.review_pack(review_pack_id)?))
}

async fn approve_reviewer(
    State(state): State<AppState>,
    Path(review_pack_id): Path<Uuid>,
    Json(request): Json<ApprovalRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut store = state.store.write().await;
    Ok((
        StatusCode::CREATED,
        Json(store.approve_reviewer(review_pack_id, request)?),
    ))
}

async fn sign_client(
    State(state): State<AppState>,
    Path(review_pack_id): Path<Uuid>,
    Json(request): Json<ApprovalRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut store = state.store.write().await;
    Ok((
        StatusCode::CREATED,
        Json(store.sign_client(review_pack_id, request)?),
    ))
}

async fn commit_correction(
    State(state): State<AppState>,
    Path((repo_id, branch_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CorrectionCommitRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut store = state.store.write().await;
    Ok((
        StatusCode::CREATED,
        Json(store.commit_correction(repo_id, branch_id, request)?),
    ))
}

#[derive(Debug)]
pub struct ApiError(StoreError);

impl From<StoreError> for ApiError {
    fn from(value: StoreError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            StoreError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            StoreError::Domain(error) => (StatusCode::BAD_REQUEST, error.to_string()),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    organizations: usize,
    users: usize,
}
