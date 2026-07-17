use axum::{body::Body, response::Response};
use blobservices_core::proto;

use crate::state::AppState;

pub trait BlobProvider: Send + Sync + Sized {
    fn env_prefix() -> &'static str {
        "BLOBSTORE_GENERIC"
    }

    fn put_object_simple(
        &self,
        body: Body,
    ) -> impl Future<Output = Result<proto::storage::UploadBlobResponse, Response>> + Send;

    fn get_object_simple(
        &self,
        address: String,
        // TODO: range header?
    ) -> impl Future<Output = Result<(u64, Body), Response>> + Send;

    fn get_object_hashes(
        &self,
        address: String,
    ) -> impl Future<Output = Result<proto::storage::GetHashesResponse, Response>> + Send;

    fn add_additional_routes(
        &self,
        router: axum::Router<AppState<Self>>,
    ) -> axum::Router<AppState<Self>> {
        router
    }
}
