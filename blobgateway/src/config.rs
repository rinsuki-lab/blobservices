use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct Config {
    pub manager: ManagerServerConfig,
    pub stores: HashMap<String, StoreServerConfig>,
}

#[derive(serde::Deserialize)]
pub struct ManagerServerConfig {
    pub url: url::Url,
}

#[derive(serde::Deserialize)]
pub struct StoreServerConfig {
    pub url: url::Url,
    pub priority: i32,
    pub can_read: bool,
    pub can_write: bool,
}
