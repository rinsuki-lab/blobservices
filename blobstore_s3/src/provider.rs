use blobstore_core::{BlobProvider, Body, Response};

use crate::handlers;

pub struct S3StoreProvider {}

impl S3StoreProvider {
    pub async fn new() -> S3StoreProvider {
        S3StoreProvider {}
    }
}

impl BlobProvider for S3StoreProvider {
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
