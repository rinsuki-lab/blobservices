use crate::transformers::TransformCreationError;

pub trait TransformerBase {
    type Config;
    type StraightTransformer;
    type ReversedTransformer;

    fn new(
        config: &Self::Config,
        src_size: u64,
        dst_size: u64,
    ) -> Result<Self, TransformCreationError>
    where
        Self: std::marker::Sized;

    fn straight(&self) -> Self::StraightTransformer;
    fn reversed(&self) -> Result<Self::ReversedTransformer, TransformCreationError>;
}
