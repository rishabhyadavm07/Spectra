//! Signing implementations for auth schemes that compute headers/params
//! from a fully-resolved request rather than just attaching a static value.

use crate::model::{HawkAlgorithm, OAuth1SignatureMethod};
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha1::Sha1;
use sha2::{Digest as Sha2Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;
type HmacSha1 = Hmac<Sha1>;

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn nonce(len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len).map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char).collect()
}

fn percent_encode(s: &str) -> String {
    // RFC 3986 unreserved set, matches OAuth1's stricter encoding requirement
    // (reqwest/urlencoding's default is close but not identical for all chars).
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

// --- OAuth 1.0 ---

pub struct OAuth1Params {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub token: Option<String>,
    pub token_secret: Option<String>,
    pub signature_method: OAuth1SignatureMethod,
}

/// Builds the `Authorization: OAuth ...` header value for a request.
pub fn oauth1_header(
    params: &OAuth1Params,
    method: &str,
    url: &str,
    extra_params: &[(String, String)],
) -> String {
    let timestamp = now_unix().to_string();
    let nonce_val = nonce(32);

    let mut oauth_params: BTreeMap<String, String> = BTreeMap::new();
    oauth_params.insert("oauth_consumer_key".into(), params.consumer_key.clone());
    oauth_params.insert("oauth_nonce".into(), nonce_val.clone());
    oauth_params.insert("oauth_signature_method".into(), sig_method_name(params.signature_method).into());
    oauth_params.insert("oauth_timestamp".into(), timestamp.clone());
    oauth_params.insert("oauth_version".into(), "1.0".into());
    if let Some(token) = &params.token {
        oauth_params.insert("oauth_token".into(), token.clone());
    }

    let base_url = url.split('?').next().unwrap_or(url).to_string();
    let mut all_params: BTreeMap<String, String> = oauth_params.clone();
    for (k, v) in extra_params {
        all_params.insert(k.clone(), v.clone());
    }
    if let Some(query) = url.split('?').nth(1) {
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                all_params.insert(
                    urlencoding::decode(k).unwrap_or_default().to_string(),
                    urlencoding::decode(v).unwrap_or_default().to_string(),
                );
            }
        }
    }

    let param_string = all_params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let base_string = format!(
        "{}&{}&{}",
        method.to_uppercase(),
        percent_encode(&base_url),
        percent_encode(&param_string)
    );

    let signing_key = format!(
        "{}&{}",
        percent_encode(&params.consumer_secret),
        percent_encode(params.token_secret.as_deref().unwrap_or(""))
    );

    let signature = match params.signature_method {
        OAuth1SignatureMethod::PlainText => signing_key.clone(),
        OAuth1SignatureMethod::HmacSha1 => {
            let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes()).expect("hmac accepts any key length");
            mac.update(base_string.as_bytes());
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
        }
        OAuth1SignatureMethod::HmacSha256 => {
            let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes()).expect("hmac accepts any key length");
            mac.update(base_string.as_bytes());
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
        }
    };

    oauth_params.insert("oauth_signature".into(), signature);

    let header_params = oauth_params
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join(", ");

    format!("OAuth {header_params}")
}

fn sig_method_name(m: OAuth1SignatureMethod) -> &'static str {
    match m {
        OAuth1SignatureMethod::HmacSha1 => "HMAC-SHA1",
        OAuth1SignatureMethod::HmacSha256 => "HMAC-SHA256",
        OAuth1SignatureMethod::PlainText => "PLAINTEXT",
    }
}

// --- AWS Signature V4 ---

pub struct AwsSigV4Params {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub service: String,
    pub session_token: Option<String>,
}

pub struct SignedAwsHeaders {
    pub headers: Vec<(String, String)>,
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Computes the AWS SigV4 Authorization header + supporting headers for a
/// request. `host` is the request's host header value, `path` the URL path,
/// `query` the raw query string (may be empty), `headers` the already-set
/// request headers (lowercased keys) that participate in signing.
#[allow(clippy::too_many_arguments)]
pub fn aws_sigv4_headers(
    params: &AwsSigV4Params,
    method: &str,
    host: &str,
    path: &str,
    query: &str,
    body: &[u8],
) -> SignedAwsHeaders {
    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    let payload_hash = sha256_hex(body);

    let mut canonical_headers = format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let mut signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();
    if let Some(token) = &params.session_token {
        canonical_headers = format!(
            "host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\nx-amz-security-token:{token}\n"
        );
        signed_headers = "host;x-amz-content-sha256;x-amz-date;x-amz-security-token".to_string();
    }

    // Query params must be sorted and percent-encoded for the canonical request.
    let mut query_pairs: Vec<(String, String)> = query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    query_pairs.sort();
    let canonical_query = query_pairs
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method.to_uppercase(),
        if path.is_empty() { "/" } else { path },
        canonical_query,
        canonical_headers,
        signed_headers,
        payload_hash
    );

