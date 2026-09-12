use std::{sync::Arc, time::Duration};

use blobservices_core::utils::load_from_env_or_file_or_panic;

use crate::config::Config;

pub struct AppStateInner {
    pub hyper_client: hyper_util::client::legacy::Client<
        hyper_tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        reqwest::Body,
    >,
    pub client: reqwest::Client,
    pub manager_client: blobmanager_client::Client,
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
        let user_agent_base = "blobgateway/dev"; // TODO: リリース時はこのバージョンをちゃんと埋めるようにする
        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent_base)
            .build()
            .expect("Failed to build HTTP client");
        let config = load_from_env_or_file_or_panic("BLOBGATEWAY_CONFIG");
        let config: Config =
            serde_json::from_str(&config).expect("failed to parse blobgateway config");
        let manager_client = blobmanager_client::Client::new(blobmanager_client::Config {
            base_url: config.manager.url.clone(),
            user_agent_base: user_agent_base.to_owned(),
        })
        .expect("Failed to build blobmanager HTTP client");
        AppState::new(AppStateInner {
            hyper_client,
            client,
            manager_client,
            config,
        })
    }
}
