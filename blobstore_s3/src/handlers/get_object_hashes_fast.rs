use blobservices_core::proto;
use blobstore_core::Response;

use crate::provider::S3StoreProvider;

pub async fn get_object_hashes_fast(
    _state: &S3StoreProvider,
    _address: String,
) -> Result<proto::storage::GetHashesResponse, Response> {
    todo!()
}
