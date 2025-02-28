#[derive(Debug, PartialEq, Eq)]
pub enum DeserializeError {
    InvalidPrefix,
    InvalidCRLF,
    InvalidUtf8,
    InvalidLength,
    InvalidInteger,
}

#[derive(Debug)]
pub enum SerializeError {
    InvaliUtf8,
}
