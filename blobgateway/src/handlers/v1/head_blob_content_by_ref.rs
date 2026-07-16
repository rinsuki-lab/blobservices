use axum::{
    extract::{Path, State},
    response::{IntoResponse as _, Response},
};
use blobservices_core::proto;
use hyper::StatusCode;
use prost::Message as _;

use crate::{NamespaceAndKey, state::AppState};

pub async fn head_blob_content_by_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
) -> Result<Response, Response> {
    let mut res = state.config.manager.url.clone();
    res.path_segments_mut()
        .unwrap()
        .push("v1")
        .push("refs")
        .push(&nk.namespace)
        .push(&nk.key);
    let res = state
        .client
        .get(res)
        .header("Accept", "application/protobuf")
        .send()
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO_HEADER");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    if !res.status().is_success() {
        return Err(StatusCode::from_u16(res.status().as_u16())
            .unwrap()
            .into_response());
    }

    let res = res.bytes().await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO_BODY");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let res = proto::manager::GetBlobRefResponse::decode(res).map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO_DECODE");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let etag = res
        .blob
        .hashes
        .md5
        .map(|x| format!("\"{}\"", hex::encode(x)));

    let mut res = Response::builder().header("Content-Length", res.blob.size);
    if let Some(etag) = etag {
        res = res.header("ETag", etag);
    }

    Ok(res
        .body("".into())
        .expect("should not fail to build empty body"))
}
