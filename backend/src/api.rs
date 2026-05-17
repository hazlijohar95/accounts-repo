use crate::{
    auth::{AuthConfig, AuthError},
    contract::api_contract,
    domain::DomainError,
    persistence::PersistentState,
    store::{
        AppStore, ApprovalRequest, CorrectionCommitRequest, ResolveReviewQueryRequest,
        ReviewQueryRequest, StoreError, WorkspaceImportRequest,
    },
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<AppStore>>,
    pub auth: AuthConfig,
    pub persistence: Option<PersistentState>,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/meta/contract", get(contract))
        .route("/api/repos", get(list_repos))
        .route("/api/imports/year-end-review-pack", post(import_workspace))
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
        .route(
            "/api/review-packs/{review_pack_id}/queries",
            post(open_review_query),
        )
        .route(
            "/api/review-packs/{review_pack_id}/queries/{query_id}/resolve",
            post(resolve_review_query),
        )
        .route(
            "/api/review-packs/{review_pack_id}/signed-export",
            post(signed_pack_export),
        )
        .layer(cors_layer())
        .with_state(state)
}

async fn contract() -> Json<crate::contract::ApiContract> {
    Json(api_contract())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let store = state.store.read().await;
    Json(HealthResponse {
        status: "ok".to_string(),
        organizations: store.organization_count(),
        users: store.user_count(),
    })
}

async fn list_repos(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let store = state.store.read().await;
    Ok(Json(store.list_repos_for_actor(&actor)?))
}

async fn import_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<WorkspaceImportRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let workspace = store.import_workspace(request, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok((StatusCode::CREATED, Json(workspace)))
}

async fn repo_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let store = state.store.read().await;
    Ok(Json(store.repo_workspace_for_actor(repo_id, &actor)?))
}

async fn audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let store = state.store.read().await;
    Ok(Json(store.audit_events_for_actor(repo_id, &actor)?))
}

async fn review_pack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(review_pack_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let store = state.store.read().await;
    Ok(Json(store.review_pack_for_actor(review_pack_id, &actor)?))
}

async fn approve_reviewer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(review_pack_id): Path<Uuid>,
    Json(request): Json<ApprovalRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let approval = store.approve_reviewer(review_pack_id, request, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok((StatusCode::CREATED, Json(approval)))
}

async fn sign_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(review_pack_id): Path<Uuid>,
    Json(request): Json<ApprovalRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let approval = store.sign_client(review_pack_id, request, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok((StatusCode::CREATED, Json(approval)))
}

async fn commit_correction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((repo_id, branch_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CorrectionCommitRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let commit = store.commit_correction(repo_id, branch_id, request, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok((StatusCode::CREATED, Json(commit)))
}

#[derive(Debug)]
pub enum ApiError {
    Store(StoreError),
    Auth(AuthError),
    Internal(String),
}

impl From<StoreError> for ApiError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

impl From<AuthError> for ApiError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Auth(AuthError::Missing) => (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
            ),
            ApiError::Auth(AuthError::Rejected | AuthError::Unavailable) => (
                StatusCode::UNAUTHORIZED,
                "Could not validate authentication session".to_string(),
            ),
            ApiError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
            ApiError::Store(error) => match error {
                StoreError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
                StoreError::InvalidImport(message) => (StatusCode::BAD_REQUEST, message),
                StoreError::Forbidden(message) => (StatusCode::FORBIDDEN, message),
                StoreError::Conflict(message) => (StatusCode::CONFLICT, message),
                StoreError::Domain(error) => match error {
                    DomainError::AlreadySigned
                    | DomainError::DuplicateApproval
                    | DomainError::FrozenBranch
                    | DomainError::BlockingQueriesOpen
                    | DomainError::DuplicateAdjustmentReference(_) => {
                        (StatusCode::CONFLICT, error.to_string())
                    }
                    _ => (StatusCode::BAD_REQUEST, error.to_string()),
                },
            },
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

async fn open_review_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(review_pack_id): Path<Uuid>,
    Json(request): Json<ReviewQueryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let query = store.open_review_query(review_pack_id, request, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok((StatusCode::CREATED, Json(query)))
}

async fn resolve_review_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((review_pack_id, query_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ResolveReviewQueryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let query = store.resolve_review_query(review_pack_id, query_id, request, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok(Json(query))
}

async fn signed_pack_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(review_pack_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.auth.actor_from_headers(&headers).await?;
    let mut store = state.store.write().await;
    let export = store.signed_pack_export(review_pack_id, &actor)?;
    persist_if_configured(&state, &store).await?;
    Ok(Json(export))
}

async fn persist_if_configured(state: &AppState, store: &AppStore) -> Result<(), ApiError> {
    if let Some(persistence) = &state.persistence {
        persistence
            .save_store(store)
            .await
            .map_err(|error| ApiError::Internal(format!("Failed to persist app state: {error}")))?;
    }

    Ok(())
}

fn cors_layer() -> CorsLayer {
    let allowed_origin = std::env::var("CORS_ALLOWED_ORIGIN")
        .ok()
        .and_then(|origin| origin.parse::<HeaderValue>().ok())
        .unwrap_or_else(|| HeaderValue::from_static("http://127.0.0.1:5173"));

    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true)
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
