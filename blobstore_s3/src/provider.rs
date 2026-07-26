use blobservices_core::utils::load_from_env_or_file_or_panic;
use blobstore_core::{BlobProvider, Body, Response};

use crate::{config::Config, handlers};

pub struct S3StoreProvider {
    pub client: reqwest::Client,
    pub config: Config,
}

impl S3StoreProvider {
    pub async fn new() -> S3StoreProvider {
        let client = reqwest::ClientBuilder::new()
            .user_agent("blobstore_s3/dev") // TODO: リリース時はこのバージョンをちゃんと埋めるようにする
            .build()
            .expect("Failed to build HTTP client");
        let config = load_from_env_or_file_or_panic("BLOBSTORE_S3_CONFIG");
        let config: Config =
            serde_json::from_str(&config).expect("failed to parse blobstore_s3 config");

        S3StoreProvider { client, config }
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
        // TODO: range header?
    ) -> Result<(u64, Body), Response> {
        handlers::get_object_simple(self, address).await
    }

    async fn get_object_hashes_fast(
        &self,
        address: String,
    ) -> Result<blobservices_core::proto::storage::GetHashesResponse, Response> {
        handlers::get_object_hashes_fast(self, address).await
    }
}
