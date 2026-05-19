use std::collections::HashMap;
use std::time::{Duration, Instant};

use foliofs_connectors::model::ProviderTokenSecret;

struct CacheEntry {
    secret: ProviderTokenSecret,
    expires_at: Instant,
}

pub struct TokenCache {
    ttl: Duration,
    entries: HashMap<String, CacheEntry>,
}

impl TokenCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            entries: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<ProviderTokenSecret> {
        let now = Instant::now();
        let entry = self.entries.get(key)?;
        if entry.expires_at <= now {
            self.entries.remove(key);
            return None;
        }

        let secret = entry.secret.clone();
        if let Some(entry) = self.entries.get_mut(key) {
            entry.expires_at = now + self.ttl;
        }

        Some(secret)
    }

    pub fn insert(&mut self, key: String, secret: ProviderTokenSecret) {
        self.entries.insert(
            key,
            CacheEntry {
                secret,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }
}
