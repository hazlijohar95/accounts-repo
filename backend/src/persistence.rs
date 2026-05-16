use crate::store::AppStore;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
pub struct PersistentState {
    pool: PgPool,
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
        let persistence = Self { pool };
        persistence.ensure_schema().await?;
        Ok(persistence)
    }

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

    pub async fn load_store(&self) -> anyhow::Result<AppStore> {
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

    pub async fn save_store(&self, store: &AppStore) -> anyhow::Result<()> {
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
}
