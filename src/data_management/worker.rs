use std::collections::HashMap;

use tokio::{sync::mpsc, task::JoinHandle};

use crate::resp::Resp;

use super::message::{DataChannelMessage, ResponseChannelMessage};

pub fn data_management_worker_thread(
    mut data_receiver: mpsc::Receiver<DataChannelMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut data_store = HashMap::<Vec<u8>, Vec<u8>>::new();
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
            }
        }
    })
}

#[cfg(test)]
mod test {
    use message::SetMessage;

    use super::*;
    #[tokio::test]
    async fn should_insert_key_value() {
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        data_management_worker_thread(data_receiver);
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
}
