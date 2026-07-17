use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blobservices_core::{
    SuperHasher,
    proto::{self, storage::UploadBlobResponse},
};
use futures_util::StreamExt as _;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

use crate::provider::LocalStoreProvider;

pub async fn put_object_simple(
    state: &LocalStoreProvider,
    body: Body,
) -> Result<UploadBlobResponse, Response> {
    let mut stream = body.into_data_stream();
    let id = uuid::Uuid::now_v7();
    let id = id.to_string();
    // one folder per 3~4 days
    let final_path = format!("{}/{}/{}.bin", &id[0..3], &id[3..5], &id);

    let mut wip_path = state.wip_dir.clone();
    wip_path.push(&final_path);

    let mut hasher = SuperHasher::new();

    fs::create_dir_all(wip_path.parent().unwrap())
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_CREATE_WIP_DIR");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(&wip_path)
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_CREATE_FILE");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_RECV");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

        hasher.update(&chunk);

        file.write_all(&chunk).await.map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_WRITE_CHUNK");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    }

    file.sync_data().await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_SYNC_FILE_DATA");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let (size, hashes) = hasher.finalize();

    let mut done_path = state.done_dir.clone();
    done_path.push(&final_path);
    fs::create_dir_all(done_path.parent().unwrap())
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_CREATE_DONE_DIR");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    tokio::fs::rename(wip_path, done_path).await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_RENAME");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    file.sync_all().await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_SYNC_FILE");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    Ok(proto::storage::UploadBlobResponse {
        address: final_path,
        size,
        hashes,
    })
}
