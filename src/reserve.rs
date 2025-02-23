use std::collections::HashMap;
use std::hash::Hash;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast::Sender;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use tokio::task::JoinHandle;

pub struct CacheReserve<PK, T>
where
    PK: Eq + Hash,
{
    size: usize,
    storage: LazyLock<RwLock<HashMap<PK, T>>>,

    monitoring_handle: Mutex<Option<JoinHandle<()>>>,
    shutdown: LazyLock<Sender<()>>
}

pub trait Fetchable {
    fn fetch<T>(
        &self,
    ) -> impl Future<Output = Result<Option<T>, Box<dyn std::error::Error>>> + Send;
}

impl<PK, T> CacheReserve<PK, T>
where
    PK: Eq + Hash + Copy,
{
    pub fn const_new(size: usize) -> Self {
        Self {
            size,
            storage: LazyLock::new(|| RwLock::new(HashMap::new())),

            monitoring_handle: Mutex::new(None),
            shutdown: LazyLock::new(|| Sender::new(1)),
        }
    }

    pub async fn set(&self, key: PK, value: T) {
        self.storage.write().await.insert(key, value);
    }

    pub async fn update(&self, key: PK, value: T) {
        let mut storage = self.storage.write().await;
        if let Some(x) = storage.get_mut(&key) {
            *x = value;
        }
    }

    async fn get_from_storage(&self, pk: &PK) -> Option<RwLockReadGuard<T>> {
        let guard = self.storage.read().await;

        if guard.contains_key(pk) {
            Some(RwLockReadGuard::map(guard, |v| v.get(&pk).unwrap()))
        } else {
            None
        }
    }

    pub async fn get_with(
        &self,
        pk: PK,
    ) -> Result<Option<RwLockReadGuard<T>>, Box<dyn std::error::Error>>
    where
        PK: Fetchable,
    {
        if !self.storage.read().await.contains_key(&pk) {
            if let Some(value) = pk.fetch::<T>().await? {
                self.set(pk, value).await;
            }
        }
        Ok(self.get_from_storage(&pk).await)
    }

    pub async fn ignore(&self, pk: PK) -> Option<T> {
        self.storage.write().await.remove(&pk)
    }
}
