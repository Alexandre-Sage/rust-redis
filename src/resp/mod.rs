use deserialize::{
    deserialize_array, deserialize_bulk_string, deserialize_integer, deserialize_simple_error,
    deserialize_simple_string,
};
use r#const::{
    ARRAY_PREFIX, BULK_STRING_PREFIX, CRLF_BYTES, INTEGERS_PREFIX, SIMPLE_ERROR_PREFIX,
    SIMPLE_STRING_PREFIX,
};
use serialize::{
    serialize_array, serialize_bulk_string, serialize_integer, serialize_simple_error,
    serialize_simple_string,
};

use crate::{
    errors::resp::{DeserializeError, SerializeError},
    ternary_expr,
};

mod r#const;
mod deserialize;
mod helpers;
pub mod serialize;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Resp {
    SimpleString(Vec<u8>),
    SimpleError(Vec<u8>),
    BulkString(Vec<u8>),
    Array(Vec<Resp>),
    Integers(i64),
}

impl Resp {
    pub fn serialize(self) -> Result<Vec<u8>, SerializeError> {
        match self {
            Resp::BulkString(bulk) => serialize_bulk_string(&bulk),
            Resp::SimpleString(simple) => serialize_simple_string(&simple),
            Resp::Array(array) => serialize_array(array),
            Resp::SimpleError(error) => serialize_simple_error(&error),
            Resp::Integers(int) => serialize_integer(int),
        }
    }

    pub fn deserialize(input: &[u8]) -> Result<Resp, DeserializeError> {
        match input[0] {
            SIMPLE_STRING_PREFIX => deserialize_simple_string(input),
            BULK_STRING_PREFIX => deserialize_bulk_string(input),
            ARRAY_PREFIX => deserialize_array(input),
            SIMPLE_ERROR_PREFIX => deserialize_simple_error(input),
            INTEGERS_PREFIX => deserialize_integer(input),
            _any => Err(DeserializeError::InvalidPrefix),
        }
    }

    pub fn bulk_string_from_str(value: &str) -> Self {
        Self::BulkString(value.into())
    }

    pub fn simple_string_from_str(value: &str) -> Self {
        Self::SimpleString(value.into())
    }

    pub fn as_str(&self) -> Result<&str, ()> {
        match self {
            Resp::BulkString(bulk) => Ok(std::str::from_utf8(bulk).unwrap()),
            Resp::SimpleString(simple_string) => Ok(std::str::from_utf8(&simple_string).unwrap()),
            //Resp::Integers(int) => Ok(std::str::from_utf8(int.to_string().as_bytes()).unwrap()),
            _ => Err(()),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::SimpleError(string) | Self::SimpleString(string) => {
                string.len() + 1 + (CRLF_BYTES.len())
            }
            Self::BulkString(string) => {
                let len = string.len();
                1 + len + len.to_string().len() + (CRLF_BYTES.len() * 2)
            }
            Self::Array(arr) => {
                let len = arr.len().to_string().len();
                arr.iter()
                    .fold(1 + len + (CRLF_BYTES.len()), |mut acc, cur| {
                        acc += cur.size();
                        acc
                    })
            }
            Self::Integers(int) => {
                let header_len = ternary_expr!(*int < 0, 2, 1);
                let len = int.to_string().len();
                header_len + len + CRLF_BYTES.len()
            }
        }
    }
}
