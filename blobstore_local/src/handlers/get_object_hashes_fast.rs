use std::io::ErrorKind;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use tokio::{fs::File, io::AsyncSeekExt};

use crate::{provider::LocalStoreProvider, utils};

pub async fn get_object_hashes_fast(
    state: &LocalStoreProvider,
    address: String,
) -> Result<proto::storage::GetHashesResponse, Response> {
    let path = utils::sanitize_path(&state.done_dir, &address);

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

    Ok(proto::storage::GetHashesResponse {
        size,
        ..Default::default()
    })
}
