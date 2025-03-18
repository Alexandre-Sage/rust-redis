use crate::{errors::resp::DeserializeError, resp::r#const::INTEGERS_PREFIX};

use super::{
    helpers::{check_prefix, find_crlf, is_valid_utf8, parse_resp_item_len},
    r#const::{
        ARRAY_PREFIX, BULK_STRING_PREFIX, CRLF_BYTES, SIMPLE_ERROR_PREFIX, SIMPLE_STRING_PREFIX,
    },
    Resp,
};

pub(super) fn deserialize_simple_string(simple_string: &[u8]) -> Result<Resp, DeserializeError> {
    check_prefix(&simple_string, SIMPLE_STRING_PREFIX)?;
    let crlf = find_crlf(simple_string)?;
    let simple_string = &simple_string[1..crlf];
    is_valid_utf8(simple_string)?;
    Ok(Resp::SimpleString(simple_string.to_owned()))
}

pub(super) fn deserialize_simple_error(simple_error: &[u8]) -> Result<Resp, DeserializeError> {
    check_prefix(simple_error, SIMPLE_ERROR_PREFIX)?;
    let crlf = find_crlf(simple_error)?;
    let simple_error = &simple_error[1..crlf];
    is_valid_utf8(simple_error)?;
    Ok(Resp::SimpleError(simple_error.to_owned()))
}

pub(super) fn deserialize_bulk_string(bulk_string: &[u8]) -> Result<Resp, DeserializeError> {
    check_prefix(bulk_string, BULK_STRING_PREFIX)?;
    let crlf_pos = find_crlf(bulk_string)?;
    let bulk_start = crlf_pos + 2;
    let bulk_string = &bulk_string[bulk_start..];
    let crlf_pos = find_crlf(bulk_string)?;
    Ok(Resp::BulkString(bulk_string[..crlf_pos].to_owned()))
}

pub(super) fn deserialize_array(arr: &[u8]) -> Result<Resp, DeserializeError> {
    check_prefix(arr, ARRAY_PREFIX)?;
    let crlf_len = CRLF_BYTES.len();
    let first_crlf = find_crlf(arr)?;
    let arr_len = parse_resp_item_len(&arr[1..first_crlf])?;
    let mut buf = Vec::with_capacity(arr_len);
    let mut current_pos = first_crlf + crlf_len;
    while current_pos < arr.len() && buf.len() < arr_len {
        let current = &arr[current_pos..];
        match current[0] {
            BULK_STRING_PREFIX => {
                let item_first_crlf_pos = find_crlf(current)?;
                let item_len = parse_resp_item_len(&current[1..item_first_crlf_pos])?;
                let header_len = item_len.to_string().len() + 1;
                let item_len = item_len + (crlf_len * 2);
                let item = &current[..item_len + header_len];
                let de_item = Resp::deserialize(item)?;
                buf.push(de_item);
                current_pos += item.len();
            }
            SIMPLE_STRING_PREFIX => {
                let item_crlf_pos = find_crlf(current)?;
                let item = &current[..item_crlf_pos + crlf_len];
                buf.push(Resp::deserialize(item)?);
                current_pos += item.len();
            }
            ARRAY_PREFIX => {
                let item = Resp::deserialize(current)?;
                current_pos += item.size();
                buf.push(item);
            }
            _any => {
                todo!()
            }
        }
    }

    Ok(Resp::Array(buf))
}

