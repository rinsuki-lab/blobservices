use axum::{body::Body, extract::State, response::Response};
use blobservices_core::extractors::ResponseFormat;

use crate::{BlobProvider, state::AppState};

pub async fn put_object_simple<P: BlobProvider>(
    state: State<AppState<P>>,
    res: ResponseFormat,
    body: Body,
) -> Result<Response, Response> {
    state
        .provider
        .put_object_simple(body)
        .await
        .map(|x| res.message_to_response(x))
}
