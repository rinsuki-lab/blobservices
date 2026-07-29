use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
pub struct Config {
    /// e.g. https://bucket-name.s3.amazonaws.com/
    pub s3_base_url: Url,
    /// e.g. us-east-1 (S3 へのリクエストの署名時に必要)
    pub s3_region: String,
    /// GET を CDN 経由で行いたい時に使う
    /// e.g. https://example.cloudfront.invalid/
    pub cdn_base_url: Option<Url>,
}
