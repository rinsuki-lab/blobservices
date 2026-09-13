mod aes_cbc;
mod error;
mod traits;

pub use aes_cbc::AesCbc;
pub use error::TransformCreationError;

use crate::proto::transform::transform_parameters::Transform;
pub use traits::TransformerBase;

pub fn make_transformer_from_proto(
    transform: &Transform,
    src_size: u64,
    dst_size: u64,
) -> Result<AesCbc, TransformCreationError> {
    match transform {
        Transform::DecDeflate(_) => Err(TransformCreationError::NotImplemented)?,
        Transform::DecAesCbc(transform_aes_cbc_parameters) => {
            aes_cbc::AesCbc::new(transform_aes_cbc_parameters, src_size, dst_size)
        }
    }
}
