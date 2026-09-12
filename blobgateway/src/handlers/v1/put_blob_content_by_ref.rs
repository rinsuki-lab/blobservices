use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use blobservices_core::proto::{self};
use futures::StreamExt;
use http_body_util::BodyExt;
use hyper::{StatusCode, body::Body as _};
use prost::Message;
use tokio::sync::Mutex;

use crate::{NamespaceAndKey, state::AppState};

pub async fn put_blob_content_by_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
    body: Body,
) -> Result<(), Response> {
    let size = body.size_hint().exact();
    let body = Arc::new(Mutex::new(Some(body)));
    let mut store_configs = state
        .config
        .stores
        .iter()
        .filter(|x| x.1.can_write)
        .collect::<Vec<_>>();
    store_configs.sort_by_key(|x| x.1.priority);

    let mut upload_result = None;

    for (store_id, store_config) in store_configs {
        {
            let Some(_) = *body.lock().await else {
                // 一回 100 Continue が帰ってきた後にエラーになってしまった場合、body はもう読まれた後なのでできることはない
                // のでループを脱出する
                // (バッファリングして再送とかはしない)
                break;
            };
        }
        if let Some(result) =
            upload_to_specific_store(&state, &nk, store_id, store_config, &body, size).await
        {
            upload_result = Some((store_id, result));
            break;
        }
    }

    let Some((store_id, upload_result)) = upload_result else {
        return Err(StatusCode::SERVICE_UNAVAILABLE.into_response());
    };

    state
        .manager_client
        .put_blob_ref(
            &nk.namespace,
            &nk.key,
            &proto::manager::PutBlobRefRequest {
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
            },
        )
        .await
        .map_err(|e| {
            match e {
                blobmanager_client::Error::Status(status) => {
                    tracing::warn!(status = status.as_u16(), "FAILED_TO_REGISTER_BLOB_STATUS");
                }
                e => tracing::warn!(err=?e, "FAILED_TO_REGISTER_BLOB_HTTP"),
            }
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    Ok(())
}

async fn upload_to_specific_store(
    state: &AppState,
    nk: &NamespaceAndKey,
    store_id: &str,
    store_config: &crate::config::StoreServerConfig,
    body: &Arc<Mutex<Option<Body>>>,
    size: Option<u64>,
) -> Option<proto::storage::UploadBlobResponse> {
    let mut req = store_config.url.clone();
    req.path_segments_mut().unwrap().push("v1").push("simple");
    req.query_pairs_mut()
        .append_pair("name_hint", &format!("{}/{}", nk.namespace, nk.key));

    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let body = body.clone();
    // Expect: 100-continue を送り、帰ってくるまで body を消費したくないので、わざわざ reqwest ではなく hyper を使っている
    // reqwest には 100 Continue を待つ機能がまだない https://github.com/seanmonstar/reqwest/issues/2845
    let mut req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .version(hyper::Version::HTTP_11)
        .uri(req.as_str())
        .header(hyper::header::EXPECT, "100-continue")
        .header(hyper::header::ACCEPT, "application/protobuf");
    if let Some(size) = size {
        req = req.header(hyper::header::CONTENT_LENGTH, size);
    }
    let req = req.body(reqwest::Body::wrap_stream(
        futures::stream::once(async move {
            match rx.await {
                Ok(_) => {}
                Err(_) => return None,
            };

            let body = body.lock().await.take()?;
            Some(body.into_data_stream())
        })
        .filter_map(async |x| x)
        .flatten(),
    ));
    let mut req = match req {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!(err=?e, "FAILED_TO_BUILD_HYPER_REQ");
            return None;
        }
    };

    {
        let tx = tx.clone();
        hyper::ext::on_informational(&mut req, move |res| {
            if res.status() != StatusCode::CONTINUE {
                return;
            }
            let tx = tx.clone();
            tokio::spawn(async move {
                let tx = tx.lock().await.take();
                let Some(tx) = tx else { return };
                _ = tx.send(());
            });
        });
    }

    let res = state.hyper_client.request(req).await;
    drop(tx.lock().await.take()); // 必要あるのかわからないが、一応ここで tx を捨てておく (100 Continue 来ずに失敗した時用)
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
