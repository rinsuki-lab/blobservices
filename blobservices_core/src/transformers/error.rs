#[derive(Debug)]
pub enum TransformCreationError {
    WrongParameter {
        key: &'static str,
        reason: &'static str,
    },
    WrongSize {
        reason: &'static str,
    },
    NotReversible,
    NotImplemented,
}
