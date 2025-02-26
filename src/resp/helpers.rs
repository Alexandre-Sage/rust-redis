use std::path::Prefix;

use super::{deserialize::DeserializeError, r#const::CRLF_BYTES};

pub(super) fn find_crlf(data: &[u8]) -> Result<usize, DeserializeError> {
    data.windows(2)
        .position(|w| w == CRLF_BYTES)
        .ok_or(DeserializeError::InvalidCRLF)
}

pub(super) fn parse_resp_item_len(input: &[u8]) -> Result<usize, DeserializeError> {
    String::from_utf8_lossy(input)
        .parse::<usize>()
        .map_err(|_| DeserializeError::InvalidLength)
}

pub(super) fn check_prefix(input: &[u8], prefix: u8) -> Result<(), DeserializeError> {
    if input[0] == prefix {
        return Ok(());
    }
    Err(DeserializeError::InvalidPrefix)
}

pub(super) fn is_valid_utf8(input: &[u8]) -> Result<(), DeserializeError> {
    if std::str::from_utf8(input).is_err() {
        return Err(DeserializeError::InvalidUtf8);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::resp::r#const::{BULK_STRING_PREFIX, SIMPLE_ERROR_PREFIX};

    use super::*;
    const INPUT: &[u8] = b"$5\r\nhello\r\n";
    #[test]
    fn check_prefix_is_ok() {
        assert!(check_prefix(INPUT, BULK_STRING_PREFIX).is_ok())
    }

    #[test]
    fn check_prefix_error() {
        assert_eq!(
            check_prefix(INPUT, SIMPLE_ERROR_PREFIX).unwrap_err(),
            DeserializeError::InvalidPrefix
        )
    }

    #[test]
    fn should_parse_length() {
        assert_eq!(parse_resp_item_len(&INPUT[1..2]).unwrap(), 5)
    }

    #[test]
    fn parse_length_error() {
        assert_eq!(
            parse_resp_item_len(INPUT).unwrap_err(),
            DeserializeError::InvalidLength
        )
    }

    #[test]
    fn should_find_crlf() {
        assert_eq!(find_crlf(INPUT).unwrap(), 2)
    }

    #[test]
    fn find_crlf_error() {
        assert_eq!(
            find_crlf(b"flsqfklsd").unwrap_err(),
            DeserializeError::InvalidCRLF
        )
    }
}
