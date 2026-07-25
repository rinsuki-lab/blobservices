use std::{sync::Arc, time::Duration};

use blobservices_core::utils::load_from_env_or_file_or_panic;
use tokio::{fs::File, io::AsyncReadExt};

use crate::config::Config;

pub struct AppStateInner {
    pub hyper_client: hyper_util::client::legacy::Client<
        hyper_tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        reqwest::Body,
    >,
    pub client: reqwest::Client,
    pub config: Config,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn new() -> AppState {
        let https_connector = hyper_tls::HttpsConnector::new();
        let hyper_client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .pool_timer(hyper_util::rt::tokio::TokioTimer::new())
                .pool_idle_timeout(Duration::from_secs(30))
                .build(https_connector);
        let client = reqwest::ClientBuilder::new()
            .user_agent("blobgateway/dev") // TODO: リリース時はこのバージョンをちゃんと埋めるようにする
            .build()
            .expect("Failed to build HTTP client");
        let config = load_from_env_or_file_or_panic("BLOBGATEWAY_CONFIG");
        let config: Config =
            serde_json::from_str(&config).expect("failed to parse blobgateway config");
        AppState::new(AppStateInner {
            hyper_client,
            client,
            config,
        })
    }
}
