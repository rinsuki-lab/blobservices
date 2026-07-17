use std::io::ErrorKind;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::{SuperHasher, proto};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{provider::LocalStoreProvider, utils};

pub async fn get_object_hashes(
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

    let mut hasher = SuperHasher::new();

    let mut buf = [0u8; 64 * 1024];
    loop {
        let result = file.read(&mut buf).await.map_err(|e| {
            tracing::error!(err=?e, address=address, "FAILED_TO_READ_FILE");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
        if result == 0 {
            break;
        }
        hasher.update(&buf[0..result]);
    }

    let (size, hashes) = hasher.finalize();

    Ok(proto::storage::GetHashesResponse { size, hashes })
}
