use std::collections::HashMap;

use async_trait::async_trait;
use mockall::automock;

use crate::{errors::RustRedisError, resp::Resp};

#[async_trait]
#[automock]
pub trait CommandHandler: std::fmt::Debug + Send + Sync {
    async fn handle(&self, args: &[Resp]) -> Result<Resp, RustRedisError>;
}

#[derive(Debug)]
pub struct CommandRegistry {
    registry: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let registry = HashMap::new();
        Self { registry }
    }

    pub fn register(&mut self, arg: &str, command_handler: Box<dyn CommandHandler>) {
        self.registry.insert(arg.to_uppercase(), command_handler);
    }

    pub fn command_handler(&self, arg: &str) -> Result<&Box<dyn CommandHandler>, RustRedisError> {
        self.registry
            .get(arg.to_uppercase().as_str())
            .ok_or(RustRedisError::UnknownCommand(arg.to_owned()))
    }

    pub async fn command_with_args(
        &self,
        command: &str,
        args: &[Resp],
    ) -> Result<Resp, RustRedisError> {
        let handler = self.command_handler(command)?;
        handler.handle(args).await
    }

    pub async fn no_args_command(&self, command: &str) -> Result<Resp, RustRedisError> {
        let handler = self.command_handler(command)?;
        handler.handle(&[]).await
    }
}

#[cfg(test)]
mod test {
    use super::{CommandRegistry, MockCommandHandler};

    #[test]
    fn should_register_command() {
        let mut registry = CommandRegistry::new();
        let command_handler = MockCommandHandler::new();
        registry.register("PING", Box::new(command_handler));
        assert!(registry.registry.get("PING").is_some())
    }

    #[test]
    fn should_get_command_handler_from_registry() {
        let mut registry = CommandRegistry::new();
        let command_handler = MockCommandHandler::new();
        registry.register("PING", Box::new(command_handler));
        let handler = registry.command_handler("PING");
        assert!(handler.is_ok())
    }

    #[test]
    fn should_throw_error_for_unknown_command() {
        let mut registry = CommandRegistry::new();
        let command_handler = MockCommandHandler::new();
        registry.register("PING", Box::new(command_handler));
        let handler = registry.command_handler("PONG");
        assert!(handler.is_err())
    }

    #[test]
    fn shoudl_return_command_case_insensitive() {
        let mut registry = CommandRegistry::new();
        let command_handler = MockCommandHandler::new();
        registry.register("PiNg", Box::new(command_handler));
        let handler = registry.command_handler("PING");
        assert!(handler.is_ok())
    }
}
