use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct DataStoreEntry {
    pub data: Vec<u8>,
    expiry: Option<SystemTime>,
}

impl DataStoreEntry {
    pub fn new(data: Vec<u8>, expiry: Option<Duration>) -> Self {
        Self {
            data,
            expiry: expiry.map(|duration| SystemTime::now() + duration),
        }
    }

    pub fn expired(&self) -> bool {
        if let Some(expiry) = self.expiry {
            let now = SystemTime::now();
            return now >= expiry;
        }
        return false;
    }
}

pub trait DataStore: Send + Sync + Default + 'static {
    fn insert(&mut self, key: Vec<u8>, data: Vec<u8>, expiry: Option<Duration>);
    fn get(&mut self, key: Vec<u8>) -> Option<Vec<u8>>;
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, time::Duration};

    use crate::data_management::datastore::DataStore;

    use super::DataStoreEntry;

    #[test]
    fn should_be_expired() {
        let entry_with_expiry = DataStoreEntry::new(vec![], Some(Duration::from_millis(1)));
        std::thread::sleep(Duration::from_millis(2));
        assert!(entry_with_expiry.expired())
    }

    #[test]
    fn should_no_be_expired() {
        let entry_with_expiry = DataStoreEntry::new(vec![], Some(Duration::from_millis(100000)));
        assert!(!entry_with_expiry.expired())
    }
}
