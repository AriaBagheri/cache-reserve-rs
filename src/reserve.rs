use std::collections::HashMap;
use std::hash::Hash;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use tokio::task::JoinHandle;
use colored::Colorize;

pub struct CacheReserve<PK, T>
where
    PK: Eq + Hash,
{
    size: usize,
    storage: LazyLock<RwLock<HashMap<PK, T>>>,

    monitoring_handle: Mutex<Option<JoinHandle<()>>>,
    shutdown: LazyLock<Sender<()>>
}

pub trait Fetchable<T> {
    fn fetch(
        &self,
    ) -> impl Future<Output = Result<Option<T>, Box<dyn std::error::Error>>> + Send;
}

impl<PK, T> CacheReserve<PK, T>
where
    PK: Eq + Hash + Copy + Send + Sync,
    T: Send + Sync
{
    pub const fn const_new(size: usize) -> Self {
        Self {
            size,
            storage: LazyLock::new(|| RwLock::new(HashMap::new())),

            monitoring_handle: Mutex::const_new(None),
            shutdown: LazyLock::new(|| Sender::new(1)),
        }
    }

    pub async fn initiate(&'static self) {
        *self.monitoring_handle.lock().await = Some(self.monitoring_process());
    }

    pub fn monitoring_process(&'static self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut shutdown = self.shutdown.subscribe();
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                _ = shutdown.recv() => {
                    break;
                },
                _ = interval.tick() => {
                    let size_of_storage = self.storage.read().await.len();
                    if size_of_storage > self.size {
                        self.random_removal(size_of_storage - self.size).await;
                    }
                }
            }
            }
        })
    }

    pub async fn random_removal(&self, mut count: usize) {
        let mut x = self.storage.write().await;
        x.retain(|_, _| {
            if count > 0 {
                count = count - 1;
                false
            }  else {
                true
            }
        });
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
        PK: Fetchable<T>,
    {
        if !self.storage.read().await.contains_key(&pk) {
            if let Some(value) = pk.fetch().await? {
                self.set(pk, value).await;
            }
        }
        Ok(self.get_from_storage(&pk).await)
    }

    pub async fn ignore(&self, pk: PK) -> Option<T> {
        self.storage.write().await.remove(&pk)
    }

    pub async fn shutdown(&self) {
        print!("\n");

        let _ = self.shutdown.send(());
        println!(
            "{}",
            "CACHE_RESERVE - SHUTDOWN - Shutdown signal was propagated to internal threads!".cyan()
        );

        if let Some(monitor_thread) = self.monitoring_handle.lock().await.as_mut() {
            let _ = tokio::time::timeout(Duration::from_secs(5), &mut *monitor_thread)
                .await
                .map(|_| {
                    println!(
                        "{}",
                        "CACHE_RESERVE - SHUTDOWN - Monitoring thread terminated gracefully!".cyan()
                    );
                })
                .map_err(|_| {
                    println!(
                        "{} {}",
                        "CACHE_RESERVE - SHUTDOWN - Monitoring thread terminated".cyan(),
                        "forcefully".bold().red()
                    );
                    monitor_thread.abort();
                });
        }
        self.storage.write().await.clear();
        println!("{}", "CACHE_RESERVE - SHUTDOWN - Goodbye!".cyan())
    }
}
