use std::{io::ErrorKind, num::NonZeroU64};

use axum::{
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use blobservices_core::parsers::http_range::BytesRange;
use blobstore_core::provider::GetObjectSimpleResponse;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;

use crate::provider::LocalStoreProvider;

pub async fn get_object_simple(
    state: &LocalStoreProvider,
    address: String,
    range: Option<BytesRange>,
) -> Result<GetObjectSimpleResponse, Response> {
    let mut path = state.done_dir.clone();
    path.push(address);

    let mut file = File::options().read(true).open(path).await.map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            StatusCode::NOT_FOUND.into_response()
        } else {
            tracing::error!(err=?e, "FAILED_TO_OPEN_FILE");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    })?;

    let size = file.seek(std::io::SeekFrom::End(0)).await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_SEEK_END");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let size = match NonZeroU64::new(size) {
        Some(x) => x,
        None => {
            // its empty
            return Ok(GetObjectSimpleResponse {
                size,
                body: Body::empty(),
                content_range: None,
            });
        }
    };

    let range = match range {
        Some(x) => Some(x.normalize(size).ok_or_else(|| {
            Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{}", size))
                .body(Body::empty())
                .unwrap()
        })?),
        None => None,
    };

    let (start, size) = range
        .as_ref()
        .map(|x| (x.start, x.size()))
        .unwrap_or((0, size.into()));

    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_SEEK_START");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    Ok(GetObjectSimpleResponse {
        size,
        body: Body::from_stream(ReaderStream::new(file.take(size))),
        content_range: range,
    })
}
