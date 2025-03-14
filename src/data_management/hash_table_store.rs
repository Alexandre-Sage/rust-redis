use std::{
    collections::HashMap,
    time::{self, Duration, SystemTime},
};

use super::datastore::{DataStore, DataStoreEntry};
#[derive(Debug, Default)]
pub struct HashTableDataStore(HashMap<Vec<u8>, DataStoreEntry>);

impl DataStore for HashTableDataStore {
    fn insert(&mut self, key: Vec<u8>, data: Vec<u8>, expiry: Option<Duration>) {
        let new_entry = DataStoreEntry::new(data, expiry);
        self.0.insert(key, new_entry);
    }

    fn get(&mut self, key: Vec<u8>) -> Option<Vec<u8>> {
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

    use crate::data_management::{
        datastore::{DataStore, DataStoreEntry},
        hash_table_store::HashTableDataStore,
    };

    #[test]
    fn should_delete_expired_data() {
        let entry_with_expiry = DataStoreEntry::new(vec![], Some(Duration::from_millis(1)));
        let mut store = HashTableDataStore(HashMap::from([(b"hello".to_vec(), entry_with_expiry)]));
        std::thread::sleep(Duration::from_millis(2));
        assert!(store.get(b"hello".to_vec()).is_none())
    }

    #[test]
    fn should_retrieve_data() {
        let entry_with_expiry = DataStoreEntry::new(vec![], Some(Duration::from_millis(10000)));
        let mut store = HashTableDataStore(HashMap::from([(b"hello".to_vec(), entry_with_expiry)]));
        assert!(store.get(b"hello".to_vec()).is_some())
    }
}
