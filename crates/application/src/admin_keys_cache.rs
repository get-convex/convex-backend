use std::{
    collections::HashMap,
    sync::{
        Arc,
        RwLock,
    },
};

use keybroker::AdminKeyHash;
use sync_types::Timestamp;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedAdminKey {
    pub doc_id: String,
    pub name: String,
    pub revoked_time: Option<Timestamp>,
    pub key_suffix: Option<String>,
}

#[derive(Debug)]
pub enum AdminKeyCheck {
    Valid,
    Revoked(Timestamp),
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub struct AdminKeysCache {
    inner: Arc<RwLock<HashMap<AdminKeyHash, CachedAdminKey>>>,
}

impl AdminKeysCache {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn load(entries: impl IntoIterator<Item = (AdminKeyHash, CachedAdminKey)>) -> Self {
        let map = entries.into_iter().collect();
        Self {
            inner: Arc::new(RwLock::new(map)),
        }
    }

    pub fn check(&self, hash: &AdminKeyHash) -> AdminKeyCheck {
        match self.inner.read().unwrap().get(hash) {
            None => AdminKeyCheck::Unknown,
            Some(c) => match c.revoked_time {
                Some(t) => AdminKeyCheck::Revoked(t),
                None => AdminKeyCheck::Valid,
            },
        }
    }

    pub fn insert(&self, hash: AdminKeyHash, entry: CachedAdminKey) {
        self.inner.write().unwrap().insert(hash, entry);
    }

    pub fn mark_revoked(&self, hash: &AdminKeyHash, at: Timestamp) {
        if let Some(entry) = self.inner.write().unwrap().get_mut(hash) {
            entry.revoked_time = Some(at);
        }
    }

    pub fn rename(&self, hash: &AdminKeyHash, new_name: String) {
        if let Some(entry) = self.inner.write().unwrap().get_mut(hash) {
            entry.name = new_name;
        }
    }

    pub fn snapshot(&self) -> HashMap<AdminKeyHash, CachedAdminKey> {
        self.inner.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use keybroker::AdminKeyHash;
    use sync_types::Timestamp;

    use super::{
        AdminKeyCheck,
        AdminKeysCache,
        CachedAdminKey,
    };

    fn hash(b: u8) -> AdminKeyHash {
        AdminKeyHash([b; 32])
    }

    fn ts(nanos: i64) -> Timestamp {
        Timestamp::try_from(nanos).unwrap()
    }

    #[test]
    fn unknown_by_default() {
        let cache = AdminKeysCache::empty();
        assert!(matches!(cache.check(&hash(1)), AdminKeyCheck::Unknown));
    }

    #[test]
    fn insert_then_valid() {
        let cache = AdminKeysCache::empty();
        cache.insert(
            hash(2),
            CachedAdminKey {
                doc_id: "d1".into(),
                name: "k".into(),
                revoked_time: None,
                key_suffix: None,
            },
        );
        assert!(matches!(cache.check(&hash(2)), AdminKeyCheck::Valid));
    }

    #[test]
    fn revoke_then_rejected() {
        let cache = AdminKeysCache::empty();
        cache.insert(
            hash(3),
            CachedAdminKey {
                doc_id: "d1".into(),
                name: "k".into(),
                revoked_time: None,
                key_suffix: None,
            },
        );
        cache.mark_revoked(&hash(3), ts(1_000_000_000));
        assert!(matches!(cache.check(&hash(3)), AdminKeyCheck::Revoked(_)));
    }

    #[test]
    fn rename_preserves_revocation() {
        let cache = AdminKeysCache::empty();
        cache.insert(
            hash(4),
            CachedAdminKey {
                doc_id: "d1".into(),
                name: "old".into(),
                revoked_time: Some(ts(1)),
                key_suffix: None,
            },
        );
        cache.rename(&hash(4), "new".into());
        assert!(matches!(cache.check(&hash(4)), AdminKeyCheck::Revoked(_)));
    }
}
