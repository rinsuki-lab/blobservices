use axum::{
    Router,
    routing::{get, post},
};

mod handlers;
mod provider;
mod state;
pub use provider::BlobProvider;

use crate::state::AppStateInner;

pub async fn run<P: BlobProvider + 'static>(provider: P) {
    let state = AppStateInner::new(provider).await;
    let app = Router::new()
        .route("/v1/simple", post(handlers::v1::put_object_simple))
        .route(
            "/v1/simple/{*address}",
            get(handlers::v1::get_object_simple),
        )
        .route(
            "/v1/hashes/{*address}",
            get(handlers::v1::get_object_hashes),
        );
    let app = state.provider.add_additional_routes(app);
    let app = app.with_state(state.clone());

    // calc listener
    let listener = std::env::var(format!("{}_LISTEN_ADDR", P::env_prefix()));
    let listener = listener.unwrap_or_else(|_| "0.0.0.0:3002".to_string());
    tracing::trace!(
        "trying to listen at {} (you can change with {}_LISTEN_ADDR env)",
        listener,
        P::env_prefix()
    );
    let listener = tokio::net::TcpListener::bind(listener)
        .await
        .expect("failed to listen server");
    tracing::info!("listening at {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
