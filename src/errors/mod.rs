pub mod resp;

use std::{io, num::ParseIntError, str::Utf8Error};

use resp::{DeserializeError, SerializeError};

use crate::{data_management::message::MessageChannelError, resp::Resp};

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
    #[error(transparent)]
    MessageChannelError(#[from] MessageChannelError),
    #[error("Invalid args expected: '{0}'")]
    InvalidArgType(String),
    #[error("Invalid arg for command {0} expected {1} got {2}")]
    InvalidArg(String, String, String),
    #[error("Could not parse expiration to a valid number")]
    InvalidExpiry(#[from] ParseIntError),
    #[error(transparent)]
    InvalidUtf8(#[from] Utf8Error),
}

impl From<RustRedisError> for Resp {
    fn from(value: RustRedisError) -> Self {
        let binding = value.to_string();
        let error = binding.as_bytes();
        Resp::SimpleError(error.to_vec())
    }
}
