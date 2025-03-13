use std::collections::HashMap;

use tokio::{sync::mpsc, task::JoinHandle};

use crate::{errors::RustRedisError, resp::Resp};

use super::message::{DataChannelMessage, ResponseChannelMessage};

pub fn data_management_worker_thread(
    mut data_receiver: mpsc::Receiver<DataChannelMessage>,
    default_value: Option<HashMap<Vec<u8>, Vec<u8>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut data_store = match default_value {
            None => HashMap::<Vec<u8>, Vec<u8>>::new(),
            Some(data) => data,
        };
        while let Some(message) = data_receiver.recv().await {
            match message {
                DataChannelMessage::Set(message) => {
                    data_store.insert(message.key, message.value);
                    match message
                        .sender
                        .send(ResponseChannelMessage(Resp::simple_string_from_str("OK")))
                    {
                        Ok(_) => (),
                        Err(err) => log::error!("Could not reply: {:?}", err.0),
                    }
                }
                DataChannelMessage::Get(message) => {
                    let response = match data_store.get(&message.key) {
                        Some(data) => Resp::deserialize(data),
                        None => Ok(Resp::bulk_string_from_str("")),
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

#[cfg(test)]
mod test {

    use crate::data_management::message::{GetMessage, SetMessage};

    use super::*;
    #[tokio::test]
    async fn should_insert_key_value() {
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        data_management_worker_thread(data_receiver, None);
        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let value = Resp::bulk_string_from_str("world").serialize().unwrap();
        let message = SetMessage::new(key, value, response_sender);
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
        let default = [(key.clone(), value.clone())].into_iter().collect();

        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        data_management_worker_thread(data_receiver, Some(default));
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
        data_management_worker_thread(data_receiver, None);
        let message = GetMessage::new(key, response_sender);
        data_sender
            .send(DataChannelMessage::Get(message))
            .await
            .unwrap();
        let res = response_receiver.await.unwrap();
        assert_eq!(res.0.serialize().unwrap(), EXPECT.as_bytes())
    }
}
