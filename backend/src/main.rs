use accounts_repo_backend::{AppState, app, store::AppStore};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "accounts_repo_backend=debug,tower_http=debug,axum=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        store: Arc::new(RwLock::new(AppStore::empty())),
    };

    let router = app(state).layer(TraceLayer::new_for_http());
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("accounts repo API listening on http://127.0.0.1:8080");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
