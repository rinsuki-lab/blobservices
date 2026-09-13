use axum::{
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use blobservices_core::proto::transform::transform_parameters::Transform;

pub fn get_merged_transform(
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
