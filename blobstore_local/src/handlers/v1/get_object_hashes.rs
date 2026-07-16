use std::{
    io::ErrorKind,
    path::Component,
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::{SuperHasher, extractors::ResponseFormat, proto};
use tokio::{fs::File, io::AsyncReadExt};

use crate::state::AppState;

pub async fn get_object_hashes(
    State(state): State<AppState>,
    Path(address): Path<String>,
    res: ResponseFormat,
) -> Result<Response, Response> {
    let mut path = state.done_dir.clone();

    let address = std::path::Path::new(&address);
    for component in address.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(p) => {
                path.push(p);
            }
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {}
        }
    }

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
            tracing::error!(err=?e, address=%address.to_string_lossy(), "FAILED_TO_READ_FILE");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
        if result == 0 {
            break;
        }
        hasher.update(&buf[0..result]);
    }

    let (size, hashes) = hasher.finalize();

    Ok(res.message_to_response(proto::storage::GetHashesResponse { size, hashes }))
}