pub(super) fn deserialize_integer(input: &[u8]) -> Result<Resp, DeserializeError> {
    check_prefix(input, INTEGERS_PREFIX)?;
    let crlf = find_crlf(input)?;
    let integer = &input[1..crlf];
    is_valid_utf8(integer)?;
    let integer = std::str::from_utf8(integer).unwrap();
    let integer = integer
        .parse()
        .map_err(|_err| DeserializeError::InvalidInteger)?;
    Ok(Resp::Integers(integer))
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn should_deserialize_bulk_string() {
        const INPUT: &[u8] = b"$5\r\nhello\r\n";
        const EXPECT: &[u8] = b"hello";
        let result = deserialize_bulk_string(INPUT).unwrap();
        assert_eq!(result, Resp::BulkString(EXPECT.to_owned()))
    }

    #[test]
    fn should_deserilaize_simple_string() {
        const INPUT: &[u8] = b"+hello\r\n";
        const EXPECT: &[u8] = b"hello";
        let result = deserialize_simple_string(INPUT).unwrap();
        assert_eq!(result, Resp::SimpleString(EXPECT.to_owned()))
    }

    #[test]
    fn should_deserialize_simple_error() {
        const EXPECT: &[u8] = b"WRONGTYPE Operation against a key holding the wrong kind of value";
        const INPUT: &[u8] =
            b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n";
        let result: Result<Resp, DeserializeError> = deserialize_simple_error(INPUT);
        assert_eq!(result.unwrap(), Resp::SimpleError(EXPECT.to_owned()))
    }

    #[test]
    fn should_deserialize_array() {
        const INPUT: &[u8] = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let expect = vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
        ];
        let result = deserialize_array(INPUT);
        assert_eq!(result.unwrap(), Resp::Array(expect))
    }

    #[test]
    fn should_deserialize_multi_type_array() {
        const INPUT: &[u8] = b"*3\r\n$5\r\nhello\r\n$5\r\nworld\r\n+PONG\r\n";
        let expect = vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
            Resp::SimpleString(b"PONG".to_vec()),
        ];
        let result = deserialize_array(INPUT);
        assert_eq!(result.unwrap(), Resp::Array(expect))
    }

    #[test]
    fn should_deserialize_nested_array() {
        const INPUT: &[u8] = b"*1\r\n*3\r\n$5\r\nhello\r\n$5\r\nworld\r\n+PONG\r\n";
        let expect = vec![Resp::Array(vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
            Resp::SimpleString(b"PONG".to_vec()),
        ])];
        let result = deserialize_array(INPUT);
        assert_eq!(result.unwrap(), Resp::Array(expect))
    }

    #[test]
    fn should_deserialize_nested_array_with_multiple_types() {
        const INPUT: &[u8] = b"*4\r\n$5\r\nhello\r\n$5\r\nworld\r\n+PONG\r\n*3\r\n$5\r\nhello\r\n$5\r\nworld\r\n+PONG\r\n";
        let expect = vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
            Resp::SimpleString(b"PONG".to_vec()),
            Resp::Array(vec![
                Resp::BulkString(b"hello".to_vec()),
                Resp::BulkString(b"world".to_vec()),
                Resp::SimpleString(b"PONG".to_vec()),
            ]),
        ];
        let result = deserialize_array(INPUT);
        assert_eq!(result.unwrap(), Resp::Array(expect))
    }

    #[test]
    fn should_deserialize_nested_array_with_nested_in_the_middle() {
        const INPUT: &[u8] = b"*6\r\n$5\r\nhello\r\n$5\r\nworld\r\n+PONG\r\n*3\r\n$5\r\nhello\r\n$5\r\nworld\r\n+PONG\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let expect = vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
            Resp::SimpleString(b"PONG".to_vec()),
            Resp::Array(vec![
                Resp::BulkString(b"hello".to_vec()),
                Resp::BulkString(b"world".to_vec()),
                Resp::SimpleString(b"PONG".to_vec()),
            ]),
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
        ];
        let result = deserialize_array(INPUT);
        assert_eq!(result.unwrap(), Resp::Array(expect))
    }
    #[test]
    fn should_deserialize_integer() {
        const INPUT: &[u8] = b":1000\r\n";
        const EXPECT: i64 = 1000;
        let result: Result<Resp, DeserializeError> = deserialize_integer(INPUT);
        assert_eq!(result.unwrap(), Resp::Integers(EXPECT))
    }

    #[test]
    fn should_deserialize_negative_integer() {
        const INPUT: &[u8] = b":-1000\r\n";
        const EXPECT: i64 = -1000;
        let result: Result<Resp, DeserializeError> = deserialize_integer(INPUT);
        assert_eq!(result.unwrap(), Resp::Integers(EXPECT))
    }

    #[test]
    fn should_deserialize_bulk_string_with_space() {
        const INPUT: &str = "$10\r\nCONFIG GET\r\n";
        const EXPECT: &[u8] = b"CONFIG GET";

        let result = deserialize_bulk_string(INPUT.as_bytes());
        assert_eq!(result.unwrap(), Resp::BulkString(EXPECT.to_owned()))
    }
}
