use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct ValidationFailedError {
    message: String,
}

impl ValidationFailedError {
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<io::Error> for ValidationFailedError {
    fn from(err: io::Error) -> Self {
        Self::new(err.to_string())
    }
}

impl From<serde_json::Error> for ValidationFailedError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(err.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for ValidationFailedError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self::new(err.to_string())
    }
}
