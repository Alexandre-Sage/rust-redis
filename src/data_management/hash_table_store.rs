use std::{collections::HashMap, time::Duration};

use super::datastore::{DataStore, DataStoreEntry};
#[derive(Debug, Default)]
pub struct HashTableDataStore(HashMap<Vec<u8>, DataStoreEntry>);

impl<I> From<I> for HashTableDataStore
where
    I: Into<HashMap<Vec<u8>, DataStoreEntry>>, //I: IntoIterator<Item = (K, V)> + FromIterator<(K, V)>,
                                               //K: Hash,
{
    fn from(value: I) -> Self {
        Self(value.into())
    }
}

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
    fn clean(&mut self) -> () {
        self.0.retain(|_, v| !v.expired());
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

    #[test]
    fn should_clean_expired() {
        let entry_expired = DataStoreEntry::new(vec![], Some(Duration::from_millis(1)));
        let entry_not_expired =
            DataStoreEntry::new(b"world".to_vec(), Some(Duration::from_millis(10000)));
        let mut store = HashTableDataStore::from([
            (b"hello".to_vec(), entry_not_expired.clone()),
            (b"expired".to_vec(), entry_expired),
        ]);
        store.clean();
        assert_eq!(
            store.get(b"hello".to_vec()).unwrap(),
            entry_not_expired.data
        )
    }
}
