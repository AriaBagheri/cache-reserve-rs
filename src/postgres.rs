use crate::CacheReserve;
use persistent_postgres::{PgChangeAction, PgChangeNotification};
use std::hash::Hash;

impl<PK, T> CacheReserve<PK, T>
where
    PK: From<T> + Hash + Eq + Copy,
{
    pub fn listener(&self, notification: PgChangeNotification) {
        match notification.action {
            PgChangeAction::Update => {}
            PgChangeAction::Delete => {}
            PgChangeAction::Insert => {}
        }
    }
}
