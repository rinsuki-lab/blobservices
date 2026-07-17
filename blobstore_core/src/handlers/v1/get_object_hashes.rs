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

use crate::{BlobProvider, state::AppState};

pub async fn get_object_hashes<P: BlobProvider>(
    state: State<AppState<P>>,
    res: ResponseFormat,
    Path(address): Path<String>,
    Query(params): Query<proto::storage::GetHashesQuery>,
) -> Result<Response, Response> {
    let result = if params.speed() == HashSpeed::Fast {
        state.provider.get_object_hashes_fast(address).await
    } else {
        let mut hasher = SuperHasher::new();

        let (size, body) = state.provider.get_object_simple(address).await?;
        let mut body = body.into_data_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| {
                tracing::error!(err=?e, "FAILED_TO_READ_BODY");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;
            hasher.update(&chunk);
        }

        let (readed_size, hashes) = hasher.finalize();

        if readed_size != size {
            tracing::error!(readed = readed_size, expected = size, "SIZE_DIDNT_MATCH");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        Ok(proto::storage::GetHashesResponse {
            size: readed_size,
            hashes,
        })
    };

    result.map(|x| res.message_to_response(x))
}
