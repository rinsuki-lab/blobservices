use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{BlobProvider, state::AppState, utils::sanitize_address};

pub async fn get_object_simple<P: BlobProvider>(
    state: State<AppState<P>>,
    Path(address): Path<String>,
) -> Result<Response, Response> {
    let address = sanitize_address(&address).ok_or_else(|| {
        tracing::warn!(address = address, "SANITIZE_ADDRESS_FAILED");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    // TODO: support range request
    state
        .provider
        .get_object_simple(address)
        .await
        .and_then(|(length, body)| {
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Length", length)
                .body(body)
                .map_err(|e| {
                    tracing::error!(err=?e, "FAILED_TO_BUILD_RESPONSE");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                })
        })
}
