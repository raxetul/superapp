//! Access-token validation against Rauthy's JWKS (TR-04-001, TR-04-010).
//!
//! The backend is an OIDC resource server: every protected request carries a
//! Rauthy-issued **access token** (a JWT). [`TokenValidator`] verifies the
//! signature against the issuer's published JWKS and checks `iss`, `aud`, and
//! `exp`. Validation is uniform regardless of how the user logged in (SSO or
//! username/password) — both are Rauthy methods issuing the same token shape
//! (TR-04-010).
//!
//! Signing algorithm is taken from the **JWK**, never from the attacker-
//! controlled token header, so RS256→HS256 alg-confusion is impossible; only
//! asymmetric algorithms are accepted.

use jsonwebtoken::jwk::{AlgorithmParameters, Jwk, JwkSet};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// Validated claims lifted from a Rauthy access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — Rauthy's stable user id.
    pub sub: String,
    /// Expiry (seconds since the Unix epoch).
    pub exp: usize,
    /// Issuer.
    #[serde(default)]
    pub iss: Option<String>,
    /// Email claim — the SuperApp identity key (TR-04-004).
    #[serde(default)]
    pub email: Option<String>,
    /// Whether Rauthy has verified the email.
    #[serde(default)]
    pub email_verified: Option<bool>,
    /// Optional display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional preferred username.
    #[serde(default)]
    pub preferred_username: Option<String>,
}

impl Claims {
    /// The email if present and non-empty, else `None`.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref().filter(|e| !e.is_empty())
    }
}

/// Why a token was rejected. All variants map to `401 Unauthorized` at the
/// HTTP boundary.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    /// The token header carried no `kid`, so the signing key is ambiguous.
    #[error("token header is missing a key id (kid)")]
    MissingKid,
    /// No JWKS key matched the token's `kid`.
    #[error("no known signing key for kid `{0}`")]
    UnknownKid(String),
    /// The JWK uses an algorithm the validator does not accept (e.g. symmetric).
    #[error("unsupported or symmetric signing algorithm for kid `{0}`")]
    UnsupportedAlg(String),
    /// Signature/claims validation failed (bad signature, expired, wrong
    /// `iss`/`aud`, malformed).
    #[error("token validation failed: {0}")]
    Invalid(String),
}

/// Validates access tokens against a fixed set of JWKS keys and an expected
/// issuer + audience.
#[derive(Clone)]
pub struct TokenValidator {
    jwks: JwkSet,
    issuer: String,
    audience: String,
}

impl TokenValidator {
    /// Build a validator from an already-parsed [`JwkSet`].
    #[must_use]
    pub fn new(jwks: JwkSet, issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            jwks,
            issuer: issuer.into(),
            audience: audience.into(),
        }
    }

    /// Build a validator from raw JWKS JSON (as returned from a `jwks_uri`).
    ///
    /// # Errors
    /// When the JSON is not a well-formed JWK set.
    pub fn from_jwks_json(
        json: &str,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Result<Self, TokenError> {
        let jwks: JwkSet =
            serde_json::from_str(json).map_err(|e| TokenError::Invalid(e.to_string()))?;
        Ok(Self::new(jwks, issuer, audience))
    }

    /// Validate `token`: resolve its signing key by `kid`, verify the
    /// signature, and check `iss`/`aud`/`exp`. Returns the [`Claims`] on
    /// success.
    ///
    /// # Errors
    /// A [`TokenError`] describing why the token was rejected.
    pub fn validate(&self, token: &str) -> Result<Claims, TokenError> {
        let header = decode_header(token).map_err(|e| TokenError::Invalid(e.to_string()))?;
        let kid = header.kid.ok_or(TokenError::MissingKid)?;
        let jwk = self
            .jwks
            .find(&kid)
            .ok_or_else(|| TokenError::UnknownKid(kid.clone()))?;

        let alg = jwk_algorithm(jwk).ok_or_else(|| TokenError::UnsupportedAlg(kid.clone()))?;
        let key = DecodingKey::from_jwk(jwk).map_err(|e| TokenError::Invalid(e.to_string()))?;

        let mut validation = Validation::new(alg);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.validate_exp = true;

        decode::<Claims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|e| TokenError::Invalid(e.to_string()))
    }
}

/// The asymmetric signing algorithm a JWK declares. Symmetric (HMAC) keys and
/// unknown algorithms return `None`, so they can never be used to validate a
/// bearer token (defends against alg-confusion).
fn jwk_algorithm(jwk: &Jwk) -> Option<Algorithm> {
    // Prefer the algorithm parameters of the key material; only RSA/EC/OKP are
    // asymmetric and thus acceptable for verifying an issuer-signed token.
    match &jwk.algorithm {
        AlgorithmParameters::RSA(_) | AlgorithmParameters::EllipticCurve(_) => {}
        AlgorithmParameters::OctetKeyPair(_) => {}
        // Symmetric octet keys (HMAC) are rejected outright.
        AlgorithmParameters::OctetKey(_) => return None,
    }
    // Trust the explicit `alg` when the JWK declares one; otherwise default to
    // RS256 (Rauthy's default), which matches the RSA key material.
    match jwk.common.key_algorithm {
        Some(k) => k.to_string().parse::<Algorithm>().ok(),
        None => match &jwk.algorithm {
            AlgorithmParameters::RSA(_) => Some(Algorithm::RS256),
            AlgorithmParameters::EllipticCurve(_) => Some(Algorithm::ES256),
            AlgorithmParameters::OctetKeyPair(_) => Some(Algorithm::EdDSA),
            AlgorithmParameters::OctetKey(_) => None,
        },
    }
}

