mod from_blob_slice;
mod from_blob_transform;
mod shared;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use uuid::Uuid;

use shared::{insert_new_blob, insert_new_location};

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
            from_blob_slice::from_blob_slice(tx, *recipe).await?
        }
        proto::manager::put_blob_ref_request::Content::FromBlobTransform(recipe) => {
            from_blob_transform::from_blob_transform(tx, *recipe).await?
        }
        proto::manager::put_blob_ref_request::Content::FromBlobConcat(_recipe) => {
            // TODO: 実装
            return Err(StatusCode::NOT_IMPLEMENTED.into_response());
        }
        proto::manager::put_blob_ref_request::Content::FromOtherRef(recipe) => {
            // TODO: 実装
            return Err(StatusCode::NOT_IMPLEMENTED.into_response());
        }
    })
}
