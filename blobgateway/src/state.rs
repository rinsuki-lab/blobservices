use std::sync::Arc;

use tokio::{fs::File, io::AsyncReadExt};

use crate::config::Config;

pub struct AppStateInner {
    pub client: reqwest::Client,
    pub config: Config,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn new() -> AppState {
        let client = reqwest::ClientBuilder::new()
            .user_agent("blobgateway/dev") // TODO: リリース時はこのバージョンをちゃんと埋めるようにする
            .build()
            .expect("Failed to build HTTP client");
        let config = std::env::var("BLOBGATEWAY_CONFIG");
        let config = match config {
            Ok(c) => c,
            Err(std::env::VarError::NotPresent) => {
                let config_file = std::env::var("BLOBGATEWAY_CONFIG_FILE")
                    .expect("BLOBGATEWAY_CONFIG_FILE or BLOBGATEWAY_CONFIG is required");
                let mut config = String::new();
                File::options()
                    .read(true)
                    .open(config_file)
                    .await
                    .expect("Failed to open BLOBGATEWAY_CONFIG_FILE")
                    .read_to_string(&mut config)
                    .await
                    .expect("Failed to read BLOBGATEWAY_CONFIG_FILE");
                config
            }
            _ => config.expect("BLOBGATEWAY_CONFIG is not valid"),
        };
        let config: Config =
            serde_json::from_str(&config).expect("failed to parse blobgateway config");
        AppState::new(AppStateInner { client, config })
    }
}
