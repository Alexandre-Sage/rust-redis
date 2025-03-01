use crate::{errors::RustRedisError, resp::Resp};

use super::command_registry::CommandHandler;

#[derive(Debug)]
pub struct PingCommand;

impl CommandHandler for PingCommand {
    fn handle(&self, _args: &[crate::resp::Resp]) -> Result<crate::resp::Resp, RustRedisError> {
        Ok(Resp::simple_string_from_str("PONG"))
    }
}

#[cfg(test)]
mod test {
    use crate::{commands::command_registry::CommandRegistry, resp::Resp};

    use super::PingCommand;

    #[test]
    fn should_reply_to_ping_command() {
        let mut registry = CommandRegistry::new();
        registry.register("PING", Box::new(PingCommand));
        let handler = registry.command_handler("PING").unwrap();
        let result = handler.handle(&[]).unwrap();
        assert_eq!(result, Resp::simple_string_from_str("PONG"))
    }
}
