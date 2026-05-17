use crate::actor::AuthenticatedActor;
use axum::http::{HeaderMap, header};
use reqwest::StatusCode;
use serde::Deserialize;
use std::net::{IpAddr, ToSocketAddrs};
use thiserror::Error;

const DEVELOPMENT_INTERNAL_TOKEN: &str = "development-internal-token";

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

#[derive(Debug, Error)]
pub enum AuthConfigError {
    #[error("AUTH_INTERNAL_TOKEN is required when Better Auth is enabled")]
    MissingInternalToken,
    #[error("AUTH_INTERNAL_TOKEN must not use the development placeholder value")]
    DevelopmentInternalToken,
    #[error("ACCOUNTS_REPO_AUTH_DISABLED_DEV can only be used with a loopback bind address")]
    UnsafeDevAuthBind,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, AuthConfigError> {
        Self::from_env_values(
            std::env::var("ACCOUNTS_REPO_AUTH_DISABLED_DEV").ok(),
            std::env::var("ACCOUNTS_REPO_BIND_ADDR").ok(),
            std::env::var("AUTH_SERVICE_URL").ok(),
            std::env::var("AUTH_INTERNAL_TOKEN").ok(),
        )
    }

    fn from_env_values(
        auth_disabled_dev: Option<String>,
        bind_addr: Option<String>,
        service_url: Option<String>,
        internal_token: Option<String>,
    ) -> Result<Self, AuthConfigError> {
        if auth_disabled_dev.as_deref() == Some("1") {
            let bind_addr = bind_addr.unwrap_or_else(|| "127.0.0.1:8080".to_string());
            if !is_loopback_bind_addr(&bind_addr) {
                return Err(AuthConfigError::UnsafeDevAuthBind);
            }

            return Ok(Self {
                mode: AuthMode::DisabledDev,
            });
        }

        let internal_token = internal_token
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty())
            .ok_or(AuthConfigError::MissingInternalToken)?;
        if internal_token == DEVELOPMENT_INTERNAL_TOKEN {
            return Err(AuthConfigError::DevelopmentInternalToken);
        }
        if internal_token.starts_with("replace-with-") {
            return Err(AuthConfigError::DevelopmentInternalToken);
        }

        Ok(Self {
            mode: AuthMode::BetterAuth {
                service_url: service_url.unwrap_or_else(|| "http://127.0.0.1:8081".to_string()),
                internal_token,
            },
        })
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

fn is_loopback_bind_addr(bind_addr: &str) -> bool {
    let Ok(addresses) = bind_addr.to_socket_addrs() else {
        return false;
    };

    addresses.into_iter().all(|address| match address.ip() {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback(),
    })
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
    if !session.user.email_verified {
        return Err(AuthError::Rejected);
    }

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
    #[serde(rename = "emailVerified")]
    email_verified: bool,
}

#[cfg(test)]
mod tests {
    use super::{AuthConfig, AuthConfigError, AuthMode};

    #[test]
    fn rejects_dev_auth_on_public_bind_to_prevent_header_spoofing_in_production() {
        let result = AuthConfig::from_env_values(
            Some("1".to_string()),
            Some("0.0.0.0:8080".to_string()),
            None,
            None,
        );

        assert!(matches!(result, Err(AuthConfigError::UnsafeDevAuthBind)));
    }

    #[test]
    fn accepts_dev_auth_on_loopback_for_explicit_local_flows() {
        let config = AuthConfig::from_env_values(
            Some("1".to_string()),
            Some("127.0.0.1:18080".to_string()),
            None,
            None,
        )
        .expect("loopback dev auth should be accepted");

        assert!(matches!(config.mode, AuthMode::DisabledDev));
    }

    #[test]
    fn rejects_missing_internal_token_to_prevent_public_default_secret() {
        let result = AuthConfig::from_env_values(None, None, None, None);

        assert!(matches!(result, Err(AuthConfigError::MissingInternalToken)));
    }

    #[test]
    fn rejects_development_internal_token_placeholder() {
        let result = AuthConfig::from_env_values(
            None,
            None,
            None,
            Some("development-internal-token".to_string()),
        );

        assert!(matches!(
            result,
            Err(AuthConfigError::DevelopmentInternalToken)
        ));
    }

    #[test]
    fn rejects_example_internal_token_placeholder() {
        let result = AuthConfig::from_env_values(
            None,
            None,
            None,
            Some("replace-with-32-plus-character-local-internal-token".to_string()),
        );

        assert!(matches!(
            result,
            Err(AuthConfigError::DevelopmentInternalToken)
        ));
    }
}
