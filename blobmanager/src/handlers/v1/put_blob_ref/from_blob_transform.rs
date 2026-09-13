use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::{
    proto,
    transformers::{TransformerBase, make_transformer_from_proto},
};
use uuid::Uuid;

use crate::handlers::v1::put_blob_ref::{
    insert_new_blob, make_blob_from_content, shared::get_blob_size,
};

pub async fn from_blob_transform(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipe: proto::manager::PutBlobRefFromTransform,
) -> Result<Uuid, Response> {
    let parent_id = Box::pin(make_blob_from_content(tx, *recipe.source)).await?;

    // TODO: shared parameter を実装する
    if let Some(shared) = recipe.transform.shared {
        // TODO: implement
        Err(StatusCode::NOT_IMPLEMENTED.into_response())?;
    }

    let Some(transform) = recipe.transform.params.transform else {
        Err(StatusCode::BAD_REQUEST.into_response())?
    };

    let src_size = get_blob_size(tx, &parent_id).await?;

    let base = make_transformer_from_proto(&transform, src_size, recipe.size).map_err(|e| {
        tracing::warn!(err = ?e, "FAILED_TO_MAKE_TRANSFORMER");
        match e {
            blobservices_core::transformers::TransformCreationError::NotImplemented => {
                StatusCode::NOT_IMPLEMENTED
            }
            _ => StatusCode::BAD_REQUEST,
        }
        .into_response()
    })?;
    let is_reversible = base.reversed().is_ok();

    let blob_id = insert_new_blob(tx, recipe.size, recipe.hashes).await?;

    let (param_tag, params) = transform.try_into().unwrap();
    let param_tag: i32 = param_tag.into();

    sqlx::query!(
        "INSERT INTO blob_transforms (
            id,
            src_blob_id,
            dst_blob_id,
            transform_type,
            is_reversible,
            shared_parameter_id,
            parameters
        ) VALUES(gen_random_uuid(), $1, $2, $3, $4, NULL, $5)",
        parent_id,
        blob_id,
        param_tag,
        is_reversible,
        params
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_INSERT_TRANSFORM");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    Ok(blob_id)
}
