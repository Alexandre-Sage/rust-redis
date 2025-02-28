use super::command_registry::CommandHandler;

#[derive(Debug)]
pub struct EchoCommand;

impl CommandHandler for EchoCommand {
    fn handle(&self, args: &[crate::resp::Resp]) -> Result<crate::resp::Resp, ()> {
        dbg!(&args);
        if args.len() > 1 {
            return Err(());
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
        let handler = EchoCommand;
        let result = handler.handle(&[Resp::BulkString(b"HELLO WORLD".to_vec())]);
        assert_eq!(result.unwrap(), Resp::BulkString(b"HELLO WORLD".to_vec()))
    }
    #[test]
    fn should_throw_error_for_invalid_arg_lenght() {
        let handler = EchoCommand;
        let result = handler.handle(&[
            Resp::BulkString(b"HELLO WORLD".to_vec()),
            Resp::BulkString(b"HELLO WORLD".to_vec()),
        ]);
        assert!(result.is_err())
    }
}
