use crate::ternary_expr;

use super::{
    errors::SerializeError,
    r#const::{
        ARRAY_PREFIX, BULK_STRING_PREFIX, CRLF_BYTES, INTEGERS_PREFIX, SIMPLE_ERROR_PREFIX,
        SIMPLE_STRING_PREFIX,
    },
    Resp,
};

pub(super) fn serialize_simple_string(simple_string: &[u8]) -> Result<Vec<u8>, SerializeError> {
    if std::str::from_utf8(simple_string).is_err() {
        return Err(SerializeError::InvaliUtf8);
    }
    let length = simple_string.len();
    let mut buf = Vec::with_capacity(1 + length + CRLF_BYTES.len());
    buf.push(SIMPLE_STRING_PREFIX);
    buf.extend_from_slice(simple_string);
    buf.extend_from_slice(CRLF_BYTES);
    Ok(buf)
}

pub(super) fn serialize_bulk_string(bulk_string: &[u8]) -> Result<Vec<u8>, SerializeError> {
    let length = bulk_string.len();
    let length_string = length.to_string();
    let mut buf = Vec::with_capacity(length + (CRLF_BYTES.len() * 2) + length_string.len() + 1);
    buf.push(BULK_STRING_PREFIX);
    buf.extend_from_slice(length_string.as_bytes());
    buf.extend_from_slice(CRLF_BYTES);
    buf.extend_from_slice(bulk_string);
    buf.extend_from_slice(CRLF_BYTES);
    Ok(buf)
}

pub(super) fn serialize_array(input: Vec<Resp>) -> Result<Vec<u8>, SerializeError> {
    let mut buf = Vec::new();
    let length = input.len();
    let length_string = length.to_string();
    buf.push(ARRAY_PREFIX);
    buf.extend_from_slice(length_string.as_bytes());
    buf.extend_from_slice(CRLF_BYTES);
    for val in input {
        let val = val.serialize()?;
        buf.extend(val);
    }
    Ok(buf)
}
pub(super) fn serialize_simple_error(error: &[u8]) -> Result<Vec<u8>, SerializeError> {
    if std::str::from_utf8(error).is_err() {
        return Err(SerializeError::InvaliUtf8);
    }
    let length = error.len();
    let mut buf = Vec::with_capacity(length + 1 + CRLF_BYTES.len());
    buf.push(SIMPLE_ERROR_PREFIX);
    buf.extend_from_slice(error);
    buf.extend_from_slice(CRLF_BYTES);
    Ok(buf)
}

pub(super) fn serialize_integer(int: i64) -> Result<Vec<u8>, SerializeError> {
    let header = ternary_expr!(int < 0, 2, 1);
    let integer_string = int.to_string();
    let len = integer_string.len();
    let mut buf = Vec::with_capacity(header + len + CRLF_BYTES.len());
    buf.push(INTEGERS_PREFIX);
    buf.extend_from_slice(integer_string.as_bytes());
    buf.extend_from_slice(CRLF_BYTES);
    Ok(buf)
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn should_serialize_bulk_string() {
        const EXPECT: &[u8] = b"$5\r\nhello\r\n";
        const INPUT: &[u8] = b"hello";
        let result = serialize_bulk_string(INPUT).unwrap();
        assert_eq!(result, EXPECT)
    }
    #[test]
    fn should_serialize_simple_string() {
        const EXPECT: &[u8] = b"+hello\r\n";
        const INPUT: &[u8] = b"hello";
        let result = serialize_simple_string(INPUT).unwrap();
        assert_eq!(result, EXPECT)
    }

    #[test]
    fn should_serialize_array() {
        const EXPECT: &[u8] = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let input = vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
        ];
        let result = serialize_array(input).unwrap();
        assert_eq!(result, EXPECT)
    }
    #[test]
    fn should_serialize_error() {
        const EXPECT: &[u8] =
            b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n";
        const INPUT: &[u8] = b"WRONGTYPE Operation against a key holding the wrong kind of value";
        let result: Result<Vec<u8>, SerializeError> = serialize_simple_error(INPUT);
        assert_eq!(result.unwrap(), EXPECT)
    }
    #[test]
    fn should_serialize_integer() {
        const INPUT: i64 = 1000;
        const EXPECT: &[u8] = b":1000\r\n";
        let result: Result<Vec<u8>, SerializeError> = serialize_integer(INPUT);
        assert_eq!(result.unwrap(), EXPECT)
    }
    #[test]
    fn should_serialize_negative_integer() {
        const INPUT: i64 = -1000;
        const EXPECT: &[u8] = b":-1000\r\n";
        let result: Result<Vec<u8>, SerializeError> = serialize_integer(INPUT);
        assert_eq!(result.unwrap(), EXPECT)
    }
}
