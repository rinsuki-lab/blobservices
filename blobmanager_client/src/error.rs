use std::{error, fmt};

use reqwest::StatusCode;

#[derive(Debug)]
pub enum Error {
    /// The request could not be sent or its response headers could not be read.
    Request(reqwest::Error),
    /// The manager returned a non-success HTTP status.
    Status(StatusCode),
    /// The response body could not be read.
    Body(reqwest::Error),
    /// The response body could not be decoded as protobuf.
    Decode(prost::DecodeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => write!(f, "failed to send blobmanager request: {error}"),
            Self::Status(status) => write!(f, "blobmanager returned HTTP {status}"),
            Self::Body(error) => write!(f, "failed to read blobmanager response: {error}"),
            Self::Decode(error) => write!(f, "failed to decode blobmanager response: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Request(error) | Self::Body(error) => Some(error),
            Self::Decode(error) => Some(error),
            Self::Status(_) => None,
        }
    }
}
