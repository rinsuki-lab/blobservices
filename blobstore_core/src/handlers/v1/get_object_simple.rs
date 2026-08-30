use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use blobservices_core::parsers::http_range::bytes_range_specifier;

use crate::{BlobProvider, state::AppState, utils::sanitize_address};

pub async fn get_object_simple<P: BlobProvider>(
    state: State<AppState<P>>,
    Path(address): Path<String>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    let address = sanitize_address(&address).ok_or_else(|| {
        tracing::warn!(address = address, "SANITIZE_ADDRESS_FAILED");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    let range = headers
        .get(header::RANGE)
        .and_then(|x| x.to_str().ok())
        .and_then(|x| bytes_range_specifier(x).ok())
        .map(|x| x.1);

    let res = state.provider.get_object_simple(address, range).await?;

    let hres = match res.content_range {
        Some(range) => Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, range.to_string()),
        None => Response::builder().status(StatusCode::OK),
    };

    hres.header(header::CONTENT_LENGTH, res.size)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(res.body)
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_BUILD_RESPONSE");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}
