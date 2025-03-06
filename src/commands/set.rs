use async_trait::async_trait;
use futures::TryFutureExt;
use tokio::sync::mpsc;

use crate::{
    data_management::message::{
        DataChannelMessage, MessageChannelError, ResponseChannelMessage, SetMessage,
    },
    errors::RustRedisError,
    resp::Resp,
};

use super::command_registry::CommandHandler;

pub const SET_COMMAND_NAME: &str = "SET";

#[derive(Debug)]
pub struct SetCommandHandler {
    data_sender: mpsc::Sender<DataChannelMessage>,
    args: String,
}

impl SetCommandHandler {
    pub fn new(data_sender: mpsc::Sender<DataChannelMessage>) -> Self {
        Self {
            data_sender,
            args: "2".to_owned(),
        }
    }
}

#[async_trait]
impl CommandHandler for SetCommandHandler {
    async fn handle(
        &self,
        args: &[crate::resp::Resp],
    ) -> Result<crate::resp::Resp, crate::errors::RustRedisError> {
        if args.len() < 2 {
            return Err(RustRedisError::InvalidArgLength(
                SET_COMMAND_NAME.to_owned(),
                args.len().to_string(),
                self.args.to_owned(),
            ));
        }

        let key = args[0].to_owned();
        let value = args[1].to_owned();
        if !(key.is_bulk_string() && value.is_bulk_string()) {
            return Err(RustRedisError::InvalidArgType("bulk string".to_owned()));
        }

        let (sender, receiver) = tokio::sync::oneshot::channel();
        let message = SetMessage::new(key.serialize()?, value.serialize()?, sender);
        self.data_sender
            .send(DataChannelMessage::Set(message))
            .map_err(MessageChannelError::from)
            .await?;

        let reply = receiver.await.map_err(MessageChannelError::from)?;
        Ok(reply.0)
    }
}
