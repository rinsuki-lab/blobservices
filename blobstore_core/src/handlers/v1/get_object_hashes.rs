use axum::{
    extract::{Path, State},
    response::Response,
};
use blobservices_core::extractors::ResponseFormat;

use crate::{BlobProvider, state::AppState};

pub async fn get_object_hashes<P: BlobProvider>(
    state: State<AppState<P>>,
    res: ResponseFormat,
    Path(address): Path<String>,
) -> Result<Response, Response> {
    state
        .provider
        .get_object_hashes(address)
        .await
        .map(|x| res.message_to_response(x))
}
