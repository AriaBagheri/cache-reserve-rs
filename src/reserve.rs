use std::collections::HashMap;
use std::hash::Hash;
use std::sync::LazyLock;
use tokio::sync::{RwLock, RwLockReadGuard};

pub struct CacheReserve<PK, T>
where
    PK: Eq + Hash,
{
    storage: LazyLock<RwLock<HashMap<PK, T>>>
}

pub trait Fetchable {
    async fn fetch<T>(&self) -> Result<Option<T>, Box<dyn std::error::Error>>;
}

impl<PK, T> CacheReserve<PK, T>
where
    PK: Eq + Hash + Fetchable + Copy,
{
    pub fn const_new() -> Self {
        Self {
            storage: LazyLock::new(|| RwLock::new(HashMap::new()))
        }
    }

    pub async fn set(&self, key: PK, value: T) {
        self.storage.write().await.insert(key, value);
    }

            if let Some(value) = pk.fetch::<T>().await? {
}