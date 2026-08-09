use base64::{Engine as _, engine::general_purpose::STANDARD};
use rsa::{
    RsaPrivateKey,
    pkcs1::DecodeRsaPrivateKey,
    pkcs8::DecodePrivateKey,
    signature::{RandomizedSigner as _, SignatureEncoding},
};

use crate::config::SecretKeyConfig;

fn encode_base64_aws<T: AsRef<[u8]>>(input: T) -> String {
    STANDARD
        .encode(input)
        .chars()
        .map(|c| match c {
            '+' => '-',
            '=' => '_',
            '/' => '~',
            _ => c,
        })
        .collect()
}

pub struct CloudFrontSigningKey {
    key_id: String,
    signing_key: rsa::pkcs1v15::SigningKey<sha1::Sha1>,
}

impl CloudFrontSigningKey {
    pub fn new(config: SecretKeyConfig) -> Self {
        let signing_key = config.source.to_content();
        let signing_key = if signing_key.contains("RSA PRIVATE KEY") {
            RsaPrivateKey::from_pkcs1_pem(&signing_key).unwrap()
        } else {
            RsaPrivateKey::from_pkcs8_pem(&signing_key).unwrap()
        };
        let signing_key = rsa::pkcs1v15::SigningKey::<sha1::Sha1>::new(signing_key);
        Self {
            key_id: config.key_id,
            signing_key,
        }
    }

    pub fn sign_to_url(&self, url: &mut url::Url) {
        let policy = serde_json::json!({
            "Statement": [{
                "Resource": &url,
                "Condition": {
                    "DateLessThan": {
                        "AWS:EpochTime": (chrono::Utc::now().timestamp() + 60),
                    }
                }
            }]
        });
        let policy = serde_json::to_vec(&policy).unwrap();

        let mut rng = rsa::rand_core::OsRng;
        let sign = self.signing_key.sign_with_rng(&mut rng, &policy);

        url.query_pairs_mut()
            .append_pair("Key-Pair-Id", &self.key_id)
            .append_pair("Policy", &encode_base64_aws(policy))
            .append_pair("Signature", &encode_base64_aws(sign.to_bytes()));
    }
}
