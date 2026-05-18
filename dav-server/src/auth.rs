//! Clerk JWT validation for the local WebDAV server.
//!
//! This validates access tokens locally against Clerk's JWKS. Clerk access
//! tokens currently only give us `sub`, so the server treats that as the user
//! namespace.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use hyper::header;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

const CLERK_ISSUER: &str = "https://settled-hamster-79.clerk.accounts.dev";
const JWKS_URL: &str = "https://settled-hamster-79.clerk.accounts.dev/.well-known/jwks.json";

#[derive(Clone)]
pub struct AuthVerifier {
    keys: HashMap<String, DecodingKey>,
    log_raw_jwt: bool,
}

#[derive(Debug, Clone)]
pub struct VerifiedUser {
    pub subject: String,
    pub user_dir: String,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
    #[serde(default)]
    kty: Option<String>,
    #[serde(default)]
    alg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    iss: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl AuthVerifier {
    pub async fn fetch(log_raw_jwt: bool) -> Result<Self> {
        let jwks = reqwest::get(JWKS_URL)
            .await
            .context("fetch Clerk JWKS")?
            .error_for_status()
            .context("Clerk JWKS status")?
            .json::<Jwks>()
            .await
            .context("decode Clerk JWKS")?;
        let mut keys = HashMap::new();
        for jwk in jwks.keys {
            if jwk.kty.as_deref() != Some("RSA") {
                continue;
            }
            if jwk.alg.as_deref().is_some_and(|alg| alg != "RS256") {
                continue;
            }
            let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                .with_context(|| format!("build decoding key {}", jwk.kid))?;
            keys.insert(jwk.kid, key);
        }
        if keys.is_empty() {
            bail!("Clerk JWKS did not contain any RSA signing keys");
        }
        tracing::info!(key_count = keys.len(), "loaded Clerk JWKS");
        Ok(Self { keys, log_raw_jwt })
    }

    pub fn verify_header(
        &self,
        headers: &hyper::HeaderMap,
        method: &hyper::Method,
        path: &str,
    ) -> Result<VerifiedUser> {
        let token = bearer_token(headers)?;
        if self.log_raw_jwt {
            tracing::warn!(jwt = %token, "raw Clerk JWT logging enabled");
        }
        self.verify_token(token, method, path)
    }

    fn verify_token(
        &self,
        token: &str,
        method: &hyper::Method,
        path: &str,
    ) -> Result<VerifiedUser> {
        let header = decode_header(token).context("decode JWT header")?;
        let kid = header.kid.context("JWT is missing kid")?;
        let key = self
            .keys
            .get(&kid)
            .with_context(|| format!("unknown JWT kid: {kid}"))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[CLERK_ISSUER]);
        validation.validate_aud = false;
        let token_data = decode::<Claims>(token, key, &validation).context("verify JWT")?;
        let claims = token_data.claims;
        let user_dir = user_dir(&claims.sub);
        tracing::info!(
            %method,
            path,
            subject = %claims.sub,
            user_dir,
            issuer = ?claims.iss,
            extra_claim_keys = ?claim_keys(&claims.extra),
            "verified Clerk JWT for user namespace"
        );
        tracing::debug!(claims = ?claims, "decoded Clerk JWT claims");
        Ok(VerifiedUser {
            subject: claims.sub,
            user_dir,
        })
    }
}

fn bearer_token(headers: &hyper::HeaderMap) -> Result<&str> {
    let value = headers
        .get(header::AUTHORIZATION)
        .context("missing Authorization header")?
        .to_str()
        .context("Authorization header is not UTF-8")?;
    value
        .strip_prefix("Bearer ")
        .context("Authorization header is not Bearer")
}

fn user_dir(subject: &str) -> String {
    subject
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                return ch;
            }
            '_'
        })
        .collect()
}

fn claim_keys(extra: &HashMap<String, serde_json::Value>) -> Vec<&str> {
    let mut keys = extra.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}
