use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::{
    proto::{
        self,
        transform::{TransformShared, transform_parameters::Transform},
    },
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

    let shared_transform = if let Some(shared) = recipe.transform.shared {
        Some(get_or_insert_shared_transform(tx, shared).await?)
    } else {
        None
    };

    let Some(transform) = recipe.transform.params.transform else {
        Err(StatusCode::BAD_REQUEST.into_response())?
    };
    let (param_tag, params) = transform.clone().try_into().unwrap();

    let merged = get_merged_transform(shared_transform.as_ref().map(|x| x.1.clone()), transform)?;

    let src_size = get_blob_size(tx, &parent_id).await?;

    let base = make_transformer_from_proto(&merged, src_size, recipe.size).map_err(|e| {
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
        ) VALUES(gen_random_uuid(), $1, $2, $3, $4, $5, $6)",
        parent_id,
        blob_id,
        param_tag,
        is_reversible,
        shared_transform.map(|x| x.0),
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

fn get_merged_transform(
    shared: Option<Transform>,
    current: Transform,
) -> Result<Transform, Response> {
    let Some(shared) = shared else {
        return Ok(current);
    };

    let (shared_tag, shared_json) = shared.try_into().unwrap();
    let (current_tag, mut current_json) = current.try_into().unwrap();

    if shared_tag != current_tag {
        Err(StatusCode::BAD_REQUEST.into_response())?;
    }

    let serde_json::Value::Object(shared_json) = shared_json else {
        panic!("shouldnt happen");
    };
    let current_json_obj = current_json.as_object_mut().unwrap();

    for (k, v) in shared_json {
        if current_json_obj.contains_key(&k) {
            continue;
        }
        current_json_obj.insert(k, v);
    }

    (current_tag, current_json).try_into().map_err(|e| {
        tracing::warn!(err=?e, "FAILED_TO_MERGE_TRANSFORM");
        StatusCode::BAD_REQUEST.into_response()
    })
}

async fn get_or_insert_shared_transform(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    transform: TransformShared,
) -> Result<(Uuid, Transform), Response> {
    if transform.namespace.is_empty() || transform.slug.is_empty() {
        Err(StatusCode::BAD_REQUEST.into_response())?
    }

    let Some(params) = transform.params.transform else {
        Err(StatusCode::BAD_REQUEST.into_response())?
    };

    let (tag, json) = params.try_into().unwrap();

    let row = sqlx::query!(
        "SELECT blob_transform_shared_parameters.*
        FROM blob_transform_shared_parameter_alias
        INNER JOIN blob_transform_shared_parameters ON blob_transform_shared_parameters.id = parameter_id
        WHERE namespace = $1 AND slug = $2",
        transform.namespace,
        transform.slug
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_QUERY_SHARED_PARAMETER_BEFORE_CREATE");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let id = if let Some(res) = row {
        if res.transform_type != (tag as i32) || res.parameters != json {
            tracing::warn!("ALIAS_CONTENT_IS_NOT_SAME");
            // probably update the alias in later, but currently it will treat as a conflict
            Err(StatusCode::CONFLICT.into_response())?
        }

        res.id
    } else {
        // TODO: 同時に同じnamespace/slugで入れようとするとパラメータが同じでもコンフリクトしてしまう問題をなんとかする
        let res = sqlx::query!(
            "INSERT INTO blob_transform_shared_parameters (id, transform_type, parameters) VALUES (gen_random_uuid(), $1, $2) RETURNING id",
            tag as i32,
            json
        ).fetch_one(&mut **tx).await.map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_INSERT_TRANSFORM_SHARED_PARAMS");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

        sqlx::query!("INSERT INTO blob_transform_shared_parameter_alias (id, namespace, slug, parameter_id) VALUES (gen_random_uuid(), $1, $2, $3)", transform.namespace, transform.slug, res.id).execute(&mut **tx).await.map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_INSERT_TRANSFORM_SHARED_PARAM_ALIAS");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

        res.id
    };

    Ok((id, (tag, json).try_into().unwrap()))
}
