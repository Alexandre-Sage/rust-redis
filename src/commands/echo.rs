use crate::errors::RustRedisError;

use super::command_registry::CommandHandler;

#[derive(Debug)]
pub struct EchoCommand {
    name: String,
    args: String,
}

impl EchoCommand {
    pub fn new() -> Self {
        Self {
            name: "echo".to_string(),
            args: "1".to_string(),
        }
    }
}

impl CommandHandler for EchoCommand {
    fn handle(&self, args: &[crate::resp::Resp]) -> Result<crate::resp::Resp, RustRedisError> {
        dbg!(&args);
        if args.len() > 1 {
            return Err(RustRedisError::InvalidArgLength(
                self.name.clone(),
                args.len().to_string(),
                self.args.clone(),
            ));
        }
        Ok(args[0].clone())
    }
}

#[cfg(test)]
mod test {
    use crate::{commands::command_registry::CommandHandler, resp::Resp};

    use super::EchoCommand;

    #[test]
    fn should_reply_to_echo() {
        let handler = EchoCommand::new();
        let result = handler.handle(&[Resp::BulkString(b"HELLO WORLD".to_vec())]);
        assert_eq!(result.unwrap(), Resp::BulkString(b"HELLO WORLD".to_vec()))
    }
    #[test]
    fn should_throw_error_for_invalid_arg_lenght() {
        let handler = EchoCommand::new();
        let result = handler.handle(&[
            Resp::BulkString(b"HELLO WORLD".to_vec()),
            Resp::BulkString(b"HELLO WORLD".to_vec()),
        ]);
        assert!(result.is_err())
    }
}
