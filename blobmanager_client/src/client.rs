use blobservices_core::proto::manager::{GetBlobRefResponse, PutBlobRefRequest};
use prost::Message as _;
use reqwest::header;
use url::Url;

use crate::{Config, Error};

/// A reusable client that leaves application-specific error handling to the caller.
#[derive(Clone, Debug)]
pub struct Client {
    http_client: reqwest::Client,
    base_url: Url,
}

impl Client {
    /// Creates an HTTP client that requests protobuf responses by default.
    pub fn new(config: Config) -> Result<Self, reqwest::Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/protobuf"),
        );
        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(format!("{} (blobmanager_client)", config.user_agent_base))
            .build()?;

        Ok(Self {
            http_client,
            base_url: config.base_url,
        })
    }

    /// Fetches the blob information and locations for a reference.
    pub async fn get_blob_ref(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<GetBlobRefResponse, Error> {
        let request = self.http_client.get(self.blob_ref_url(namespace, key));
        let response = send_request(request).await?;

        let body = response.bytes().await.map_err(Error::Body)?;
        GetBlobRefResponse::decode(body).map_err(Error::Decode)
    }

    /// Creates or updates a reference using the supplied blob recipe.
    pub async fn put_blob_ref(
        &self,
        namespace: &str,
        key: &str,
        request: &PutBlobRefRequest,
    ) -> Result<(), Error> {
        let request = self
            .http_client
            .put(self.blob_ref_url(namespace, key))
            .header(header::CONTENT_TYPE, "application/protobuf")
            .body(request.encode_to_vec());
        send_request(request).await?;

        Ok(())
    }

    fn blob_ref_url(&self, namespace: &str, key: &str) -> Url {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .expect("blobmanager base URL must support path segments")
            .push("v1")
            .push("refs")
            .push(namespace)
            .push(key);
        url
    }
}

async fn send_request(request: reqwest::RequestBuilder) -> Result<reqwest::Response, Error> {
    let response = request.send().await.map_err(Error::Request)?;
    if !response.status().is_success() {
        return Err(Error::Status(response.status()));
    }
    Ok(response)
}
