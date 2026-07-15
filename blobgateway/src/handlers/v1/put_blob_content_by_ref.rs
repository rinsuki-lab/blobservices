use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use blobservices_core::proto::{self, storage::UploadBlobResponse};
use http_body_util::BodyExt;
use hyper::StatusCode;
use prost::Message;

use crate::{NamespaceAndKey, state::AppState};

pub async fn put_blob_content_by_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
    body: Body,
) -> Result<(), Response> {
    let mut store_configs = state
        .config
        .stores
        .iter()
        .filter(|x| x.1.can_write)
        .collect::<Vec<_>>();
    store_configs.sort_by_key(|x| x.1.priority);

    let mut upload_result = None;

    for (store_id, store_config) in store_configs {
        if let Some(result) = upload_to_specific_store(&state, &nk, store_id, store_config).await {
            upload_result = Some((store_id, result));
            break;
        }
    }

    let Some((store_id, upload_result)) = upload_result else {
        return Err(StatusCode::SERVICE_UNAVAILABLE.into_response());
    };

    let mut req = state.config.manager.url.clone();
    req.path_segments_mut()
        .unwrap()
        .push("v1")
        .push("refs")
        .push(&nk.namespace)
        .push(&nk.key);
    let res = state
        .client
        .put(req)
        .header("Content-Type", "application/protobuf")
        .body(
            proto::manager::PutBlobRefRequest {
                content: Some(
                    proto::manager::put_blob_ref_request::Content::UnsafeNewBlob(
                        proto::manager::PutBlobRefWithNewBlobInfo {
                            size: upload_result.size,
                            hashes: upload_result.hashes,
                            storage: store_id.clone(),
                            address: upload_result.address,
                        },
                    ),
                ),
            }
            .encode_to_vec(),
        )
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(err=?e, "FAILED_TO_REGISTER_BLOB_HTTP");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    if !res.status().is_success() {
        tracing::warn!(
            status = res.status().as_u16(),
            "FAILED_TO_REGISTER_BLOB_STATUS"
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    Ok(())
}

async fn upload_to_specific_store(
    state: &AppState,
    nk: &NamespaceAndKey,
    store_id: &str,
    store_config: &crate::config::StoreServerConfig,
) -> Option<proto::storage::UploadBlobResponse> {
    let mut req = store_config.url.clone();
    req.path_segments_mut().unwrap().push("v1").push("simple");
    req.query_pairs_mut()
        .append_pair("name_hint", &format!("{}/{}", nk.namespace, nk.key));

    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .version(hyper::Version::HTTP_11)
        .uri(req.as_str())
        .header(hyper::header::EXPECT, "100-continue")
        .header(hyper::header::ACCEPT, "application/protobuf")
        .body(reqwest::Body::wrap("hello".to_string())); // TODO: 100 が来るまで待つ
    let mut req = match req {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!(err=?e, "FAILED_TO_BUILD_HYPER_REQ");
            return None;
        }
    };

    hyper::ext::on_informational(&mut req, move |res| {
        println!("{:?}", res);
    });

    let res = state.hyper_client.request(req).await;
    let res = match res {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(err=?e, storage=store_id, "FAILED_TO_UPLOAD_SIMPLE");
            return None;
        }
    };
    if !res.status().is_success() {
        tracing::warn!(
            status = res.status().as_u16(),
            storage = store_id,
            "FAILED_TO_UPLOAD_SIMPLE_STATUS"
        );
        return None;
    }
    let res = res.into_body().collect().await;
    let res = match res {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(err=?e, storage=store_id, "FAILED_TO_RECV_SIMPLE_RES");
            return None;
        }
    };
    let res = res.to_bytes();
    let res = proto::storage::UploadBlobResponse::decode(res);
    let res = match res {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(err=?e, storage=store_id, "FAILED_TO_DECODE_SIMPLE_RES");
            return None;
        }
    };

    Some(res)
}
