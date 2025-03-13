use std::sync::Arc;

use async_trait::async_trait;
use futures::TryFutureExt;
use tokio::sync::mpsc::Sender;

use crate::{
    data_management::message::{DataChannelMessage, GetMessage, MessageChannelError},
    errors::RustRedisError,
    resp::Resp,
};

use super::command_registry::CommandHandler;

pub const GET_COMMAND_NAME: &str = "GET";

#[derive(Debug)]
pub struct GetCommandHandler {
    data_sender: Arc<Sender<DataChannelMessage>>,
}

impl GetCommandHandler {
    pub fn new(sender: Arc<Sender<DataChannelMessage>>) -> Self {
        Self {
            data_sender: sender,
        }
    }
}

#[async_trait]
impl CommandHandler for GetCommandHandler {
    async fn handle(&self, args: &[Resp]) -> Result<Resp, RustRedisError> {
        if args.len() < 1 {
            return Err(RustRedisError::InvalidArgLength(
                GET_COMMAND_NAME.to_owned(),
                "1".to_owned(),
                args.len().to_string(),
            ));
        }

        let (sender, receiver) = tokio::sync::oneshot::channel();
        let message = GetMessage::new(args[0].to_owned().serialize()?, sender);

        self.data_sender
            .send(DataChannelMessage::Get(message))
            .map_err(MessageChannelError::from)
            .await?;

        let reply = receiver.await.map_err(MessageChannelError::from)?;
        Ok(reply.0)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        commands::{command_registry::CommandHandler, get::GET_COMMAND_NAME},
        data_management::message::{DataChannelMessage, ResponseChannelMessage},
        errors::RustRedisError,
        resp::Resp,
    };

    use super::GetCommandHandler;

    #[tokio::test]
    async fn should_retrieve_data() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1000);
        let handler = GetCommandHandler::new(sender.into());

        tokio::spawn(async move {
            if let Some(message) = receiver.recv().await {
                match message {
                    DataChannelMessage::Get(message) => message
                        .sender
                        .send(ResponseChannelMessage(Resp::bulk_string_from_str("world")))
                        .unwrap(),
                    _ => panic!(),
                }
            };
        });

        let result = handler.handle(&[Resp::bulk_string_from_str("HELLO")]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Resp::bulk_string_from_str("world"));
    }

    #[tokio::test]
    async fn should_throw_error_if_not_enough_args() {
        let (sender, _) = tokio::sync::mpsc::channel(1000);
        let handler = GetCommandHandler::new(sender.into());

        let result = handler.handle(&[]).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            RustRedisError::InvalidArgLength(
                GET_COMMAND_NAME.to_owned(),
                "1".to_owned(),
                "0".to_owned(),
            )
            .to_string()
        );
    }
}
