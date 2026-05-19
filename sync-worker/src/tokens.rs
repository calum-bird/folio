use anyhow::{Context, Result};
use aws_sdk_kms::Client as Kms;
use base64::Engine;
use foliofs_connectors::model::{ConnectionRecord, ProviderTokenSecret};

use crate::token_cache::TokenCache;

pub struct TokenLoader {
    kms: Kms,
    cache: TokenCache,
}

impl TokenLoader {
    pub fn new(kms: Kms, cache_ttl_secs: u64) -> Self {
        Self {
            kms,
            cache: TokenCache::new(cache_ttl_secs),
        }
    }

    pub async fn load(&mut self, connection: &ConnectionRecord) -> Result<ProviderTokenSecret> {
        let cache_key = token_cache_key(connection);
        if let Some(secret) = self.cache.get(&cache_key) {
            return Ok(secret);
        }

        let secret = decrypt_token(
            &self.kms,
            &connection.encrypted_token,
            &connection.pk,
            &connection.sk,
        )
        .await?;
        self.cache.insert(cache_key, secret.clone());
        Ok(secret)
    }
}

fn token_cache_key(connection: &ConnectionRecord) -> String {
    format!("{}#{}", connection.pk, connection.sk)
}

async fn decrypt_token(
    kms: &Kms,
    encrypted_token: &str,
    pk: &str,
    sk: &str,
) -> Result<ProviderTokenSecret> {
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(encrypted_token.trim())
        .context("decode encrypted token")?;

    let response = kms
        .decrypt()
        .ciphertext_blob(ciphertext.into())
        .encryption_context("pk", pk)
        .encryption_context("sk", sk)
        .send()
        .await
        .context("decrypt connection token")?;

    let plaintext = response.plaintext().context("decrypt returned no plaintext")?;
    serde_json::from_slice(plaintext.as_ref()).context("decode provider token JSON")
}
