#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DeserializeError {
    #[error("ERR invalid resp prefix")]
    InvalidPrefix,
    #[error("ERR invalid crlf")]
    InvalidCRLF,
    #[error("ERR invalid UTF-8")]
    InvalidUtf8,
    #[error("ERR invalid length")]
    InvalidLength,
    #[error("ERR invalid integer")]
    InvalidInteger,
}

#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("ERR invalid UTF-8")]
    InvaliUtf8,
}
