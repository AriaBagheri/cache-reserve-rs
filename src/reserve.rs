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
