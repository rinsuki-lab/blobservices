use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use blobservices_core::proto;
use hyper::header;
use prost::Message as _;
use reqwest::StatusCode;

use crate::{NamespaceAndKey, state::AppState};

async fn get_current_blob_info_by_ref(
    state: &AppState,
    nk: &NamespaceAndKey,
) -> Result<proto::manager::GetBlobRefResponse, Response> {
    let mut res = state.config.manager.url.clone();
    res.path_segments_mut()
        .unwrap()
        .push("v1")
        .push("refs")
        .push(&nk.namespace)
        .push(&nk.key);
    let res = state
        .client
        .get(res)
        .header("Accept", "application/protobuf")
        .send()
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO_HEADER");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    if !res.status().is_success() {
        return Err(StatusCode::from_u16(res.status().as_u16())
            .unwrap()
            .into_response());
    }

    let res = res.bytes().await.map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO_BODY");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    proto::manager::GetBlobRefResponse::decode(res).map_err(|e| {
        tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO_DECODE");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })
}

fn build_response_from_blob_info(
    info: &proto::manager::GetBlobRefResponse,
) -> axum::http::response::Builder {
    let mut res = Response::builder().header(header::CONTENT_LENGTH, info.blob.size);

    let etag = info
        .blob
        .hashes
        .md5
        .as_ref()
        .map(|x| format!("\"{}\"", hex::encode(x)));

    if let Some(etag) = etag {
        res = res.header("ETag", etag);
    }

    res
}

pub async fn head_blob_content_by_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
) -> Result<Response, Response> {
    let info = get_current_blob_info_by_ref(&state, &nk).await?;
    let res = build_response_from_blob_info(&info);
    Ok(res
        .body("".into())
        .expect("should not fail to build empty body"))
}

pub async fn get_blob_content_by_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
) -> Result<Response, Response> {
    let info = get_current_blob_info_by_ref(&state, &nk).await?;
    let res = build_response_from_blob_info(&info);

    if info.blob.size == 0 {
        // 何もないならそのまま返してしまえばいいじゃない
        return Ok(res
            .body("".into())
            .expect("should not fail to build empty body"));
    }

    let mut storage_res = None;
    let locations_and_configs = {
        let mut lac: Vec<(
            &proto::manager::BlobLocation,
            &crate::config::StoreServerConfig,
        )> = info
            .locations
            .iter()
            .filter_map(|location| {
                let location_config = &state.config.stores.get(&location.storage)?;
                if !location_config.can_read {
                    return None;
                }
                Some((location, *location_config))
            })
            .collect::<Vec<_>>();
        lac.sort_by_key(|x| x.1.priority);
        lac
    };
    for (location, location_config) in locations_and_configs {
        // TODO: range request に対応したい
        let mut res = location_config.url.clone();
        res.path_segments_mut()
            .unwrap()
            .push("v1")
            .push("simple")
            .push(&location.address);
        let res = state.client.get(res).send().await;
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(err=?e, storage=location.storage, "FAILED_TO_GET_BLOB_FROM_STORAGE_RES");
                continue;
            }
        };
        if !res.status().is_success() {
            tracing::warn!(
                status = res.status().as_u16(),
                storage = location.storage,
                "FAILED_TO_GET_BLOB_FROM_STORAGE_HTTPERR"
            );
            continue;
        }
        storage_res = Some(res);
        break;
    }
    let Some(storage_res) = storage_res else {
        let blob_id = uuid::Uuid::from_slice(&info.blob.id).unwrap();
        tracing::info!(blob_id=%blob_id, "FAILED_TO_GET_BLOB_NO_READABLE_PROVIDER");
        return Err(StatusCode::SERVICE_UNAVAILABLE.into_response());
    };

    let res = res
        .body(Body::from_stream(storage_res.bytes_stream()))
        .expect("FAILED_TO_BUILD_RES");

    Ok(res)
}
