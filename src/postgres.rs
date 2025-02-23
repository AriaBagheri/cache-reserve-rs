use crate::CacheReserve;
use persistent_postgres::{PgChangeAction, PgChangeNotification};
use std::hash::Hash;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use colored::Colorize;

impl<PK, T> CacheReserve<PK, T>
where
    PK: From<T> + Hash + Eq + Copy + Send + Sync,
    T: Clone + Send + Sync + serde::de::DeserializeOwned,
{
    pub fn listener(&'static self, notification: PgChangeNotification, mut shutdown: Receiver<()>) -> JoinHandle<()> {
        tokio::spawn(async move {
            // Short circuit. If the record was just inserted, it clearly is not cached yet!
            if let PgChangeAction::Insert = notification.action {
                return
            }
            let record: T = match serde_json::from_value(notification.payload) {
                Ok(v) => {
                    v
                },
                Err(e) => {
                    println!(
                        "{} {}",
                        "CACHE_RESERVE - PG_LISTENER - \
                                Failed to deserialize change notification".red(),
                        format!("| error = {}", e.to_string())
                            .red()
                            .dimmed()
                    );
                    return
                }
            };
            match notification.action {
                PgChangeAction::Update => {
                    tokio::select! {
                        _ = shutdown.recv() => {}
                        _ = self.update(PK::from(record.clone()), record) => {}
                    }
                }
                PgChangeAction::Delete => {
                    tokio::select! {
                        _ = shutdown.recv() => {}
                        _ = self.ignore(PK::from(record)) => {}
                    }
                }
                PgChangeAction::Insert => {
                    // unreachable due to initial short-circuiting of the insert variation
                    unreachable!();
                }
            };
        })
    }
}
