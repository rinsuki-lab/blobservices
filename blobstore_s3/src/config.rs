use std::{env, fs, ops::Deref};

use serde::Deserialize;
use url::Url;

use crate::signer::CloudFrontSigningKey;

#[derive(Deserialize)]
pub struct Config {
    /// e.g. https://bucket-name.s3.amazonaws.com/
    pub s3_base_url: Url,
    /// e.g. us-east-1 (S3 へのリクエストの署名時に必要)
    pub s3_region: String,
    /// GET を CDN 経由で行いたい時に使う
    pub cdn: Option<CdnConfig>,
}

#[derive(Deserialize)]
pub struct CdnConfig {
    /// e.g. https://example.cloudfront.invalid/
    pub base_url: Url,
    /// RSA Private Key
    pub private_key: Option<ParsedSecretKey>,
}

pub struct ParsedSecretKey(CloudFrontSigningKey);

impl<'de> serde::Deserialize<'de> for ParsedSecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let config = SecretKeyConfig::deserialize(deserializer)?;
        Ok(ParsedSecretKey(CloudFrontSigningKey::new(config)))
    }
}

impl Deref for ParsedSecretKey {
    type Target = CloudFrontSigningKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize)]
pub struct SecretKeyConfig {
    pub key_id: String,
    pub source: SecretSource,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SecretSource {
    File(String),
    Env(String),
    Constant(String),
}

impl SecretSource {
    pub fn to_content(&self) -> String {
        match self {
            SecretSource::File(path) => fs::read_to_string(path)
                .map_err(|e| panic!("failed to read file from {}: {}", path, e))
                .unwrap(),
            SecretSource::Env(key) => env::var(key)
                .map_err(|e| {
                    panic!("failed to read {} env: {}", key, e);
                })
                .unwrap(),
            SecretSource::Constant(value) => value.clone(),
        }
    }
}
