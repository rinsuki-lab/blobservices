use std::time::Duration;

use blobservices_core::{parsers::http_range::BytesRange, utils::load_from_env_or_file_or_panic};
use blobstore_core::{BlobProvider, Body, Response, provider::GetObjectSimpleResponse};

use crate::{config::Config, handlers, signer::SigV4Signer};

pub struct S3StoreProvider {
    pub hyper_client: hyper_util::client::legacy::Client<
        hyper_tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        reqwest::Body,
    >,
    pub client: reqwest::Client,
    pub config: Config,
    pub sigv4_signer: SigV4Signer,
}

impl S3StoreProvider {
    pub async fn new() -> S3StoreProvider {
        let https_connector = hyper_tls::HttpsConnector::new();
        let hyper_client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .pool_timer(hyper_util::rt::tokio::TokioTimer::new())
                .pool_idle_timeout(Duration::from_secs(30))
                .build(https_connector);
        let client = reqwest::ClientBuilder::new()
            .user_agent("blobstore_s3/dev") // TODO: リリース時はこのバージョンをちゃんと埋めるようにする
            .build()
            .expect("Failed to build HTTP client");
        let config = load_from_env_or_file_or_panic("BLOBSTORE_S3_CONFIG");
        let config: Config =
            serde_json::from_str(&config).expect("failed to parse blobstore_s3 config");
        let sigv4_signer = SigV4Signer::new(config.s3_region.clone());

        S3StoreProvider {
            hyper_client,
            client,
            config,
            sigv4_signer,
        }
    }
}

impl BlobProvider for S3StoreProvider {
    fn env_prefix() -> &'static str {
        "BLOBSTORE_S3"
    }

    async fn put_object_simple(
        &self,
        body: Body,
    ) -> Result<blobservices_core::proto::storage::UploadBlobResponse, Response> {
        handlers::put_object_simple(self, body).await
    }

    async fn get_object_simple(
        &self,
        address: String,
        range: Option<BytesRange>,
    ) -> Result<GetObjectSimpleResponse, Response> {
        handlers::get_object_simple(self, address, range).await
    }

    async fn get_object_hashes_fast(
        &self,
        address: String,
    ) -> Result<blobservices_core::proto::storage::GetHashesResponse, Response> {
        handlers::get_object_hashes_fast(self, address).await
    }
}
