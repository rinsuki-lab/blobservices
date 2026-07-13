use axum::{
    Router,
    routing::{get, post},
};
use tokio::net::TcpListener;

mod extractors;
mod handlers;
mod state;

#[derive(serde::Deserialize, std::fmt::Debug)]
struct NamespaceAndKey {
    namespace: String,
    key: String,
}

#[tokio::main]
async fn main() {
    blobservices_core::init_tracing_registry();
    tracing::info!("Hello, world!");

    let state = state::AppStateInner::new().await;

    let app = Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .route(
            "/v1/refs/{namespace}/{*key}",
            get(handlers::v1::get_blob_ref).put(handlers::v1::put_blob_ref),
        )
        .route("/v1/storages/{storage}/start_upload", post(|| async {}))
        .with_state(state);

    let listener = std::env::var("BLOBMANAGER_LISTEN_ADDR");
    let listener = listener.unwrap_or_else(|_| "0.0.0.0:3001".to_string());
    tracing::trace!("trying to listen at {}", listener);
    let listener = TcpListener::bind(listener)
        .await
        .expect("failed to listen server");
    tracing::info!("listening at {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
