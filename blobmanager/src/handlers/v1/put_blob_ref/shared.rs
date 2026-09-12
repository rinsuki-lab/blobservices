use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use uuid::Uuid;

pub(super) async fn insert_new_blob(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    size: u64,
    hashes: proto::core::BlobHashes,
) -> Result<Uuid, Response> {
    let size: i64 = size.try_into().map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_CAST_I64");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    sqlx::query!(
        r#"
        INSERT INTO blobs(
            id, size, cs_crc32, cs_crc32c, cs_xxh64, cs_md5, cs_sha1,
            cs_sha256, cs_sha256_dropbox, cs_sha512, cs_sha3_256, cs_sha3_512, cs_blake2sp
        )
        VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id
        "#,
        size,
        hashes.crc32.map(|hash| hash as i32),
        hashes.crc32c.map(|hash| hash as i32),
        hashes.xxh64.map(|hash| hash as i64),
        hashes.md5,
        hashes.sha1,
        hashes.sha256,
        hashes.sha256_dropbox,
        hashes.sha512,
        hashes.sha3_256,
        hashes.sha3_512,
        hashes.blake2sp,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_INSERT_BLOB");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })
    .map(|r| r.id)
}

pub(super) async fn insert_new_location(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    blob_id: Uuid,
    storage: &str,
    address: &str,
) -> Result<Uuid, Response> {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO blob_locations(id, blob_id, storage_id, address)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        id,
        blob_id,
        storage,
        address
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

pub(super) async fn check_current_blob(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    size: u64,
    hashes: proto::core::BlobHashes,
) -> Result<(), Response> {
    let res = sqlx::query!("SELECT * FROM blobs WHERE id = $1", id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            tracing::error!(err=?e, id=%id, "FAILED_TO_FIND_BLOB");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    if res.size != (size as i64) {
        tracing::error!(blob=%res.id, expected=size, "WRONG_SIZE");
        return Err(StatusCode::BAD_REQUEST.into_response());
    }

    // TODO: hashes を検証する

    Ok(())
}
