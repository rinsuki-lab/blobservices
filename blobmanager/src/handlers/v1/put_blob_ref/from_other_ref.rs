use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto::manager::PutBlobRefFromOtherRef;
use uuid::Uuid;

pub(super) async fn from_other_ref(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipe: PutBlobRefFromOtherRef,
) -> Result<Uuid, Response> {
    let expected = recipe
        .expected_blob_id
        .as_deref()
        .map(Uuid::from_slice)
        .transpose()
        .map_err(|e| {
            tracing::warn!(err=?e, namespace=%recipe.namespace, key=%recipe.key, "INVALID_EXPECTED_BLOB_ID");
            StatusCode::BAD_REQUEST.into_response()
        })?;

    // Resolve and check the source using the same read snapshot.
    let row = sqlx::query!(
        r#"
        SELECT revision.blob_id
        FROM blob_references AS reference
        INNER JOIN blob_reference_revisions AS revision
            ON revision.id = reference.current_revision_id
        WHERE reference.namespace = $1 AND reference.key = $2
        "#,
        recipe.namespace,
        recipe.key,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_QUERY_SOURCE_REF");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?
    .ok_or_else(|| {
        tracing::warn!(namespace=%recipe.namespace, key=%recipe.key, "SOURCE_REF_NOT_FOUND");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    if let Some(expected) = expected
        && expected != row.blob_id
    {
        tracing::warn!(namespace=%recipe.namespace, key=%recipe.key, %expected, actual=%row.blob_id, "SOURCE_REF_BLOB_ID_MISMATCH");
        return Err(StatusCode::CONFLICT.into_response());
    }
    Ok(row.blob_id)
}
