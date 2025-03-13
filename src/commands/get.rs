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
    data_sender: Sender<DataChannelMessage>,
}

impl GetCommandHandler {
    pub fn new(sender: Sender<DataChannelMessage>) -> Self {
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
        commands::command_registry::CommandHandler,
        data_management::message::{DataChannelMessage, ResponseChannelMessage},
        resp::Resp,
    };

    use super::GetCommandHandler;

    #[tokio::test]
    async fn should_insert_data() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1000);
        let handler = GetCommandHandler::new(sender);

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
}
