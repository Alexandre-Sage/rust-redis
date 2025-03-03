pub mod resp;

use std::io;

use resp::{DeserializeError, SerializeError};

use crate::resp::Resp;

#[derive(Debug, thiserror::Error)]
pub enum RustRedisError {
    #[error("ERR invalid command '{0}'")]
    InvalidCommand(String),
    #[error("ERR unknown command '{0}'")]
    UnknownCommand(String),
    #[error("ERR wrong number of arguments for '{0}' expected {1} command got {2}")]
    InvalidArgLength(String, String, String),
    #[error(transparent)]
    SocketError(#[from] io::Error),
    #[error(transparent)]
    SerializeError(#[from] SerializeError),
    #[error(transparent)]
    DeserializeError(#[from] DeserializeError),
}

impl From<RustRedisError> for Resp {
    fn from(value: RustRedisError) -> Self {
        let binding = value.to_string();
        let error = binding.as_bytes();
        Resp::SimpleError(error.to_vec())
    }
}
