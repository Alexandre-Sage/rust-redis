use std::time::Duration;

use crate::resp::Resp;

#[derive(Debug, thiserror::Error)]
pub enum MessageChannelError {
    #[error(transparent)]
    DataSending(#[from] tokio::sync::mpsc::error::SendError<DataChannelMessage>),
    #[error(transparent)]
    DataReplying(#[from] tokio::sync::oneshot::error::RecvError),
}

#[derive(Debug)]
pub struct SetMessage {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub sender: tokio::sync::oneshot::Sender<ResponseChannelMessage>,
    pub expiry: Option<Duration>,
}

impl SetMessage {
    pub fn new(
        key: Vec<u8>,
        value: Vec<u8>,
        sender: tokio::sync::oneshot::Sender<ResponseChannelMessage>,
        expiry: Option<Duration>,
    ) -> Self {
        Self {
            key,
            value,
            sender,
            expiry,
        }
    }
}

#[derive(Debug)]
pub struct GetMessage {
    pub key: Vec<u8>,
    pub sender: tokio::sync::oneshot::Sender<ResponseChannelMessage>,
}

impl GetMessage {
    pub fn new(key: Vec<u8>, sender: tokio::sync::oneshot::Sender<ResponseChannelMessage>) -> Self {
        Self { key, sender }
    }
}

#[derive(Debug)]
pub enum DataChannelMessage {
    Set(SetMessage),
    Get(GetMessage),
}

#[derive(Debug)]
pub struct ResponseChannelMessage(pub(crate) Resp);
