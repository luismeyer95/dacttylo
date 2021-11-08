use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum NetMessageError {
    ConnectionTimeout,
    UnexpectedEvent,
}
impl error::Error for NetMessageError {}
impl fmt::Display for NetMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::ConnectionTimeout => write!(f, "remote endpoint connection timeout"),
            Self::UnexpectedEvent => write!(f, "unexpected"),
        }
    }
}
