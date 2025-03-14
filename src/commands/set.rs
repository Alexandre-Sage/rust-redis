use std::{fmt::format, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::TryFutureExt;
use tokio::sync::mpsc::Sender;

use crate::{
    data_management::message::{DataChannelMessage, MessageChannelError, SetMessage},
    errors::RustRedisError,
    resp::Resp,
};

use super::command_registry::CommandHandler;

pub const SET_COMMAND_NAME: &str = "SET";

#[derive(Debug)]
pub struct SetCommandHandler {
    data_sender: Arc<Sender<DataChannelMessage>>,
    args: String,
}

impl SetCommandHandler {
    pub fn new(data_sender: Arc<Sender<DataChannelMessage>>) -> Self {
        Self {
            data_sender,
            args: "2".to_owned(),
        }
    }

    fn check_args_len(&self, args_len: usize) -> Result<(), RustRedisError> {
        if args_len < 2 {
            return Err(RustRedisError::InvalidArgLength(
                SET_COMMAND_NAME.to_owned(),
                args_len.to_string(),
                self.args.to_owned(),
            ));
        }

        if args_len > 2 && args_len < 4 {
            return Err(RustRedisError::InvalidArgLength(
                format!("{SET_COMMAND_NAME} with EXPIRY"),
                args_len.to_string(),
                "4".to_owned(),
            ));
        }

        Ok(())
    }

    fn handle_args(&self, args: &[Resp]) -> Result<(Resp, Resp, Option<Duration>), RustRedisError> {
        let key = args[0].to_owned();
        let value = args[1].to_owned();

        if !(key.is_bulk_string() && value.is_bulk_string()) {
            return Err(RustRedisError::InvalidArgType("bulk string".to_owned()));
        }

        if args.len() > 2 {
            if let Resp::BulkString(px) = &args[2] {
                if px != b"PX" {
                    return Err(RustRedisError::InvalidArg(
                        SET_COMMAND_NAME.to_owned(),
                        "PX".to_owned(),
                        String::from_utf8(px.to_vec()).unwrap(),
                    ));
                }
            } else {
                return Err(RustRedisError::InvalidArgType("bulk string".to_owned()));
            }
            if let Resp::BulkString(duration) = &args[3] {
                let duration = std::str::from_utf8(&duration)?;
                let duration = duration.parse::<u64>()?;
                let duration = Duration::from_millis(duration);
                return Ok((key, value, Some(duration)));
            } else {
                return Err(RustRedisError::InvalidArgType("bulk string".to_owned()));
            };
        }
        Ok((key, value, None))
    }
}

#[async_trait]
impl CommandHandler for SetCommandHandler {
    async fn handle(
        &self,
        args: &[crate::resp::Resp],
    ) -> Result<crate::resp::Resp, crate::errors::RustRedisError> {
        self.check_args_len(args.len())?;

        let (key, value, duration) = self.handle_args(args)?;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let message = SetMessage::new(key.serialize()?, value.serialize()?, sender, duration);

        self.data_sender
            .send(DataChannelMessage::Set(message))
            .map_err(MessageChannelError::from)
            .await?;

        let reply = receiver.await.map_err(MessageChannelError::from)?;
        Ok(reply.0)
    }
}
#[cfg(test)]
mod test {
    use crate::{
        commands::{command_registry::CommandHandler, set::SET_COMMAND_NAME},
        data_management::message::{DataChannelMessage, ResponseChannelMessage},
        errors::RustRedisError,
        resp::Resp,
    };

    use super::SetCommandHandler;

    #[tokio::test]
    async fn should_insert_data() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1000);
        let handler = SetCommandHandler::new(sender.into());
        tokio::spawn(async move {
            if let Some(message) = receiver.recv().await {
                match message {
                    DataChannelMessage::Set(message) => message
                        .sender
                        .send(ResponseChannelMessage(Resp::simple_string_from_str("OK")))
                        .unwrap(),
                    _ => panic!(),
                }
            };
        });
        let result = handler
            .handle(&[
                Resp::bulk_string_from_str("HELLO"),
                Resp::bulk_string_from_str("WORLD"),
            ])
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Resp::simple_string_from_str("OK"));
    }

    #[tokio::test]
    async fn should_throw_error_if_not_enough_args() {
        let (sender, _) = tokio::sync::mpsc::channel(1000);
        let handler = SetCommandHandler::new(sender.into());

        let result = handler.handle(&[Resp::bulk_string_from_str("HELLO")]).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            RustRedisError::InvalidArgLength(
                SET_COMMAND_NAME.to_owned(),
                "1".to_owned(),
                "2".to_owned(),
            )
            .to_string()
        );
    }

    #[tokio::test]
    async fn should_set_key_with_expiryt() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1000);
        let handler = SetCommandHandler::new(sender.into());
        tokio::spawn(async move {
            if let Some(message) = receiver.recv().await {
                match message {
                    DataChannelMessage::Set(message) => message
                        .sender
                        .send(ResponseChannelMessage(Resp::simple_string_from_str("OK")))
                        .unwrap(),
                    _ => panic!(),
                }
            };
        });
        let result = handler
            .handle(&[
                Resp::bulk_string_from_str("HELLO"),
                Resp::bulk_string_from_str("WORLD"),
                Resp::bulk_string_from_str("PX"),
                Resp::bulk_string_from_str("1000"),
            ])
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Resp::simple_string_from_str("OK"));
    }

    #[tokio::test]
    async fn should_throw_error_for_missing_expiry_arg() {
        let (sender, _) = tokio::sync::mpsc::channel(1000);
        let handler = SetCommandHandler::new(sender.into());

        let result = handler
            .handle(&[
                Resp::bulk_string_from_str("HELLO"),
                Resp::bulk_string_from_str("WORLD"),
                Resp::bulk_string_from_str("PX"),
            ])
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            RustRedisError::InvalidArgLength(
                format!("{SET_COMMAND_NAME} with EXPIRY"),
                "3".to_string(),
                "4".to_owned(),
            )
            .to_string()
        )
    }

    #[tokio::test]
    async fn should_throw_error_if_third_args_is_not_px() {
        let (sender, _) = tokio::sync::mpsc::channel(1000);
        let handler = SetCommandHandler::new(sender.into());

        let result = handler
            .handle(&[
                Resp::bulk_string_from_str("HELLO"),
                Resp::bulk_string_from_str("WORLD"),
                Resp::bulk_string_from_str("XYZ"),
                Resp::bulk_string_from_str("1000"),
            ])
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            RustRedisError::InvalidArg(
                SET_COMMAND_NAME.to_owned(),
                "PX".to_owned(),
                "XYZ".to_owned()
            )
            .to_string()
        )
    }

    #[tokio::test]
    async fn should_throw_error_if_couldnot_parse_expiry() {
        let (sender, _) = tokio::sync::mpsc::channel(1000);
        let handler = SetCommandHandler::new(sender.into());

        let result = handler
            .handle(&[
                Resp::bulk_string_from_str("HELLO"),
                Resp::bulk_string_from_str("WORLD"),
                Resp::bulk_string_from_str("PX"),
                Resp::bulk_string_from_str("dqlskdml"),
            ])
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Could not parse expiration to a valid number".to_owned()
        )
    }
}
