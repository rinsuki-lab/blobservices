use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use uuid::Uuid;

use super::{
    make_blob_from_content,
    shared::{check_current_blob, insert_new_blob},
};

pub(super) async fn from_blob_slice(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipe: proto::manager::PutBlobRefFromSlice,
) -> Result<Uuid, Response> {
    let parent_id = Box::pin(make_blob_from_content(tx, *recipe.source)).await?;

    // 完全に同じ部分を切り出しているblobがあったらそれを使い回す
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
        Ok(res.dst_blob_id)
    } else {
        let parent_size = sqlx::query!("SELECT size FROM blobs WHERE id = $1 LIMIT 1", parent_id)
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

        Ok(blob_id)
    }
}
