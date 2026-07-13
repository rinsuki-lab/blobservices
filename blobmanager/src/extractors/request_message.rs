use axum::{
    Json,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{StatusCode, header},
    response::{IntoResponse as _, Response},
};
use prost::Message;
use serde::de::DeserializeOwned;

/// An encoded protobuf message extracted from either JSON or protobuf request
/// bodies.
pub struct RequestMessage<M>(pub M);

impl<M> RequestMessage<M> {
    pub fn into_inner(self) -> M {
        self.0
    }
}

impl<S, M> FromRequest<S> for RequestMessage<M>
where
    S: Send + Sync,
    M: Message + Default + DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let is_protobuf = req
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|content_type| content_type == "application/protobuf");

        if is_protobuf {
            let bytes = Bytes::from_request(req, state)
                .await
                .map_err(|rejection| rejection.into_response())?;
            let message = M::decode(bytes).map_err(|error| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("failed to decode protobuf request: {error}"),
                )
                    .into_response()
            })?;
            Ok(Self(message))
        } else {
            Json::<M>::from_request(req, state)
                .await
                .map(|Json(message)| Self(message))
                .map_err(|rejection| rejection.into_response())
        }
    }
}
