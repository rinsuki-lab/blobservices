use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{BlobProvider, state::AppState};

pub async fn get_object_simple<P: BlobProvider>(
    state: State<AppState<P>>,
    Path(address): Path<String>,
) -> Result<Response, Response> {
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
