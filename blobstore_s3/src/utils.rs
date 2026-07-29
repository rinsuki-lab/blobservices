use percent_encoding::{AsciiSet, utf8_percent_encode};
use url::Url;

/// https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/reference_sigv-create-signed-request.html
const S3_ALLOWED_CHARS: AsciiSet = percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'/');

pub fn get_s3_url_with_key(base_url: &Url, key: &str) -> Url {
    let mut url = base_url.clone();
    let key = utf8_percent_encode(key, &S3_ALLOWED_CHARS);
    let key = key.to_string();

    url.set_path(&format!(
        "{}{}{}",
        url.path(),
        if url.path().ends_with("/") { "" } else { "/" },
        key
    ));
    url
}
