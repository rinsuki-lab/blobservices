use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use blobservices_core::proto;
use tokio::net::TcpListener;

use crate::state::AppState;

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
        .route("/v1/refs/{namespace}/{*key}", get(|
            State(state): State<AppState>,
            Path(nk): Path<NamespaceAndKey>,
        | async move {
            println!("{:?}", nk);
            let res = sqlx::query!(
                "SELECT blobs.* FROM blob_references INNER JOIN blobs ON blobs.id = blob_references.blob_id WHERE namespace = $1 AND key = $2 LIMIT 1",
                nk.namespace,
                nk.key
            )
                .fetch_one(&state.db_pool)
                .await;
            let res = match res {
                Ok(r) => r,
                Err(sqlx::Error::RowNotFound) => {
                    return StatusCode::NOT_FOUND.into_response()
                },
                Err(e) => {
                    tracing::error!(err=?e, "FAILED_TO_QUERY_REF");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            };

            let locations = sqlx::query!(
                "SELECT * FROM blob_locations WHERE blob_id = $1",
                &res.id
            )
                .fetch_all(&state.db_pool)
                .await;
            let locations = match locations {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(err=?e, "FAILED_TO_QUERY_LOCATIONS");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            };

            let res = proto::manager::GetBlobInfoByNamespaceAndKeyResponse {
                blob: Some(proto::manager::BlobInfo {
                    id: res.id.as_bytes().to_vec(),
                    size: res.size as u64,
                    hashes: Some(proto::manager::BlobHashes {
                        ..Default::default() // TODO
                    }),
                }),
                locations: locations.into_iter().map(|l| {
                    proto::manager::BlobLocation {
                        address: l.address,
                        storage: l.storage_id,
                    }
                }).collect(),
            };

            Json(res).into_response()
        }))
        .route("/v1/refs/{namespace}/{*key}", put(|| async {}))
        .route("/v1/storages/{storage}/start_upload", post(|| async {}))
        .with_state(state)
    ;

    let listener = std::env::var("BLOBMANAGER_LISTEN_ADDR");
    let listener = listener.unwrap_or_else(|_| "0.0.0.0:3001".to_string());
    tracing::trace!("trying to listen at {}", listener);
    let listener = TcpListener::bind(listener)
        .await
        .expect("failed to listen server");
    tracing::info!("listening at {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
