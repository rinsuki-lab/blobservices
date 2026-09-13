use std::marker::PhantomData;

use digest::{
    Key,
    array::ArraySize,
    common::{Iv, IvSizeUser, KeySizeUser},
    consts::{U16, U24, U32},
};

use crate::{
    proto::transform::TransformAesCbcParameters,
    transformers::{TransformCreationError, TransformerBase},
};

struct IvSizeProxy<Size: ArraySize>(PhantomData<Size>);
impl<S: ArraySize> IvSizeUser for IvSizeProxy<S> {
    type IvSize = S;
}

type AesCbcIv = Iv<IvSizeProxy<U16>>;

enum AesKey {
    Aes128(Key<dyn KeySizeUser<KeySize = U16>>),
    Aes192(Key<dyn KeySizeUser<KeySize = U24>>),
    Aes256(Key<dyn KeySizeUser<KeySize = U32>>),
}

pub struct AesCbc {
    key: AesKey,
    iv: Option<AesCbcIv>,
    padding: crate::proto::transform::aes_cbc_padding::Padding,

    src_size: u64,
    dst_size: u64,
}

pub struct DecAesCbc {}
pub struct EncAesCbc {}

impl TransformerBase for AesCbc {
    type Config = TransformAesCbcParameters;
    type StraightTransformer = DecAesCbc;
    type ReversedTransformer = EncAesCbc;

    fn new(
        config: &TransformAesCbcParameters,
        src_size: u64,
        dst_size: u64,
    ) -> Result<AesCbc, TransformCreationError> {
        let key = config.key();
        let key = match key.len() {
            16 => AesKey::Aes128(key.try_into().unwrap()),
            24 => AesKey::Aes192(key.try_into().unwrap()),
            32 => AesKey::Aes256(key.try_into().unwrap()),
            _ => Err(TransformCreationError::WrongParameter {
                key: "key",
                reason: "length should be 16 or 24 or 32",
            })?,
        };

        let iv = config.iv();
        let iv: Option<AesCbcIv> = if iv.is_empty() {
            if !config.iv_prepended() {
                Err(TransformCreationError::WrongParameter {
                    key: "iv",
                    reason: "required if iv_prepended is false",
                })?
            }
            None
        } else {
            Some(
                iv.try_into()
                    .map_err(|e| TransformCreationError::WrongParameter {
                        key: "iv",
                        reason: "length should be 16",
                    })?,
            )
        };

        let Some(padding) = config
            .padding
            .as_ref()
            .and_then(|p| p.padding.as_ref())
            .cloned()
        else {
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
                reason: "src too small",
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
            crate::proto::transform::aes_cbc_padding::Padding::Iso10126(ref bytes) => {
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

        Ok(AesCbc {
            key,
            iv,
            padding,
            src_size,
            dst_size,
        })
    }

    fn straight(&self) -> DecAesCbc {
        DecAesCbc {} // todo
    }

    fn reversed(&self) -> Result<EncAesCbc, TransformCreationError> {
        if self.iv.is_none() {
            Err(TransformCreationError::WrongParameter {
                key: "iv",
                reason: "iv is required when reversing",
            })?
        }
        if let crate::proto::transform::aes_cbc_padding::Padding::Iso10126(ref bytes) = self.padding
            && bytes.is_empty()
        {
            Err(TransformCreationError::WrongParameter {
                key: "padding.iso10126",
                reason: "bytes are required when reversing",
            })?
        }
        Ok(EncAesCbc {}) // todo
    }
}
