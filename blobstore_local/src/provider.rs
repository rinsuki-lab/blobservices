use std::{path::PathBuf, str::FromStr};

use axum::body::Body;

use crate::handlers;

pub struct LocalStoreProvider {
    pub wip_dir: PathBuf,
    pub done_dir: PathBuf,
}

impl LocalStoreProvider {
    pub async fn new() -> LocalStoreProvider {
        let root_dir = std::env::var("BLOBSTORE_LOCAL_ROOT_DIR")
            .expect("BLOBSTORE_LOCAL_ROOT_DIR is required");
        let root_dir_buf = PathBuf::from_str(&root_dir)
            .expect("BLOBSTORE_LOCAL_ROOT_DIR is not valid for PathBuf");
        let wip_dir = {
            let mut p = root_dir_buf.clone();
            p.push("wip");
            p
        };
        let done_dir = {
            let mut p = root_dir_buf;
            p.push("done");
            p
        };
        LocalStoreProvider { wip_dir, done_dir }
    }
}

impl blobstore_core::BlobProvider for LocalStoreProvider {
    fn env_prefix() -> &'static str {
        "BLOBSTORE_LOCAL"
    }

    async fn put_object_simple(
        &self,
        body: axum::body::Body,
    ) -> Result<blobservices_core::proto::storage::UploadBlobResponse, axum::response::Response>
    {
        handlers::put_object_simple(self, body).await
    }

    async fn get_object_simple(
        &self,
        address: String,
        // TODO: range header?
    ) -> Result<(u64, Body), axum::response::Response> {
        handlers::get_object_simple(self, address).await
    }

    async fn get_object_hashes_fast(
        &self,
        address: String,
    ) -> Result<blobservices_core::proto::storage::GetHashesResponse, axum::response::Response>
    {
        handlers::get_object_hashes_fast(self, address).await
    }
}
