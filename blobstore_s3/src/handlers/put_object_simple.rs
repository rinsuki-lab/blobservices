use std::sync::Arc;

use blobservices_core::{
    SuperHasher,
    proto::{self, storage::UploadBlobResponse},
};
use blobstore_core::{Body, IntoResponse, Response};
use futures::StreamExt as _;
use http_body_util::BodyExt as _;
use hyper::{StatusCode, body::Body as _};
use tokio::sync::Mutex;

use crate::{provider::S3StoreProvider, utils::get_s3_url_with_key};

pub async fn put_object_simple(
    state: &S3StoreProvider,
    body: Body,
) -> Result<UploadBlobResponse, Response> {
    let size = body.size_hint().exact().ok_or_else(|| {
        tracing::warn!("SIZE_HINT_IS_NOT_EXACT");
        StatusCode::BAD_REQUEST.into_response()
    })?;

    let body = Arc::new(Mutex::new(Some(body)));
    let id = uuid::Uuid::now_v7();
    let id = id.to_string();
    // one folder per 3~4 days
    let address = format!("{}/{}/{}.bin", &id[0..3], &id[3..5], id);

    let url = get_s3_url_with_key(&state.config.s3_base_url, &address);

    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let hasher = Arc::new(Mutex::new(SuperHasher::new()));
    let hasher_for_result = hasher.clone();

    let mut req = hyper::Request::builder()
        .method(hyper::Method::PUT)
        .uri(url.as_str())
        .header("Expect", "100-continue")
        .header("If-None-Match", "*")
        .header("Content-Length", size.to_string())
        .body(reqwest::Body::wrap_stream(
            futures::stream::once(async move {
                match rx.await {
                    Ok(_) => {}
                    Err(_) => return None,
                };

                let body = body.lock().await.take()?;
                Some(body.into_data_stream())
            })
            .filter_map(async |x| x)
            .flatten()
            .then(move |x| {
                let hasher = hasher.clone();
                async move {
                    if let Ok(x) = &x {
                        hasher.lock().await.update(x);
                    }
                    x
                }
            }),
        ))
        .unwrap();
    state.sigv4_signer.sign(&mut req);

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
            tracing::warn!(err=?e, "FAILED_TO_UPLOAD_S3");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    let headers = res.headers();
    let status = res.status();

    if !status.is_success() {
        let res = res.into_body().collect().await;
        let res = match res {
            Ok(r) => r.to_bytes(),
            Err(e) => {
                tracing::warn!(err=?e, status=status.as_u16(), "FAILED_TO_RECV_S3_UPLOAD_RES");
                return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };
        let content = String::from_utf8_lossy(&res);
        tracing::warn!(
            status = status.as_u16(),
            content = content.as_ref(),
            "FAILED_TO_UPLOAD_S3_STATUS"
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    let hasher = Arc::into_inner(hasher_for_result).unwrap();
    let hasher = hasher.into_inner();
    let (size, hashes) = hasher.finalize();

    let upstream_etag = headers
        .get("ETag")
        .and_then(|x| x.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("S3_DIDNT_SEND_ETAG_HEADER");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    let expected_etag = format!("\"{}\"", hex::encode(hashes.md5()));
    if upstream_etag != expected_etag {
        tracing::error!(
            upstream = upstream_etag,
            expected = expected_etag,
            "S3_ETAG_MISMATCH"
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    Ok(proto::storage::UploadBlobResponse {
        address,
        hashes,
        size,
    })
}
