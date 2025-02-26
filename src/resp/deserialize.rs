use super::{
    helpers::{find_crlf, parse_resp_item_len},
    r#const::{
        ARRAY_PREFIX, BULK_STRING_PREFIX, CRLF_BYTES, SIMPLE_ERROR_PREFIX, SIMPLE_STRING_PREFIX,
    },
    Resp,
};

#[derive(Debug, PartialEq, Eq)]
pub enum DeserializeError {
    InvalidPrefix,
    InvalidCRLF,
    InvalidUtf8,
    InvalidLength,
}

pub fn deserialize_resp_simple_string(simple_string: &[u8]) -> Result<Resp, DeserializeError> {
    if !matches!(simple_string[0], SIMPLE_STRING_PREFIX) {
        return Err(DeserializeError::InvalidPrefix);
    }
    match simple_string.windows(2).position(|w| w == CRLF_BYTES) {
        Some(pos) => {
            let simple_string = &simple_string[1..pos];
            if std::str::from_utf8(simple_string).is_err() {
                return Err(DeserializeError::InvalidUtf8);
            }
            Ok(Resp::SimpleString(simple_string.to_owned()))
        }
        None => Err(DeserializeError::InvalidCRLF),
    }
}

pub(super) fn deserialize_resp_simple_error(simple_error: &[u8]) -> Result<Resp, DeserializeError> {
    if !matches!(simple_error[0], SIMPLE_ERROR_PREFIX) {
        return Err(DeserializeError::InvalidPrefix);
    }
    let crlf = find_crlf(simple_error)?;
    let simple_error = &simple_error[1..crlf];
    if std::str::from_utf8(simple_error).is_err() {
        return Err(DeserializeError::InvalidUtf8);
    }
    Ok(Resp::SimpleError(simple_error.to_owned()))
}

pub fn deserialize_resp_bulk_string(bulk_string: &[u8]) -> Result<Resp, DeserializeError> {
    if !matches!(bulk_string[0], BULK_STRING_PREFIX) {
        return Err(DeserializeError::InvalidPrefix);
    }
    let crlf_pos = bulk_string
        .windows(2)
        .position(|w| w == CRLF_BYTES)
        .ok_or(DeserializeError::InvalidCRLF)?;
    let _length = std::str::from_utf8(&bulk_string[1..crlf_pos])
        .map(|length| length.parse::<usize>())
        .map_err(|_| DeserializeError::InvalidLength)?;
    let bulk_start = crlf_pos + 2;
    let bulk_string = &bulk_string[bulk_start..];
    match bulk_string.windows(2).position(|w| w == CRLF_BYTES) {
        Some(pos) => Ok(Resp::BulkString(bulk_string[..pos].to_owned())),
        None => Err(DeserializeError::InvalidCRLF),
    }
}

pub fn deserialize_resp_array(arr: &[u8]) -> Result<Resp, DeserializeError> {
    let crlf_len = CRLF_BYTES.len();
    let first_crlf = find_crlf(arr)?;
    let arr_len = parse_resp_item_len(&arr[1..first_crlf])?;
    let mut buf = Vec::with_capacity(arr_len);
    let mut current_pos = first_crlf + crlf_len;
    let mut parsed_item = 0;
    while current_pos < arr.len() && parsed_item < arr_len {
        let current = &arr[current_pos..];
        dbg!(String::from_utf8_lossy(current));
        dbg!(&current_pos, arr.len());
        dbg!(&buf);
        match current[0] {
            BULK_STRING_PREFIX => {
                let item_first_crlf_pos = find_crlf(current)?;
                let item_len = parse_resp_item_len(&current[1..item_first_crlf_pos])?;
                let item_len = item_len + (crlf_len * 2);
                let item = &current[..item_len + 2];
                let de_item = Resp::deserialize(item)?;
                buf.push(de_item);
                parsed_item += 1;
                current_pos += item.len();
            }
            SIMPLE_STRING_PREFIX => {
                let item_crlf_pos = find_crlf(current)?;
                let item = &current[..item_crlf_pos + crlf_len];
                buf.push(Resp::deserialize(item)?);
                parsed_item += 1;
                current_pos += item.len();
            }
            ARRAY_PREFIX => {
                let item_first_crlf_pos = find_crlf(current)?;
                let item_len = parse_resp_item_len(&current[1..item_first_crlf_pos])?;
                let item = Resp::deserialize(current)?;
                current_pos += item.size() + 1;
                buf.push(item);
                parsed_item += 1;
            }
            _any => {
                dbg!();
                dbg!(&buf);
                todo!()
            }
        }
    }

    Ok(Resp::Array(buf))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn should_deserialize_bulk_string() {
        const INPUT: &[u8] = b"$5\r\nhello\r\n";
        const EXPECT: &[u8] = b"hello";
        let result = deserialize_resp_bulk_string(INPUT).unwrap();
        assert_eq!(result, Resp::BulkString(EXPECT.to_owned()))
    }

    #[test]
    fn should_deserilaize_simple_string() {
        const INPUT: &[u8] = b"+hello\r\n";
        const EXPECT: &[u8] = b"hello";
        let result = deserialize_resp_simple_string(INPUT).unwrap();
        assert_eq!(result, Resp::SimpleString(EXPECT.to_owned()))
    }

    #[test]
    fn should_deserialize_simple_error() {
        const EXPECT: &[u8] = b"WRONGTYPE Operation against a key holding the wrong kind of value";
        const INPUT: &[u8] =
            b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n";
        let result: Result<Resp, DeserializeError> = deserialize_resp_simple_error(INPUT);
        assert_eq!(result.unwrap(), Resp::SimpleError(EXPECT.to_owned()))
    }

    #[test]
    fn should_deserialize_array() {
        const INPUT: &[u8] = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let expect = vec![
            Resp::BulkString(b"hello".to_vec()),
            Resp::BulkString(b"world".to_vec()),
        ];
        let result = deserialize_resp_array(INPUT);
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
        let result = deserialize_resp_array(INPUT);
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
        let result = deserialize_resp_array(INPUT);
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
        let result = deserialize_resp_array(INPUT);
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
        let result = deserialize_resp_array(INPUT);
        assert_eq!(result.unwrap(), Resp::Array(expect))
    }
}