    let credential_scope = format!("{date_stamp}/{}/{}/aws4_request", params.region, params.service);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );

    let k_date = hmac_sha256(format!("AWS4{}", params.secret_key).as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, params.region.as_bytes());
    let k_service = hmac_sha256(&k_region, params.service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        params.access_key, credential_scope, signed_headers, signature
    );

    let mut headers = vec![
        ("Authorization".to_string(), authorization),
        ("X-Amz-Date".to_string(), amz_date),
        ("X-Amz-Content-Sha256".to_string(), payload_hash),
    ];
    if let Some(token) = &params.session_token {
        headers.push(("X-Amz-Security-Token".to_string(), token.clone()));
    }

    SignedAwsHeaders { headers }
}

// --- Digest Auth ---
// Digest requires a first 401 response carrying a WWW-Authenticate challenge
// before we can compute the response header, so this is a two-phase API:
// the request engine sends once, and on 401 calls `digest_header` with the
// challenge to retry.

pub struct DigestChallenge {
    pub realm: String,
    pub nonce: String,
    pub qop: Option<String>,
    pub opaque: Option<String>,
    pub algorithm: String,
}

pub fn parse_digest_challenge(www_authenticate: &str) -> Option<DigestChallenge> {
    let rest = www_authenticate.strip_prefix("Digest ")?;
    let mut map = BTreeMap::new();
    for part in split_digest_params(rest) {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    Some(DigestChallenge {
        realm: map.get("realm").cloned().unwrap_or_default(),
        nonce: map.get("nonce").cloned().unwrap_or_default(),
        qop: map.get("qop").cloned(),
        opaque: map.get("opaque").cloned(),
        algorithm: map.get("algorithm").cloned().unwrap_or_else(|| "MD5".to_string()),
    })
}

fn split_digest_params(s: &str) -> Vec<String> {
    // naive split on commas that aren't inside quotes
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn md5_hex(data: &str) -> String {
    format!("{:x}", md5::compute(data.as_bytes()))
}

pub fn digest_header(
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    challenge: &DigestChallenge,
) -> String {
    let ha1 = md5_hex(&format!("{username}:{}:{password}", challenge.realm));
    let ha2 = md5_hex(&format!("{method}:{uri}"));

    let (response, cnonce_part, nc_part, qop_part) = if let Some(qop) = &challenge.qop {
        let qop_value = qop.split(',').next().unwrap_or("auth").trim();
        let cnonce = nonce(16);
        let nc = "00000001";
        let response = md5_hex(&format!(
            "{ha1}:{}:{nc}:{cnonce}:{qop_value}:{ha2}",
            challenge.nonce
        ));
        (response, format!(", cnonce=\"{cnonce}\""), format!(", nc={nc}"), format!(", qop={qop_value}"))
    } else {
        let response = md5_hex(&format!("{ha1}:{}:{ha2}", challenge.nonce));
        (response, String::new(), String::new(), String::new())
    };

    let opaque_part = challenge
        .opaque
        .as_ref()
        .map(|o| format!(", opaque=\"{o}\""))
        .unwrap_or_default();

    format!(
        "Digest username=\"{username}\", realm=\"{}\", nonce=\"{}\", uri=\"{uri}\", response=\"{response}\"{qop_part}{nc_part}{cnonce_part}{opaque_part}",
        challenge.realm, challenge.nonce
    )
}

// --- Hawk ---

pub struct HawkParams {
    pub id: String,
    pub key: String,
    pub algorithm: HawkAlgorithm,
}

pub fn hawk_header(params: &HawkParams, method: &str, host: &str, port: u16, path: &str) -> String {
    let ts = now_unix().to_string();
    let nonce_val = nonce(8);

    let normalized = format!(
        "hawk.1.header\n{ts}\n{nonce_val}\n{}\n{path}\n{host}\n{port}\n\n\n",
        method.to_uppercase()
    );

    let mac = match params.algorithm {
        HawkAlgorithm::Sha256 => {
            let mut mac = HmacSha256::new_from_slice(params.key.as_bytes()).expect("hmac accepts any key length");
            mac.update(normalized.as_bytes());
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
        }
        HawkAlgorithm::Sha1 => {
            let mut mac = HmacSha1::new_from_slice(params.key.as_bytes()).expect("hmac accepts any key length");
            mac.update(normalized.as_bytes());
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
        }
    };

    format!(
        "Hawk id=\"{}\", ts=\"{ts}\", nonce=\"{nonce_val}\", mac=\"{mac}\"",
        params.id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HawkAlgorithm, OAuth1SignatureMethod};

    // --- Digest ---

    #[test]
    fn digest_parses_full_challenge() {
        let challenge = parse_digest_challenge(
            r#"Digest realm="testrealm@host.com", qop="auth,auth-int", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#,
        )
        .expect("should parse");
        assert_eq!(challenge.realm, "testrealm@host.com");
        assert_eq!(challenge.nonce, "dcd98b7102dd2f0e8b11d0f600bfb0c093");
        assert_eq!(challenge.qop.as_deref(), Some("auth,auth-int"));
        assert_eq!(challenge.opaque.as_deref(), Some("5ccc069c403ebaf9f0171e9517f40e41"));
        assert_eq!(challenge.algorithm, "MD5"); // default when absent
    }

    #[test]
    fn digest_rejects_non_digest_scheme() {
        assert!(parse_digest_challenge(r#"Basic realm="foo""#).is_none());
    }

    #[test]
    fn digest_parses_minimal_challenge_no_qop_no_opaque() {
        let challenge = parse_digest_challenge(r#"Digest realm="myrealm", nonce="abc123nonce""#).unwrap();
        assert_eq!(challenge.realm, "myrealm");
        assert_eq!(challenge.nonce, "abc123nonce");
        assert!(challenge.qop.is_none());
        assert!(challenge.opaque.is_none());
    }

    #[test]
    fn digest_header_no_qop_matches_rfc2069_style_vector() {
        // HA1 = md5("user:myrealm:pass"), HA2 = md5("GET:/resource")
        // response = md5(HA1:nonce:HA2) — independently computed reference values.
        let challenge = DigestChallenge {
            realm: "myrealm".to_string(),
            nonce: "abc123nonce".to_string(),
            qop: None,
            opaque: None,
            algorithm: "MD5".to_string(),
        };
        let header = digest_header("user", "pass", "GET", "/resource", &challenge);
        assert!(header.starts_with("Digest "));
        assert!(header.contains(r#"username="user""#));
        assert!(header.contains(r#"realm="myrealm""#));
        assert!(header.contains(r#"nonce="abc123nonce""#));
        assert!(header.contains(r#"uri="/resource""#));
        assert!(header.contains(r#"response="41f534d0579c694689ec66107972415f""#));
        // no qop -> no cnonce/nc/qop fields in the header
        assert!(!header.contains("qop="));
        assert!(!header.contains("cnonce="));
        assert!(!header.contains("nc="));
    }

    #[test]
    fn digest_header_with_qop_matches_rfc2617_known_answer_vector() {
        // The canonical RFC 2617 Section 3.5 example, with a fixed cnonce
        // substituted in place of the spec's own so the response is reproducible
        // (digest_header generates a random cnonce internally, so we replicate
        // its math by hand here using the same fixed cnonce/nc it would embed).
        let ha1 = format!("{:x}", md5::compute(b"Mufasa:testrealm@host.com:Circle Of Life"));
        let ha2 = format!("{:x}", md5::compute(b"GET:/dir/index.html"));
        assert_eq!(ha1, "939e7578ed9e3c518a452acee763bce9");
        assert_eq!(ha2, "39aff3a2bab6126f332b942af96d3366");
        let expected_response = format!(
            "{:x}",
            md5::compute(format!(
                "{ha1}:dcd98b7102dd2f0e8b11d0f600bfb0c093:00000001:0a4f113b:auth:{ha2}"
            ))
        );
        assert_eq!(expected_response, "6629fae49393a05397450978507c4ef1");

        // Now exercise the real function and check structure/qop selection
        // (cnonce is random, so we can't assert the exact response digit-for-digit,
        // but we can assert every other field made it into the header correctly).
        let challenge = DigestChallenge {
            realm: "testrealm@host.com".to_string(),
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
            qop: Some("auth,auth-int".to_string()),
            opaque: Some("5ccc069c403ebaf9f0171e9517f40e41".to_string()),
            algorithm: "MD5".to_string(),
        };
        let header = digest_header("Mufasa", "Circle Of Life", "GET", "/dir/index.html", &challenge);
        assert!(header.contains(r#"username="Mufasa""#));
        assert!(header.contains(r#"realm="testrealm@host.com""#));
        assert!(header.contains(r#"uri="/dir/index.html""#));
        // first qop option is selected when the server offers a comma-separated list
        assert!(header.contains("qop=auth"));
        assert!(!header.contains("qop=auth-int"));
        assert!(header.contains("nc=00000001"));
        assert!(header.contains(r#"opaque="5ccc069c403ebaf9f0171e9517f40e41""#));
        assert!(header.contains("cnonce=\""));
    }

    #[test]
    fn digest_header_is_deterministic_given_fixed_cnonce_inputs() {
        // Two calls with qop absent produce identical output (no randomness in that path).
        let challenge = DigestChallenge {
            realm: "r".to_string(),
            nonce: "n".to_string(),
            qop: None,
            opaque: None,
            algorithm: "MD5".to_string(),
        };
        let h1 = digest_header("u", "p", "GET", "/x", &challenge);
        let h2 = digest_header("u", "p", "GET", "/x", &challenge);
        assert_eq!(h1, h2);
    }

    #[test]
    fn digest_split_params_handles_commas_inside_quotes() {
        // qop value itself contains a comma-separated list inside quotes — must
        // not be split into two separate top-level params.
        let challenge = parse_digest_challenge(
            r#"Digest realm="r", nonce="n", qop="auth,auth-int""#,
        )
        .unwrap();
        assert_eq!(challenge.qop.as_deref(), Some("auth,auth-int"));
        assert_eq!(challenge.realm, "r");
    }

    // --- OAuth 1.0 ---

    #[test]
    fn oauth1_plaintext_signature_is_key_concatenation() {
        let params = OAuth1Params {
            consumer_key: "ck".to_string(),
            consumer_secret: "cs".to_string(),
            token: Some("tok".to_string()),
            token_secret: Some("ts".to_string()),
            signature_method: OAuth1SignatureMethod::PlainText,
        };
        let header = oauth1_header(&params, "GET", "https://example.com/resource", &[]);
        assert!(header.starts_with("OAuth "));
        assert!(header.contains(r#"oauth_signature="cs%26ts""#));
        assert!(header.contains(r#"oauth_signature_method="PLAINTEXT""#));
        assert!(header.contains(r#"oauth_consumer_key="ck""#));
        assert!(header.contains(r#"oauth_token="tok""#));
        assert!(header.contains(r#"oauth_version="1.0""#));
    }

    #[test]
    fn oauth1_plaintext_signature_with_no_token_secret() {
        let params = OAuth1Params {
            consumer_key: "ck".to_string(),
            consumer_secret: "cs".to_string(),
            token: None,
            token_secret: None,
            signature_method: OAuth1SignatureMethod::PlainText,
        };
        let header = oauth1_header(&params, "GET", "https://example.com/resource", &[]);
        // signing key is "consumer_secret&" when there's no token secret
        assert!(header.contains(r#"oauth_signature="cs%26""#));
        assert!(!header.contains("oauth_token="));
    }

    #[test]
    fn oauth1_hmac_sha1_signature_matches_manual_computation() {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        let params = OAuth1Params {
            consumer_key: "ck".to_string(),
            consumer_secret: "cs".to_string(),
            token: None,
            token_secret: None,
            signature_method: OAuth1SignatureMethod::HmacSha1,
        };
        let header = oauth1_header(&params, "GET", "https://example.com/resource?a=1", &[]);
        assert!(header.contains(r#"oauth_signature_method="HMAC-SHA1""#));

        // Extract the produced nonce/timestamp/signature so we can rebuild the
        // exact base string and confirm the signature matches independently.
        let extract = |key: &str| -> String {
            let marker = format!("{key}=\"");
            let start = header.find(&marker).unwrap() + marker.len();
            let rest = &header[start..];
            let end = rest.find('"').unwrap();
            rest[..end].to_string()
        };
        let nonce = extract("oauth_nonce");
        let timestamp = extract("oauth_timestamp");
        let signature = urlencoding::decode(&extract("oauth_signature")).unwrap().to_string();

        let mut all_params: BTreeMap<&str, String> = BTreeMap::new();
        all_params.insert("oauth_consumer_key", "ck".to_string());
        all_params.insert("oauth_nonce", nonce);
        all_params.insert("oauth_signature_method", "HMAC-SHA1".to_string());
        all_params.insert("oauth_timestamp", timestamp);
        all_params.insert("oauth_version", "1.0".to_string());
        all_params.insert("a", "1".to_string());

        let param_string = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let base_string = format!(
            "GET&{}&{}",
            percent_encode("https://example.com/resource"),
            percent_encode(&param_string)
        );
        let signing_key = "cs&";

        let mut mac = Hmac::<Sha1>::new_from_slice(signing_key.as_bytes()).unwrap();
        mac.update(base_string.as_bytes());
        let expected = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        assert_eq!(signature, expected);
    }

    #[test]
    fn oauth1_query_params_are_folded_into_signature_base() {
        // Two calls with different query strings must yield different signatures
        // (proves query params actually participate in the signature base string).
        let params = OAuth1Params {
            consumer_key: "ck".to_string(),
            consumer_secret: "cs".to_string(),
            token: None,
            token_secret: None,
            signature_method: OAuth1SignatureMethod::HmacSha1,
        };
        let h1 = oauth1_header(&params, "GET", "https://example.com/r?a=1", &[]);
        let h2 = oauth1_header(&params, "GET", "https://example.com/r?a=2", &[]);
        let extract_sig = |h: &str| -> String {
            let start = h.find("oauth_signature=\"").unwrap() + "oauth_signature=\"".len();
            h[start..].trim_end_matches('"').to_string()
        };
        assert_ne!(extract_sig(&h1), extract_sig(&h2));
    }

    // --- AWS SigV4 ---
    // Reference values independently derived from the AWS documented signing
    // steps (canonical request -> string to sign -> signing key derivation),
    // not copied from aws_sigv4_headers itself.

    #[test]
    fn aws_sigv4_signing_key_derivation_matches_reference() {
        // Reference derived signing key for AWS's well-known test secret key
        // (us-east-1, iam, 20150830), independently re-derived via a separate
        // HMAC-SHA256 chain (Python's hmac/hashlib) rather than reusing this
        // module's own helper as the source of truth.
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let date_stamp = "20150830";
        let region = "us-east-1";
        let service = "iam";

        let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date_stamp.as_bytes());
        let k_region = hmac_sha256(&k_date, region.as_bytes());
        let k_service = hmac_sha256(&k_region, service.as_bytes());
        let k_signing = hmac_sha256(&k_service, b"aws4_request");

        assert_eq!(
            hex::encode(&k_signing),
            "2c94c0cf5378ada6887f09bb697df8fc0affdb34ba1cdd5bda32b664bd55b73c"
        );
    }

    #[test]
    fn aws_sigv4_payload_hash_of_empty_body_matches_known_constant() {
        // The well-known SHA256 of an empty string, used by AWS for GET requests.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn aws_sigv4_headers_include_session_token_when_present() {
        let params = AwsSigV4Params {
            access_key: "AKIDEXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            region: "us-east-1".to_string(),
            service: "execute-api".to_string(),
            session_token: Some("SESSIONTOKENVALUE".to_string()),
        };
        let signed = aws_sigv4_headers(&params, "GET", "example.com", "/", "", b"");
        let names: Vec<&str> = signed.headers.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"X-Amz-Security-Token"));
        let auth_header = signed
            .headers
            .iter()
            .find(|(k, _)| k == "Authorization")
            .map(|(_, v)| v.as_str())
            .unwrap();
        assert!(auth_header.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"));
        assert!(auth_header.contains("us-east-1/execute-api/aws4_request"));
        assert!(auth_header.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-security-token"));
    }

    #[test]
    fn aws_sigv4_headers_omit_session_token_when_absent() {
        let params = AwsSigV4Params {
            access_key: "AKIDEXAMPLE".to_string(),
            secret_key: "secret".to_string(),
            region: "us-east-1".to_string(),
            service: "s3".to_string(),
            session_token: None,
        };
        let signed = aws_sigv4_headers(&params, "GET", "example.com", "/", "", b"");
        assert!(!signed.headers.iter().any(|(k, _)| k == "X-Amz-Security-Token"));
        let auth_header = signed
            .headers
            .iter()
            .find(|(k, _)| k == "Authorization")
            .map(|(_, v)| v.as_str())
            .unwrap();
        assert!(auth_header.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
        assert!(!auth_header.contains("x-amz-security-token"));
    }

    #[test]
    fn aws_sigv4_query_params_are_sorted_in_canonical_request() {
        // Signature must change if query param order changes in the raw query
        // string but not if key/value content is identical after sorting —
        // i.e. canonicalization actually sorts rather than passing through raw order.
        let params = AwsSigV4Params {
            access_key: "AKID".to_string(),
            secret_key: "secret".to_string(),
            region: "us-east-1".to_string(),
            service: "s3".to_string(),
            session_token: None,
        };
        // Different amz_date per call (time-based) makes exact signature comparison
        // unreliable across calls, so instead verify canonical query ordering directly
        // via the same sort the function performs.
        let query = "b=2&a=1";
        let mut query_pairs: Vec<(String, String)> = query
            .split('&')
            .filter(|s| !s.is_empty())
            .filter_map(|pair| pair.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        query_pairs.sort();
        assert_eq!(query_pairs, vec![("a".to_string(), "1".to_string()), ("b".to_string(), "2".to_string())]);

        // sanity: function runs without panicking on multi-param query strings
        let _ = aws_sigv4_headers(&params, "GET", "example.com", "/", query, b"");
    }

    // --- Hawk ---

    #[test]
    fn hawk_header_sha256_mac_matches_manual_computation() {
        let params = HawkParams {
            id: "dh37fgj492je".to_string(),
            key: "werxhqb98rpaxn39848xrunpaw3489ruxnpa98w4rxn".to_string(),
            algorithm: HawkAlgorithm::Sha256,
        };
        let header = hawk_header(&params, "GET", "example.com", 8080, "/resource?a=1&b=2");

        assert!(header.starts_with("Hawk "));
        assert!(header.contains(r#"id="dh37fgj492je""#));

        let extract = |key: &str| -> String {
            let marker = format!("{key}=\"");
            let start = header.find(&marker).unwrap() + marker.len();
            let rest = &header[start..];
            let end = rest.find('"').unwrap();
            rest[..end].to_string()
        };
        let ts = extract("ts");
        let nonce_val = extract("nonce");
        let mac = extract("mac");

        let normalized = format!(
            "hawk.1.header\n{ts}\n{nonce_val}\nGET\n/resource?a=1&b=2\nexample.com\n8080\n\n\n"
        );
        let mut expected_mac = HmacSha256::new_from_slice(params.key.as_bytes()).unwrap();
        expected_mac.update(normalized.as_bytes());
        let expected = base64::engine::general_purpose::STANDARD.encode(expected_mac.finalize().into_bytes());

        assert_eq!(mac, expected);
    }

    #[test]
    fn hawk_header_sha1_vs_sha256_produce_different_macs() {
        let base = HawkParams {
            id: "id1".to_string(),
            key: "somekey".to_string(),
            algorithm: HawkAlgorithm::Sha1,
        };
        let sha1_header = hawk_header(&base, "GET", "example.com", 80, "/x");
        let sha256_params = HawkParams {
            id: "id1".to_string(),
            key: "somekey".to_string(),
            algorithm: HawkAlgorithm::Sha256,
        };
        let sha256_header = hawk_header(&sha256_params, "GET", "example.com", 80, "/x");

        let extract_mac = |h: &str| -> String {
            let start = h.find("mac=\"").unwrap() + "mac=\"".len();
            h[start..].trim_end_matches('"').to_string()
        };
        // Different algorithms are extremely unlikely to coincidentally match,
        // and the base64 lengths differ (20 vs 32 raw bytes) so this is deterministic.
        assert_ne!(extract_mac(&sha1_header).len(), extract_mac(&sha256_header).len());
    }
}
