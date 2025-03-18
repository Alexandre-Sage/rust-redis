use std::collections::HashMap;

use async_trait::async_trait;

use crate::{config::ConfigField, errors::RustRedisError, resp::Resp};

use super::command_registry::CommandHandler;

pub const GET_CONFIG_COMMAND_NAME: &str = "CONFIG GET";

#[derive(Debug)]
pub struct GetConfigCommandHandler {
    config: HashMap<ConfigField, Resp>,
}

impl GetConfigCommandHandler {
    pub fn new(config: HashMap<ConfigField, Resp>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl CommandHandler for GetConfigCommandHandler {
    async fn handle(&self, args: &[Resp]) -> Result<Resp, RustRedisError> {
        if args.len() < 1 {
            return Err(RustRedisError::InvalidArgLength(
                GET_CONFIG_COMMAND_NAME.to_owned(),
                "1".to_owned(),
                args.len().to_string(),
            ));
        }

        let field = ConfigField::try_from(&args[0])?;
        let value = self.config.get(&field).unwrap();
        let response = Resp::Array([args[0].to_owned(), value.to_owned()].into());

        Ok(response)
    }
}

#[cfg(test)]
mod tests {

    use clap::Parser;

    use crate::config::AppConfig;

    use super::*;

    #[tokio::test]
    async fn should_get_config_field() {
        let config = AppConfig::parse_from(["config", "--dbfilename", "redis.rdb"]);
        let handler = GetConfigCommandHandler::new((&config).into());
        let arg = Resp::bulk_string_from_str("dbfilename");

        let result = handler.handle(&[arg]).await.unwrap();
        assert_eq!(
            result,
            Resp::Array(
                [
                    Resp::bulk_string_from_str("dbfilename"),
                    Resp::bulk_string_from_str("redis.rdb")
                ]
                .into()
            )
        )
    }

    #[tokio::test]
    async fn should_throw_error_for_non_bulk_string_arg() {
        let config = AppConfig::parse_from(["config", "--dbfilename", "redis.rdb"]);
        let handler = GetConfigCommandHandler::new((&config).into());
        let arg = Resp::simple_string_from_str("HELLO");

        let result = handler.handle(&[arg]).await;
        assert_eq!(
            result.unwrap_err().to_string(),
            RustRedisError::InvalidArgType("bulk string".to_owned()).to_string()
        )
    }

    #[tokio::test]
    async fn should_throw_error_for_invalid_arg_length() {
        let config = AppConfig::parse_from(["config", "--dbfilename", "redis.rdb"]);
        let handler = GetConfigCommandHandler::new((&config).into());

        let result = handler.handle(&[]).await;
        assert_eq!(
            result.unwrap_err().to_string(),
            RustRedisError::InvalidArgLength(
                GET_CONFIG_COMMAND_NAME.to_owned(),
                "1".to_owned(),
                "0".to_owned()
            )
            .to_string()
        )
    }

    //#[tokio::test]
    //async fn should_throw_error_for_invalid_config_field_name() {
    //    let config = AppConfig::parse_from(["config", "--dbfilename", "redis.rdb"]);
    //    let handler = GetConfigCommandHandler::new(config.into());
    //}
}
