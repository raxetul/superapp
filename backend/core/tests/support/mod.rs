//! Shared test support: the deterministic RSA key material (matching
//! `config/test.yaml`'s static JWKS) and a helper to mint signed access tokens.

#![allow(dead_code)]

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub const ISSUER: &str = "https://rauthy.example/auth/v1";
pub const AUDIENCE: &str = "superapp";
pub const KID: &str = "test-key-1";

/// The public JWKS matching [`PRIVATE_PEM`] (single RSA key, kid `test-key-1`).
pub const JWKS_JSON: &str = r#"{"keys":[{"kty":"RSA","use":"sig","alg":"RS256","kid":"test-key-1","n":"rOWhAhYJGCX4lcWllUuqfSCDbgNjLSHbTkI8wlAtIRI5gySpt8wzIzgX0sMQMpMiQhM_973W9aS-I0ytoDSXZ_qs8j8woqTxWoxGR_U3uQWD6dxJg5Fun3po8AvBO6T7AE34LlGV6ENT7d4Ado0dtCGoLlOvQqRKD3oSTpPKzXIeXq1hLtU9FM2Xqhj4k749fZ411NJByq0dH2IKyweuRKxtu4k2_biMqKF2Bsf8qeeu1N2e0mzZHIzNcFin4FnotJZ-qHJdAYv391c2tnHki5Z8N67M6cbPGKXp0-Ci5UZvvTyTpYadBx9e46kYLQQvZk0SQ0Fg-ILfhs4NCSmMPQ","e":"AQAB"}]}"#;

pub const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCs5aECFgkYJfiV
xaWVS6p9IINuA2MtIdtOQjzCUC0hEjmDJKm3zDMjOBfSwxAykyJCEz/3vdb1pL4j
TK2gNJdn+qzyPzCipPFajEZH9Te5BYPp3EmDkW6femjwC8E7pPsATfguUZXoQ1Pt
3gB2jR20IaguU69CpEoPehJOk8rNch5erWEu1T0UzZeqGPiTvj19njXU0kHKrR0f
YgrLB65ErG27iTb9uIyooXYGx/yp567U3Z7SbNkcjM1wWKfgWei0ln6ocl0Bi/f3
Vza2ceSLlnw3rszpxs8YpenT4KLlRm+9PJOlhp0HH17jqRgtBC9mTRJDQWD4gt+G
zg0JKYw9AgMBAAECggEABKJI7PHjP4LxBS6dcAFtqdnfu+iaVWDVbGUJFNoqQ3sm
EHZAmD0AV97OgKcavYhFAElcywqLAt1t/Ss2chdGwZIt5XY9GvbZwygEUDRp0Gst
7GwrijuxH9QbkOd3v939njX4w2ELaAS58KOlFohUtfm69LboeRxHIeAPMozygAwW
J2VFZZqesUpR5w1suf6/6sKXOXzpMOTQ2MGEX/nhtygXz923IGJOTp76Zd01tGJO
bCioi/N+o8XE6X/yoWjPkI0Md3cJwB8n90GygVtwlgUC6oMtTy6sCNRt/nF0Sses
9ZUviyp7XYpyAhYL7Xa1JcnnChiBHkZO03buI7sbsQKBgQDhZdVWLqlkMoz2h2D6
FO64Ug2j7BnpyRoVZov9/Z/hw9n3Q2OzjrWo/5ZQCGTVUD+SCkHPCVexNki1DO7t
Fn26TxWNY3wnd0w9nKFLfa+J4HHBV2mAUg/hVv2cTlRfnuLlZsKuh7wEtD4R8qjD
ayy4zSCgGtqxaPPJKhZ82vdfxQKBgQDEXwWLo2cMkDJRgUJWdrmr3Scy8fSCl4tX
8Z+YbS1BKn8aAB8/W0XUu3BK0TDcpH6tKATdCEG9ij3fzOEl7nNSal6d+XcpWNru
1cSZ0MdRWMihAMIVFaPzS2QewZolp5rjbD5psUfuPJvYof/2+ByaTmZCNUsJOcyT
qaH6VO2KGQKBgDL4ODn4601BMSc+jet/eEwuIe+DoIiBdWiJe/a/t7vx8gQ8NPuR
tfq1cWQ/wa2LLBT2RNNwpWfpgp+OgSkUAPJ0l8cVTCoQjCkSllbf4SYonxC9l5nw
9K5cYZVLEUFeSKjfh/63qwjVSYP9f7RRNBfGKy6JZBLiTN6cqeEqEu6RAoGAQ6Lq
+Q6+FrCv9CzOWZOpHg4dS0nVSwnBf/gEXW71UGW/w4fZO2xHoB8zbUGiT80EqMWI
70zBv/QWnbgmsHbyF6f1kPn01jP5rJvKjKRBkr4+1F27e6Gph8om4QUT//Y7vitx
T9w1B1Su5S3fSQRqbaelRxauEq5qzU13Mg8AhjkCgYA5bpsXvUy2MXUvsadU4J6m
herU4i8hU1VNqe5rggBLogsi0IjRbnwdTpW69J3aH+IuRfwUyihTj+jtOI2CFWq7
l3vK3A8XrIO8xM9gkaMtX/S+rPYtkHoSVsONfXlkWVXLJwS5YYV2sF29jJ5ovS7k
rJXijqEIwJ6DXCyl3hiZ5g==
-----END PRIVATE KEY-----";

/// Seconds since the Unix epoch.
#[must_use]
pub fn now() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

/// Sign a Rauthy-style access token for `email` (valid `iss`/`aud`, 1h expiry),
/// using the deterministic test key.
#[must_use]
pub fn access_token(email: &str) -> String {
    let claims = serde_json::json!({
        "sub": format!("sub-{email}"),
        "iss": ISSUER,
        "aud": AUDIENCE,
        "exp": now() + 3600,
        "email": email,
        "email_verified": true,
    });
    sign(&claims)
}

/// Sign arbitrary claims with the test key (kid `test-key-1`, RS256).
#[must_use]
pub fn sign(claims: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KID.to_string());
    let key = EncodingKey::from_rsa_pem(PRIVATE_PEM.as_bytes()).unwrap();
    encode(&header, claims, &key).unwrap()
}

/// The `Authorization: Bearer …` header value for `email`.
#[must_use]
pub fn bearer(email: &str) -> String {
    format!("Bearer {}", access_token(email))
}
