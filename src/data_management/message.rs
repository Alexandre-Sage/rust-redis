use crate::resp::Resp;

#[derive(Debug, thiserror::Error)]
pub enum MessageChannelError {
    #[error(transparent)]
    DataSending(#[from] tokio::sync::mpsc::error::SendError<DataChannelMessage>),
    #[error(transparent)]
    DataReplying(#[from] tokio::sync::oneshot::error::RecvError),
}

pub struct SetMessage {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub sender: tokio::sync::oneshot::Sender<ResponseChannelMessage>,
}

impl SetMessage {
    pub fn new(
        key: Vec<u8>,
        value: Vec<u8>,
        sender: tokio::sync::oneshot::Sender<ResponseChannelMessage>,
    ) -> Self {
        Self { key, value, sender }
    }
}
pub enum DataChannelMessage {
    Set(SetMessage),
}

pub struct ResponseChannelMessage(pub(crate) Resp);
