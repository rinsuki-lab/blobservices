use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use sqlx::types::Uuid;

use crate::{NamespaceAndKey, extractors::RequestMessage, state::AppState};

pub async fn put_blob_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
    RequestMessage(body): RequestMessage<proto::manager::PutBlobRefRequest>,
) -> Result<Response, Response> {
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        tracing::error!(err = ?e, "FAILED_TO_BEGIN_TX");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let content = body.content.ok_or_else(|| {
        tracing::info!("MISSING_CONTENT");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    let blob_id = match content {
        proto::manager::put_blob_ref_request::Content::UnsafeNewBlob(new_blob) => {
            let blob_id =
                insert_new_blob(&mut tx, new_blob.size, new_blob.hashes.unwrap_or_default())
                    .await?;
            insert_new_location(&mut tx, blob_id, &new_blob.storage, &new_blob.address).await?;
            blob_id
        }
        proto::manager::put_blob_ref_request::Content::UnsafeSetBlobId(blob_id) => {
            blob_id.blob_id.try_into().map_err(|e| {
                tracing::info!(err=?e, "FAILED_TO_PARSE_AS_UUID");
                StatusCode::BAD_REQUEST.into_response()
            })?
        }
    };

    sqlx::query!(
        "INSERT INTO blob_references (id, blob_id, namespace, key) VALUES (gen_random_uuid(), $1, $2, $3)",
        blob_id,
        nk.namespace,
        nk.key
    )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            // TODO: conflictをちゃんとハンドルする
            tracing::error!(err = ?e, "FAILED_TO_INSERT_BLOB_REF");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(err = ?e, "FAILED_TO_COMMIT");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    Ok(StatusCode::CREATED.into_response())
}

async fn insert_new_blob(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    size: u64,
    hashes: proto::manager::BlobHashes,
) -> Result<Uuid, Response> {
    let size: i64 = size.try_into().map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_CAST_I64");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    sqlx::query!(
        "INSERT INTO blobs(id, size) VALUES (gen_random_uuid(), $1) RETURNING id",
        size as i64 // TODO: hashesを保存する
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_INSERT_BLOB");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })
    .map(|r| r.id)
}

async fn insert_new_location(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    blob_id: Uuid,
    storage: &str,
    address: &str,
) -> Result<Uuid, Response> {
    sqlx::query!(
        "INSERT INTO blob_locations(id, blob_id, storage_id, address) VALUES (gen_random_uuid(), $1, $2, $3) RETURNING id",
        blob_id, storage, address
    )
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            // TODO: conflictをちゃんとハンドルする (か、上書きするかを検討する)
            tracing::error!(err=?e, "FAILED_TO_INSERT_BLOB_LOC");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
        .map(|r| r.id)
}