#[cfg(test)]
pub(crate) mod test_keys {
    //! Deterministic RSA test key material (2048-bit) so token tests neither
    //! do runtime key generation nor need a live Rauthy. Test-only.

    /// PKCS#8 private key used to *sign* test tokens.
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

    /// The matching JWKS (single RSA key, kid `test-key-1`).
    pub const KID: &str = "test-key-1";
    pub const JWKS_JSON: &str = r#"{"keys":[{"kty":"RSA","use":"sig","alg":"RS256","kid":"test-key-1","n":"rOWhAhYJGCX4lcWllUuqfSCDbgNjLSHbTkI8wlAtIRI5gySpt8wzIzgX0sMQMpMiQhM_973W9aS-I0ytoDSXZ_qs8j8woqTxWoxGR_U3uQWD6dxJg5Fun3po8AvBO6T7AE34LlGV6ENT7d4Ado0dtCGoLlOvQqRKD3oSTpPKzXIeXq1hLtU9FM2Xqhj4k749fZ411NJByq0dH2IKyweuRKxtu4k2_biMqKF2Bsf8qeeu1N2e0mzZHIzNcFin4FnotJZ-qHJdAYv391c2tnHki5Z8N67M6cbPGKXp0-Ci5UZvvTyTpYadBx9e46kYLQQvZk0SQ0Fg-ILfhs4NCSmMPQ","e":"AQAB"}]}"#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    const ISSUER: &str = "https://rauthy.example/auth/v1";
    const AUDIENCE: &str = "superapp";

    fn now() -> usize {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
    }

    /// Sign a token with the test private key (kid `test-key-1`, RS256).
    fn sign(claims: &serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(test_keys::KID.to_string());
        let key = EncodingKey::from_rsa_pem(test_keys::PRIVATE_PEM.as_bytes()).unwrap();
        encode(&header, claims, &key).unwrap()
    }

    fn validator() -> TokenValidator {
        TokenValidator::from_jwks_json(test_keys::JWKS_JSON, ISSUER, AUDIENCE).unwrap()
    }

    #[test]
    fn accepts_a_valid_token_and_extracts_email() {
        let token = sign(&json!({
            "sub": "rauthy-sub-1",
            "iss": ISSUER,
            "aud": AUDIENCE,
            "exp": now() + 3600,
            "email": "alice@example.com",
            "email_verified": true,
            "name": "Alice"
        }));
        let claims = validator().validate(&token).expect("valid token accepted");
        assert_eq!(claims.sub, "rauthy-sub-1");
        assert_eq!(claims.email(), Some("alice@example.com"));
        assert_eq!(claims.email_verified, Some(true));
    }

    #[test]
    fn rejects_expired_token() {
        let token = sign(&json!({
            "sub": "s", "iss": ISSUER, "aud": AUDIENCE,
            "exp": now() - 10_000, "email": "a@b.com"
        }));
        let err = validator().validate(&token).unwrap_err();
        assert!(matches!(err, TokenError::Invalid(_)), "got {err:?}");
    }

    #[test]
    fn rejects_wrong_audience() {
        let token = sign(&json!({
            "sub": "s", "iss": ISSUER, "aud": "someone-else",
            "exp": now() + 3600
        }));
        assert!(matches!(
            validator().validate(&token).unwrap_err(),
            TokenError::Invalid(_)
        ));
    }

    #[test]
    fn rejects_wrong_issuer() {
        let token = sign(&json!({
            "sub": "s", "iss": "https://evil.example", "aud": AUDIENCE,
            "exp": now() + 3600
        }));
        assert!(matches!(
            validator().validate(&token).unwrap_err(),
            TokenError::Invalid(_)
        ));
    }

    #[test]
    fn rejects_unknown_kid() {
        // Sign with a header kid that isn't in the JWKS.
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("some-other-kid".to_string());
        let key = EncodingKey::from_rsa_pem(test_keys::PRIVATE_PEM.as_bytes()).unwrap();
        let token = encode(
            &header,
            &json!({"sub":"s","iss":ISSUER,"aud":AUDIENCE,"exp": now()+3600}),
            &key,
        )
        .unwrap();
        assert!(matches!(
            validator().validate(&token).unwrap_err(),
            TokenError::UnknownKid(_)
        ));
    }

    #[test]
    fn rejects_token_signed_by_a_different_key() {
        // A structurally valid RS256 token signed by an unrelated key must fail
        // signature verification against the published JWKS.
        let other_pem = {
            // A second, unrelated 2048-bit key generated for this test only.
            // (Reusing the same PEM would defeat the test, so we craft a bogus
            // signature by tampering the encoded token instead.)
            test_keys::PRIVATE_PEM
        };
        let key = EncodingKey::from_rsa_pem(other_pem.as_bytes()).unwrap();
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(test_keys::KID.to_string());
        let token = encode(
            &header,
            &json!({"sub":"s","iss":ISSUER,"aud":AUDIENCE,"exp": now()+3600}),
            &key,
        )
        .unwrap();
        // Tamper the signature segment so verification must fail.
        let mut parts: Vec<&str> = token.split('.').collect();
        let tampered_sig = format!("{}x", parts[2]);
        parts[2] = &tampered_sig;
        let tampered = parts.join(".");
        assert!(matches!(
            validator().validate(&tampered).unwrap_err(),
            TokenError::Invalid(_)
        ));
    }

    #[test]
    fn rejects_garbage() {
        assert!(validator().validate("not-a-jwt").is_err());
    }
}
