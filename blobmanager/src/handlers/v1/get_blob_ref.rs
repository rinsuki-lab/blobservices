use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use blobservices_core::proto;

use crate::{NamespaceAndKey, state::AppState};

pub async fn get_blob_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
) -> Response {
    let res = sqlx::query!(
        "SELECT blobs.* FROM blob_references INNER JOIN blobs ON blobs.id = blob_references.blob_id WHERE namespace = $1 AND key = $2 LIMIT 1",
        nk.namespace,
        nk.key
    )
        .fetch_one(&state.db_pool)
        .await;
    let res = match res {
        Ok(r) => r,
        Err(sqlx::Error::RowNotFound) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(err=?e, "FAILED_TO_QUERY_REF");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let locations = sqlx::query!("SELECT * FROM blob_locations WHERE blob_id = $1", &res.id)
        .fetch_all(&state.db_pool)
        .await;
    let locations = match locations {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(err=?e, "FAILED_TO_QUERY_LOCATIONS");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
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
        locations: locations
            .into_iter()
            .map(|l| proto::manager::BlobLocation {
                address: l.address,
                storage: l.storage_id,
            })
            .collect(),
    };

    Json(res).into_response()
}
