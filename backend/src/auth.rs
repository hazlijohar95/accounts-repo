use crate::store::AuthenticatedActor;
use axum::http::{HeaderMap, header};
use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;

#[derive(Clone, Debug)]
pub enum AuthMode {
    BetterAuth {
        service_url: String,
        internal_token: String,
    },
    DisabledDev,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    mode: AuthMode,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("authentication required")]
    Missing,
    #[error("auth service rejected session")]
    Rejected,
    #[error("auth service unavailable")]
    Unavailable,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        if std::env::var("ACCOUNTS_REPO_AUTH_DISABLED_DEV")
            .ok()
            .as_deref()
            == Some("1")
        {
            return Self {
                mode: AuthMode::DisabledDev,
            };
        }

        Self {
            mode: AuthMode::BetterAuth {
                service_url: std::env::var("AUTH_SERVICE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string()),
                internal_token: std::env::var("AUTH_INTERNAL_TOKEN")
                    .unwrap_or_else(|_| "development-internal-token".to_string()),
            },
        }
    }

    pub fn disabled_dev() -> Self {
        Self {
            mode: AuthMode::DisabledDev,
        }
    }

    pub fn better_auth(service_url: impl Into<String>, internal_token: impl Into<String>) -> Self {
        Self {
            mode: AuthMode::BetterAuth {
                service_url: service_url.into(),
                internal_token: internal_token.into(),
            },
        }
    }

    pub async fn actor_from_headers(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthenticatedActor, AuthError> {
        match &self.mode {
            AuthMode::DisabledDev => {
                Ok(
                    dev_actor_from_headers(headers).unwrap_or_else(|| AuthenticatedActor {
                        auth_user_id: "seed-preparer".to_string(),
                        display_name: "Aina Rahman".to_string(),
                        email: "aina@ahadvisory.test".to_string(),
                    }),
                )
            }
            AuthMode::BetterAuth {
                service_url,
                internal_token,
            } => actor_from_better_auth(headers, service_url, internal_token).await,
        }
    }
}

fn dev_actor_from_headers(headers: &HeaderMap) -> Option<AuthenticatedActor> {
    let email = headers.get("x-dev-user-email")?.to_str().ok()?.to_string();
    let display_name = headers
        .get("x-dev-user-name")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(email.as_str())
        .to_string();
    let auth_user_id = headers
        .get("x-dev-user-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(email.as_str())
        .to_string();

    Some(AuthenticatedActor {
        auth_user_id,
        display_name,
        email,
    })
}

async fn actor_from_better_auth(
    headers: &HeaderMap,
    service_url: &str,
    internal_token: &str,
) -> Result<AuthenticatedActor, AuthError> {
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .ok_or(AuthError::Missing)?;
    let endpoint = format!("{}/internal/session", service_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(endpoint)
        .header(header::COOKIE.as_str(), cookie)
        .header("x-internal-auth-token", internal_token)
        .send()
        .await
        .map_err(|_| AuthError::Unavailable)?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(AuthError::Missing);
    }
    if !response.status().is_success() {
        return Err(AuthError::Rejected);
    }

    let session = response
        .json::<InternalSession>()
        .await
        .map_err(|_| AuthError::Rejected)?;

    Ok(AuthenticatedActor {
        auth_user_id: session.user.id,
        display_name: session.user.name,
        email: session.user.email,
    })
}

#[derive(Debug, Deserialize)]
struct InternalSession {
    user: InternalUser,
}

#[derive(Debug, Deserialize)]
struct InternalUser {
    id: String,
    name: String,
    email: String,
}
