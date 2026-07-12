use std::sync::Arc;

pub struct AppStateInner {
    pub db_pool: sqlx::Pool<sqlx::Postgres>,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn new() -> AppState {
        let url = std::env::var("BLOBMANAGER_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("BLOBMANAGER_DATABASE_URL or DATABASE_URL env is required");
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .connect(&url)
            .await
            .expect("Failed to connect Postgres");
        AppState::new(AppStateInner { db_pool })
    }
}
