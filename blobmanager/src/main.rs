use axum::{Router, routing::{get, post, put}};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    blobservices_core::init_tracing_registry();
    tracing::info!("Hello, world!");

    let app = Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .route("/v1/refs/{namespace}/{*key}", get(|| async {}))
        .route("/v1/refs/{namespace}/{*key}", put(|| async {}))
        .route("/v1/storages/{storage}/start_upload", post(|| async {}))
    ;

    let listener = std::env::var("BLOBMANAGER_LISTEN_ADDR");
    let listener = listener.unwrap_or_else(|_| { "0.0.0.0:3001".to_string() });
    tracing::info!("trying to listen at {}", listener);
    let listener = TcpListener::bind(listener)
        .await.expect("failed to listen server");
    axum::serve(listener, app).await.unwrap();
}
