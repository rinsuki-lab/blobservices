use axum::{Router, routing::get};
use tokio::net::TcpListener;

mod config;
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
            "/v1/content/by-ref/{namespace}/{*key}",
            get(handlers::v1::get_blob_content_by_ref).put(handlers::v1::put_blob_content_by_ref),
        )
        .with_state(state);

    let listener = std::env::var("BLOBGATEWAY_LISTEN_ADDR");
    let listener = listener.unwrap_or_else(|_| "0.0.0.0:3003".to_string());
    tracing::info!("trying to listen at {}", listener);
    let listener = TcpListener::bind(listener)
        .await
        .expect("failed to listen server");
    axum::serve(listener, app).await.unwrap();
}
