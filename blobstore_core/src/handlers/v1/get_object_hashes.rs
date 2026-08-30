use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::{
    SuperHasher,
    extractors::ResponseFormat,
    proto::{self, storage::HashSpeed},
};
use futures::StreamExt as _;

use crate::{BlobProvider, state::AppState, utils::sanitize_address};

pub async fn get_object_hashes<P: BlobProvider>(
    state: State<AppState<P>>,
    res: ResponseFormat,
    Path(address): Path<String>,
    Query(params): Query<proto::storage::GetHashesQuery>,
) -> Result<Response, Response> {
    let address = sanitize_address(&address).ok_or_else(|| {
        tracing::warn!(address = address, "SANITIZE_ADDRESS_FAILED");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    let result = if params.speed() == HashSpeed::Fast {
        state.provider.get_object_hashes_fast(address).await
    } else {
        let mut hasher = SuperHasher::new();

        let res = state.provider.get_object_simple(address, None).await?;
        if let Some(range) = res.content_range
            && (range.start != 0
                || range.end != (range.entire_size - 1)
                || range.entire_size != res.size)
        {
            tracing::error!(
                size = res.size,
                content_range = range.to_string(),
                "UNEXPECTED_CONTENT_RANGE"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
        let mut body = res.body.into_data_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| {
                tracing::error!(err=?e, "FAILED_TO_READ_BODY");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;
            hasher.update(&chunk);
        }

        let (readed_size, hashes) = hasher.finalize();

        if readed_size != res.size {
            tracing::error!(
                readed = readed_size,
                expected = res.size,
                "SIZE_DIDNT_MATCH"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        Ok(proto::storage::GetHashesResponse {
            size: readed_size,
            hashes,
        })
    };

    result.map(|x| res.message_to_response(x))
}
