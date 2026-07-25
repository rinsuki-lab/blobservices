use blobstore_core::{Body, Response};

use crate::provider::S3StoreProvider;

pub async fn get_object_simple(
    _state: &S3StoreProvider,
    _address: String,
) -> Result<(u64, Body), Response> {
    todo!()
}
