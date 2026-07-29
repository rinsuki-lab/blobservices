use blobstore_core::{Body, IntoResponse as _, Response};

use crate::{provider::S3StoreProvider, utils::get_s3_url_with_key};
use blobstore_core::StatusCode;

pub async fn get_object_simple(
    state: &S3StoreProvider,
    address: String,
) -> Result<(u64, Body), Response> {
    // TODO: use cdn_url for GET (if exists)
    let url = get_s3_url_with_key(&state.config.s3_base_url, &address);

    let mut req = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri(url.as_str())
        .body(reqwest::Body::default())
        .unwrap();
    state.sigv4_signer.sign(&mut req);

    let res = state
        .client
        .execute(req.try_into().unwrap())
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_REQUEST_S3");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    if !res.status().is_success() {
        let status = res.status();
        if status == 404 {
            tracing::info!(address = address, "S3_NOT_FOUND");
            return Err(StatusCode::NOT_FOUND.into_response());
        }
        let content = res.text().await;
        tracing::warn!(
            address = address,
            status = status.as_u16(),
            content = ?content,
            "S3_FAIL_STATUS_CODE"
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    let headers = res.headers();

    let size = headers
        .get("Content-Length")
        .ok_or_else(|| {
            tracing::error!(
                address = address,
                header = "Content-Length",
                "S3_CRITICAL_HEADER_MISSING"
            );
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?
        .to_str()
        .map_err(|e| {
            tracing::error!(err=?e, address=address, header="Content-Length", "S3_CRITICAL_HEADER_INVALID_STR");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?
        .parse::<u64>()
        .map_err(|e| {
            tracing::error!(err=?e, address=address, header="Content-Length", "S3_CRITICAL_HEADER_PARSE_FAILED");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    Ok((size, Body::from_stream(res.bytes_stream())))
}
