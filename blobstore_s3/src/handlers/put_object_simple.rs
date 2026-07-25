use blobservices_core::proto::storage::UploadBlobResponse;
use blobstore_core::{Body, Response};

use crate::provider::S3StoreProvider;

pub async fn put_object_simple(
    _state: &S3StoreProvider,
    _body: Body,
) -> Result<UploadBlobResponse, Response> {
    todo!()
}
