use axum::{
    Router,
    routing::{get, post},
};
use tokio::net::TcpListener;

mod handlers;
mod state;

#[tokio::main]
async fn main() {
    blobservices_core::init_tracing_registry();
    tracing::info!("Hello, world!");

    let state = state::AppStateInner::new().await;

    let app = Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .route("/v1/simple", post(handlers::v1::put_object_simple))
        .nest_service(
            "/v1/simple/",
            tower_http::services::ServeDir::new(&state.root_dir)
                .append_index_html_on_directories(false),
        )
        .with_state(state);

    let listener = std::env::var("BLOBSTORE_LOCAL_LISTEN_ADDR");
    let listener = listener.unwrap_or_else(|_| "0.0.0.0:3002".to_string());
    tracing::trace!("trying to listen at {}", listener);
    let listener = TcpListener::bind(listener)
        .await
        .expect("failed to listen server");
    tracing::info!("listening at {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
