use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use blobservices_core::proto;

use crate::{NamespaceAndKey, extractors::ResponseFormat, state::AppState};

pub async fn get_blob_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
    response_format: ResponseFormat,
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

    let res = proto::manager::GetBlobRefResponse {
        blob: Some(proto::manager::BlobInfo {
            id: res.id.as_bytes().to_vec(),
            size: res.size as u64,
            hashes: Some(proto::manager::BlobHashes {
                crc32: res.cs_crc32.map(|hash| hash as u32),
                crc32c: res.cs_crc32c.map(|hash| hash as u32),
                xxh64: res.cs_xxh64.map(|hash| hash as u64),
                md5: res.cs_md5,
                sha1: res.cs_sha1,
                sha256: res.cs_sha256,
                sha256_dropbox: res.cs_sha256_dropbox,
                sha512: res.cs_sha512,
                sha3_512: res.cs_sha3_512,
                blake2sp: res.cs_blake2sp,
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

    response_format.message_to_response(res)
}
