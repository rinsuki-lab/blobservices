use std::env;

use chrono::{Datelike, Timelike};
use hmac::{KeyInit, Mac};
use hyper::{Request, header::HeaderValue};
use sha2::Digest;

const AWS_SERVICE: &'static str = "s3";

pub struct SigV4Signer {
    access_key_id: String,
    secret_access_key: String,
    region: String,
}

/// https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/reference_sigv-create-signed-request.html#create-canonical-request
fn make_canonical_request<T>(req: &Request<T>) -> (sha2::digest::Output<sha2::Sha256>, String) {
    let mut parts = String::new();
    // <HTTPMethod>
    parts.push_str(req.method().as_str());
    parts.push('\n');
    // <CanonicalURI>
    let uri = req.uri();
    {
        let path = uri.path();
        if path.is_empty() {
            parts.push('/');
        } else {
            parts.push_str(path);
        }
    }
    parts.push('\n');
    // <CanonicalQueryString>
    // TODO: implement
    parts.push('\n');
    // <CanonicalHeaders>
    let mut headers = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .filter(|(k, _)| {
            *k == "host" || *k == "content-type" || k.starts_with("x-amz-") || k.starts_with("if-")
        })
        .collect::<Vec<_>>();
    headers.sort_by_key(|x| x.0);
    for (k, v) in &headers {
        parts.push_str(k);
        parts.push(':');
        parts.push_str(v.to_str().unwrap());
        parts.push('\n');
    }
    parts.push('\n');
    // <SignedHeaders>
    let signed_headers = headers
        .iter()
        .map(|(k, _)| *k)
        .collect::<Vec<_>>()
        .join(";");
    parts.push_str(&signed_headers);
    // <HashedPayload>
    parts.push_str("\nUNSIGNED-PAYLOAD");
    tracing::trace!(parts = parts, "S3_MADE_CANONICAL_REQUEST");

    let parts = parts.as_bytes();

    (sha2::Sha256::digest(parts), signed_headers)
}

/// https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/reference_sigv-create-signed-request.html#create-string-to-sign
fn make_signature_string(
    current_datetime: &str,
    cred_scope: &str,
    canonical_request_hash: sha2::digest::Output<sha2::Sha256>,
) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        // Algorithm
        "AWS4-HMAC-SHA256",
        // RequestDateTime
        current_datetime,
        // CredentialScope
        cred_scope,
        // HashedCanonicalRequest
        hex::encode(canonical_request_hash)
    )
}

type HmacSha256 = hmac::Hmac<sha2::Sha256>;

impl SigV4Signer {
    pub fn new(region: String) -> SigV4Signer {
        SigV4Signer {
            access_key_id: env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID is missing"),
            secret_access_key: env::var("AWS_SECRET_ACCESS_KEY")
                .expect("AWS_SECRET_ACCESS_KEY is missing"),
            region,
        }
    }

    /// https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/reference_sigv-create-signed-request.html#derive-signing-key
    fn make_signing_key(&self, current_date: &str) -> sha2::digest::Output<sha2::Sha256> {
        let mut date_key =
            HmacSha256::new_from_slice(format!("AWS4{}", self.secret_access_key).as_bytes())
                .unwrap();
        date_key.update(current_date.as_bytes());
        let date_key = date_key.finalize();

        let mut date_region_key = HmacSha256::new_from_slice(date_key.as_bytes()).unwrap();
        date_region_key.update(self.region.as_bytes());
        let date_region_key = date_region_key.finalize();

        let mut date_region_service_key =
            HmacSha256::new_from_slice(date_region_key.as_bytes()).unwrap();
        date_region_service_key.update(AWS_SERVICE.as_bytes());
        let date_region_service_key = date_region_service_key.finalize();

        let mut signing_key =
            HmacSha256::new_from_slice(date_region_service_key.as_bytes()).unwrap();
        signing_key.update(b"aws4_request");

        *signing_key.finalize().as_bytes()
    }

    pub fn sign<T>(&self, req: &mut Request<T>) {
        let now = chrono::Utc::now();
        let current_datetime = format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
        );
        let current_date = format!("{:04}{:02}{:02}", now.year(), now.month(), now.day());
        {
            let host = HeaderValue::from_str(req.uri().host().unwrap()).unwrap();
            let headers = req.headers_mut();
            if !headers.contains_key("host") {
                headers.append("host", host);
            }
            if !headers.contains_key("x-amz-content-sha256") {
                headers.append("x-amz-content-sha256", "UNSIGNED-PAYLOAD".parse().unwrap());
            }
            if !headers.contains_key("x-amz-date") {
                headers.append("x-amz-date", current_datetime.parse().unwrap());
            }
        }

        let cred_scope = format!(
            "{}/{}/{}/aws4_request",
            current_date, self.region, AWS_SERVICE
        );
        let cred_scope_with_keyid = format!("{}/{}", self.access_key_id, cred_scope);

        let (canonical_request, signed_headers) = make_canonical_request(&req);
        let signing_key = self.make_signing_key(&current_date);

        let mut signature = HmacSha256::new_from_slice(&signing_key).unwrap();
        let signature_source =
            make_signature_string(&current_datetime, &cred_scope, canonical_request);
        tracing::trace!(source = signature_source, "S3_SIGNING");
        signature.update(signature_source.as_bytes());
        let signature = signature.finalize();

        req.headers_mut().append(
            "Authorization",
            format!(
                "AWS4-HMAC-SHA256 Credential={},SignedHeaders={},Signature={}",
                cred_scope_with_keyid,
                signed_headers,
                hex::encode(signature.as_bytes())
            )
            .parse()
            .unwrap(),
        );
    }
}
