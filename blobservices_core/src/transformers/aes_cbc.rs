use crate::{proto::transform::TransformAesCbcParameters, transformers::TransformCreationError};

pub struct AesCbc {}

pub struct DecAesCbc {}
pub struct EncAesCbc {}

impl AesCbc {
    fn new_shared(
        config: &TransformAesCbcParameters,
        src_size: u64,
        dst_size: u64,
    ) -> Result<AesCbc, TransformCreationError> {
        let key = config.key();
        if key.len() != 16 && key.len() != 24 && key.len() != 32 {
            Err(TransformCreationError::WrongParameter {
                key: "key",
                reason: "length should be 16 or 24 or 32",
            })?
        };

        let iv = config.iv();
        if iv.is_empty() {
            if !config.iv_prepended() {
                Err(TransformCreationError::WrongParameter {
                    key: "iv",
                    reason: "required if iv_prepended is false",
                })?
            }
            // will be denied if user tried to do reverse
        } else if iv.len() != 16 {
            Err(TransformCreationError::WrongParameter {
                key: "iv",
                reason: "length should be 16",
            })?
        }

        let Some(padding) = config.padding.as_ref().and_then(|p| p.padding.as_ref()) else {
            Err(TransformCreationError::WrongParameter {
                key: "padding",
                reason: "required",
            })?
        };

        if !src_size.is_multiple_of(16) {
            Err(TransformCreationError::WrongSize {
                reason: "src size should be multiple of 16 if there are no padding",
            })?
        }
        let Some(src_size_without_iv) =
            src_size.checked_sub(if config.iv_prepended() { 16 } else { 0 })
        else {
            Err(TransformCreationError::WrongSize {
                reason: "dst too small",
            })?
        };
        if dst_size > src_size_without_iv {
            Err(TransformCreationError::WrongSize {
                reason: "dst too big",
            })?;
        }
        if let Some(dst_min_size) = src_size_without_iv.checked_sub(16)
            && dst_size < dst_min_size
        {
            Err(TransformCreationError::WrongSize {
                reason: "dst too small",
            })?;
        }

        match padding {
            crate::proto::transform::aes_cbc_padding::Padding::Passthrough(_) => {
                if dst_size != src_size_without_iv {
                    Err(TransformCreationError::WrongSize {
                        reason: "dst size should be same as src size (minus prepended iv if exists) if there are no padding",
                    })?
                }
            }
            crate::proto::transform::aes_cbc_padding::Padding::Iso10126(bytes) => {
                if bytes.is_empty() {
                    if dst_size == src_size_without_iv {
                        Err(TransformCreationError::WrongSize {
                            reason: "dst size should smaller than src size (minus prepended iv if exists) for current padding type",
                        })?
                    }
                } else {
                    let expected_size = src_size_without_iv - dst_size;

                    if bytes.len() != expected_size as usize {
                        Err(TransformCreationError::WrongParameter {
                            key: "padding.iso10126",
                            reason: "wrong padding size",
                        })?
                    }

                    if *bytes.last().unwrap() != bytes.len() as u8 {
                        Err(TransformCreationError::WrongParameter {
                            key: "padding.iso10126",
                            reason: "padding mismatched with size",
                        })?
                    }
                }
            }
            crate::proto::transform::aes_cbc_padding::Padding::Pkcs7(_) => {
                if dst_size == src_size_without_iv {
                    Err(TransformCreationError::WrongSize {
                        reason: "dst size should smaller than src size (minus prepended iv if exists) for current padding type",
                    })?
                }
            }
        }

        Ok(AesCbc {}) // todo
    }

    pub fn new_straight(
        config: &TransformAesCbcParameters,
        src_size: u64,
        dst_size: u64,
    ) -> Result<DecAesCbc, TransformCreationError> {
        let shared = Self::new_shared(config, src_size, dst_size)?;
        Ok(DecAesCbc {}) // todo
    }

    pub fn new_reversed(
        config: &TransformAesCbcParameters,
        src_size: u64,
        dst_size: u64,
    ) -> Result<EncAesCbc, TransformCreationError> {
        let shared = Self::new_shared(config, src_size, dst_size)?;
        // TODO: deny if iv is empty (even iv_prepended is true)
        // TODO: deny if iso10126 padding but padding bytes are empty
        Ok(EncAesCbc {}) // todo
    }
}
