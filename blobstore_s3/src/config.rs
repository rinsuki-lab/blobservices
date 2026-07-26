use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    s3_base_url: String,
}
