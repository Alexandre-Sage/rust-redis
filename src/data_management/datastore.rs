use std::{
    collections::HashMap,
    time::{self, Duration, SystemTime},
};

use crate::ternary_expr;

#[derive(Debug)]
struct DataStoreEntry {
    data: Vec<u8>,
    expiry: Option<SystemTime>,
}

impl DataStoreEntry {
    fn new(data: Vec<u8>, expiry: Option<Duration>) -> Self {
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
#[derive(Debug, Default)]
pub struct DataStore(HashMap<Vec<u8>, DataStoreEntry>);

impl DataStore {
    pub fn insert(&mut self, key: Vec<u8>, data: Vec<u8>, expiry: Option<Duration>) {
        let new_entry = DataStoreEntry::new(data, expiry);
        self.0.insert(key, new_entry);
    }

    pub fn get(&mut self, key: Vec<u8>) -> Option<Vec<u8>> {
        match self.0.get(&key) {
            Some(entry) => {
                if entry.expired() {
                    self.0.remove(&key);
                    None
                } else {
                    Some(entry.data.to_owned())
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, time::Duration};

    use super::{DataStore, DataStoreEntry};

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

    #[test]
    fn should_delete_expired_data() {
        let entry_with_expiry = DataStoreEntry::new(vec![], Some(Duration::from_millis(1)));
        let mut store = DataStore(HashMap::from([(b"hello".to_vec(), entry_with_expiry)]));
        std::thread::sleep(Duration::from_millis(2));
        assert!(store.get(b"hello".to_vec()).is_none())
    }

    #[test]
    fn should_retrieve_data() {
        let entry_with_expiry = DataStoreEntry::new(vec![], Some(Duration::from_millis(10000)));
        let mut store = DataStore(HashMap::from([(b"hello".to_vec(), entry_with_expiry)]));
        assert!(store.get(b"hello".to_vec()).is_some())
    }
}
