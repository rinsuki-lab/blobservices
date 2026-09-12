use url::Url;

/// Settings for the blobmanager HTTP client.
#[derive(Clone, Debug)]
pub struct Config {
    /// The base URL must support path segments, such as an HTTP(S) URL.
    pub base_url: Url,
    /// The caller's User-Agent, followed by ` (blobmanager_client)` in requests.
    pub user_agent_base: String,
}
