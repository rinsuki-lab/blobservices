use std::convert::Infallible;

use axum::{
    Json,
    extract::FromRequestParts,
    http::{header, request::Parts},
    response::{IntoResponse as _, Response},
};
use prost::Message;
use serde::Serialize;

pub enum ResponseFormat {
    Json,
    Protobuf,
}

impl ResponseFormat {
    pub fn message_to_response<M>(self, message: M) -> Response
    where
        M: Message + Serialize,
    {
        match self {
            Self::Json => Json(message).into_response(),
            Self::Protobuf => (
                [(header::CONTENT_TYPE, "application/protobuf")],
                message.encode_to_vec(),
            )
                .into_response(),
        }
    }
}

impl<S: Sync> FromRequestParts<S> for ResponseFormat {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(
            if parts
                .headers
                .get(header::ACCEPT)
                .is_some_and(|accept| accept == "application/protobuf")
            {
                Self::Protobuf
            } else {
                Self::Json
            },
        )
    }
}
