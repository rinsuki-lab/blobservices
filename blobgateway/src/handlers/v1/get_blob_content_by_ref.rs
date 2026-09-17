use std::collections::{HashMap, HashSet};

use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use blobservices_core::{parsers::http_content_range::ContentRange, proto};
use hyper::{HeaderMap, header};
use reqwest::StatusCode;

use crate::{
    NamespaceAndKey,
    config::{Config, StoreServerConfig},
    state::AppState,
};

struct ReadCandidate<'a> {
    location: &'a proto::manager::BlobLocation,
    config: &'a StoreServerConfig,
    range: Option<ContentRange>,
}

fn find_read_candidates<'a>(
    info: &'a proto::manager::GetBlobRefResponse,
    config: &'a Config,
) -> Vec<ReadCandidate<'a>> {
    let blobs: HashMap<_, _> = std::iter::once(&info.blob)
        .chain(&info.related_blobs)
        .map(|blob| (blob.id.as_slice(), blob))
        .collect();
    let mut pending = vec![(&info.blob, 0)];
    let mut visited = HashSet::new();
    let mut candidates = Vec::new();

    while let Some((blob, start)) = pending.pop() {
        if !visited.insert((blob.id.as_slice(), start)) {
            continue;
        }

        for location in info.locations.iter().filter(|l| l.blob_id == blob.id) {
            let Some(store) = config.stores.get(&location.storage) else {
                continue;
            };
            if store.can_read {
                candidates.push(ReadCandidate {
                    location,
                    config: store,
                    range: (blob.id != info.blob.id).then(|| ContentRange {
                        start,
                        end: start + info.blob.size - 1,
                        entire_size: blob.size,
                    }),
                });
            }
        }

        for operation in info
            .operations
            .iter()
            .filter(|op| op.dst_blob_id == blob.id)
        {
            let Some(proto::manager::blob_operation::Operation::Slice(slice)) =
                &operation.operation
            else {
                continue;
            };
            let dst_start = slice.dst_start();
            let size = slice.size.unwrap_or(blob.size);
            // This operation must cover the entire requested interval; joining pieces is separate.
            if start < dst_start || start + info.blob.size > dst_start + size {
                continue;
            }
            let source = blobs[operation.src_blob_id.as_slice()];
            pending.push((source, slice.src_start() + (start - dst_start)));
        }
    }

    candidates.sort_by_key(|candidate| candidate.config.priority);
    candidates
}

fn matches_content_range(headers: &HeaderMap, expected: &ContentRange) -> bool {
    headers
        .get(header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| ContentRange::parse(value).ok())
        .is_some_and(|(_, actual)| actual == *expected)
}

async fn get_current_blob_info_by_ref(
    state: &AppState,
    nk: &NamespaceAndKey,
) -> Result<proto::manager::GetBlobRefResponse, Response> {
    state
        .manager_client
        .get_blob_ref(&nk.namespace, &nk.key)
        .await
        .map_err(|e| match e {
            blobmanager_client::Error::Status(status) => status.into_response(),
            e => {
                tracing::error!(err=?e, "FAILED_TO_GET_REF_INFO");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
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
    for candidate in find_read_candidates(&info, &state.config) {
        let location = candidate.location;
        let mut url = candidate.config.url.clone();
        url.path_segments_mut()
            .unwrap()
            .push("v1")
            .push("simple")
            .push(&location.address);
        let mut request = state.client.get(url);
        if let Some(range) = &candidate.range {
            request = request.header(
                header::RANGE,
                format!("bytes={}-{}", range.start, range.end),
            );
        }
        let res = request.send().await;
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(err=?e, storage=location.storage, "FAILED_TO_GET_BLOB_FROM_STORAGE_RES");
                continue;
            }
        };
        if !res.status().is_success()
            || (candidate.range.is_some() && res.status() != StatusCode::PARTIAL_CONTENT)
        {
            tracing::warn!(
                status = res.status().as_u16(),
                storage = location.storage,
                "FAILED_TO_GET_BLOB_FROM_STORAGE_HTTPERR"
            );
            continue;
        }
        if let Some(expected) = &candidate.range
            && !matches_content_range(res.headers(), expected)
        {
            tracing::warn!(
                storage = location.storage,
                expected = %expected,
                actual = ?res.headers().get(header::CONTENT_RANGE),
                "FAILED_TO_GET_BLOB_FROM_STORAGE_CONTENT_RANGE"
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
