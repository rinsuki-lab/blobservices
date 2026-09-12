use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use uuid::Uuid;

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

    let blob_id = make_blob_from_content(&mut tx, body).await?;

    tx.commit().await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_COMMIT_BLOB_TX");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let mut tx = state.db_pool.begin().await.map_err(|e| {
        tracing::error!(err = ?e, "FAILED_TO_BEGIN_TX");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let new_reference_id = Uuid::now_v7();
    let revision_id = Uuid::now_v7();

    let reference_id = sqlx::query!(
        r#"
        INSERT INTO blob_references (id, namespace, key, current_revision_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (namespace, key) DO UPDATE
        SET current_revision_id = EXCLUDED.current_revision_id
        RETURNING id
        "#,
        new_reference_id,
        nk.namespace,
        nk.key,
        revision_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(err = ?e, "FAILED_TO_UPSERT_BLOB_REF");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?
    .id;

    sqlx::query!(
        r#"
        INSERT INTO blob_reference_revisions (id, reference_id, blob_id)
        VALUES ($1, $2, $3)
        "#,
        revision_id,
        reference_id,
        blob_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(err = ?e, "FAILED_TO_INSERT_BLOB_REF_REVISION");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(err = ?e, "FAILED_TO_COMMIT");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    Ok(StatusCode::CREATED.into_response())
}

async fn make_blob_from_content(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: proto::manager::PutBlobRefRequest,
) -> Result<Uuid, Response> {
    let content = request.content.ok_or_else(|| {
        tracing::info!("MISSING_CONTENT");
        StatusCode::BAD_REQUEST.into_response()
    })?;
    Ok(match content {
        proto::manager::put_blob_ref_request::Content::UnsafeSetBlobId(recipe) => {
            recipe.blob_id.try_into().unwrap()
        }
        proto::manager::put_blob_ref_request::Content::UnsafeNewBlob(recipe) => {
            // TODO: 既にlocationが使われているかを確認し、使われていたらそのlocationが使われているblobと{size,hashes}が被るかを確認する
            let blob_id = insert_new_blob(tx, recipe.size, recipe.hashes).await?;
            insert_new_location(tx, blob_id, &recipe.storage, &recipe.address).await?;
            blob_id
        }
        proto::manager::put_blob_ref_request::Content::FromBlobSlice(recipe) => {
            let parent_id = Box::pin(make_blob_from_content(tx, *recipe.source)).await?;

            let res = sqlx::query!(
                "SELECT dst_blob_id FROM blob_slices JOIN blobs dst ON dst.id = blob_slices.dst_blob_id WHERE src_blob_id = $1 AND start = $2 AND dst.size = $3",
                parent_id,
                recipe.start as i64,
                recipe.size as i64
            )
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| {
                tracing::error!(err=?e, "FAILED_TO_FIND_SLICE");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;

            if let Some(res) = res {
                check_current_blob(tx, res.dst_blob_id, recipe.size, recipe.hashes).await?;
                res.dst_blob_id
            } else {
                let parent_size =
                    sqlx::query!("SELECT size FROM blobs WHERE id = $1 LIMIT 1", parent_id)
                        .fetch_one(&mut **tx)
                        .await
                        .map_err(|e| {
                            tracing::error!(err=?e, "FAILED_TO_QUERY_PARENT");
                            StatusCode::INTERNAL_SERVER_ERROR.into_response()
                        })?
                        .size as u64;
                let recipe_end = recipe.start.checked_add(recipe.size).ok_or_else(|| {
                    tracing::warn!(recipe.start, recipe.size, "RECIPE_TOO_BIG");
                    StatusCode::BAD_REQUEST.into_response()
                })?;
                if recipe_end > parent_size {
                    tracing::warn!(recipe_end, parent_size, "RECIPE_OVERRUN");
                    return Err(StatusCode::BAD_REQUEST.into_response());
                }

                let blob_id = insert_new_blob(tx, recipe.size, recipe.hashes).await?;

                sqlx::query!(
                    "INSERT INTO blob_slices(src_blob_id, dst_blob_id, start) VALUES($1, $2, $3)",
                    parent_id,
                    blob_id,
                    recipe.start as i64
                )
                .execute(&mut **tx)
                .await
                .map_err(|e| {
                    tracing::error!(err=?e, "FAILED_TO_INSERT_SLICE");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                })?;

                blob_id
            }
        }
        proto::manager::put_blob_ref_request::Content::FromBlobTransform(_recipe) => {
            // TODO: 実装
            return Err(StatusCode::NOT_IMPLEMENTED.into_response());
        }
        proto::manager::put_blob_ref_request::Content::FromBlobConcat(_recipe) => {
            // TODO: 実装
            return Err(StatusCode::NOT_IMPLEMENTED.into_response());
        }
    })
}

async fn insert_new_blob(
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

async fn insert_new_location(
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

async fn check_current_blob(
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
