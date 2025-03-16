use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

use crate::{errors::RustRedisError, resp::Resp};

use super::{
    datastore::DataStore,
    message::{DataChannelMessage, ResponseChannelMessage},
};

#[derive(Debug)]
pub struct DataManager<T>
where
    T: DataStore,
{
    data_store: T,
    data_receiver: mpsc::Receiver<DataChannelMessage>,
    cleanup_intervall: Arc<Duration>,
}

impl<T> DataManager<T>
where
    T: DataStore,
{
    pub fn new(
        data_receiver: mpsc::Receiver<DataChannelMessage>,
        data_store: Option<T>,
        cleanup_intervall: Option<Duration>,
    ) -> Self {
        Self {
            data_store: match data_store {
                Some(data) => data,
                None => Default::default(),
            },
            data_receiver,
            cleanup_intervall: match cleanup_intervall {
                Some(duration) => duration,
                None => Duration::from_secs(60 * 60),
            }
            .into(),
        }
    }

    fn worker(
        data_receiver: mpsc::Receiver<DataChannelMessage>,
        data_store: Option<T>,
        cleanup_intervall: Option<Duration>,
    ) -> JoinHandle<()> {
        let manager = Self::new(data_receiver, data_store, cleanup_intervall);
        manager.run()
    }
    //pub fn cleanup(&mut self) -> JoinHandle<()> {
    //    let cleanup_intervall = self.cleanup_intervall.clone();
    //    let store = self.data_store.clone();
    //    tokio::spawn(async move {
    //        loop {
    //            store.lock().await.clean();
    //            tokio::time::sleep(*cleanup_intervall).await;
    //        }
    //    })
    //}

    pub fn run(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(message) = self.data_receiver.recv().await {
                match message {
                    DataChannelMessage::Set(message) => {
                        self.data_store.insert(message.key, message.value, None);

                        match message
                            .sender
                            .send(ResponseChannelMessage(Resp::simple_string_from_str("OK")))
                        {
                            Ok(_) => (),
                            Err(err) => log::error!("Could not reply: {:?}", err.0),
                        }
                    }
                    DataChannelMessage::Get(message) => {
                        let response = match self.data_store.get(message.key) {
                            Some(data) => Resp::deserialize(&data),
                            None => Ok(Resp::null_bulk_string()),
                        };

                        let response = match response {
                            Ok(data) => data,
                            Err(err) => RustRedisError::from(err).into(),
                        };

                        message
                            .sender
                            .send(ResponseChannelMessage(response))
                            .unwrap();
                    }
                    _ => todo!(),
                }
            }
        })
    }
}

#[cfg(test)]
mod test {

    use std::time::Duration;

    use super::*;
    use crate::data_management::{
        datastore::DataStoreEntry,
        hash_table_store::HashTableDataStore,
        message::{GetMessage, SetMessage},
    };

    #[tokio::test]
    async fn should_insert_key_value() {
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        DataManager::<HashTableDataStore>::worker(data_receiver, None, None);

        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let value = Resp::bulk_string_from_str("world").serialize().unwrap();

        let message = SetMessage::new(key, value, response_sender, None);
        data_sender
            .send(DataChannelMessage::Set(message))
            .await
            .unwrap();

        let res = response_receiver.await.unwrap();
        let expect = Resp::simple_string_from_str("OK");
        assert_eq!(res.0, expect)
    }

    #[tokio::test]
    async fn should_retrieve_data() {
        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let value = Resp::bulk_string_from_str("world").serialize().unwrap();
        let entry = DataStoreEntry::new(value.clone(), None);
        let default = HashTableDataStore::from([(key.clone(), entry)]);

        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        DataManager::worker(data_receiver, Some(default), None);
        let message = GetMessage::new(key, response_sender);
        data_sender
            .send(DataChannelMessage::Get(message))
            .await
            .unwrap();

        let res = response_receiver.await.unwrap();
        assert_eq!(res.0, Resp::deserialize(&value).unwrap())
    }

    #[tokio::test]
    async fn should_reply_null_bulk_string_if_no_data() {
        const EXPECT: &str = "$-1\r\n";
        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        DataManager::<HashTableDataStore>::worker(data_receiver, None, None);
        let message = GetMessage::new(key, response_sender);
        data_sender
            .send(DataChannelMessage::Get(message))
            .await
            .unwrap();

        let res = response_receiver.await.unwrap();
        assert_eq!(res.0.serialize().unwrap(), EXPECT.as_bytes())
    }

    #[tokio::test]
    async fn should_reply_null_bulk_string_if_expired_data() {
        const EXPECT: &str = "$-1\r\n";
        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let value = Resp::bulk_string_from_str("world").serialize().unwrap();
        let expiry = Duration::from_millis(1);
        let entry = DataStoreEntry::new(value, Some(expiry));
        let data_store = HashTableDataStore::from([(key.clone(), entry)]);
        tokio::time::sleep(Duration::from_millis(2)).await;
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        DataManager::worker(data_receiver, Some(data_store), None);
        let message = GetMessage::new(key, response_sender);
        data_sender
            .send(DataChannelMessage::Get(message))
            .await
            .unwrap();

        let res = response_receiver.await.unwrap();
        assert_eq!(res.0.serialize().unwrap(), EXPECT.as_bytes())
    }
}
