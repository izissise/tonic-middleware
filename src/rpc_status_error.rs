use thiserror::Error;
use std::fmt;
use tonic::Status;

/// Wrap tonic status so it's an error type
#[derive(Error, Debug)]
pub struct RPCStatusError(Status);

impl RPCStatusError {
    /// Transform tonic::Status to RPCStatusError
    pub fn from_status(s: Status) -> Self {
        Self(s)
    }
}

impl From<RPCStatusError> for Status {
    fn from(e: RPCStatusError) -> Status {
        e.0
    }
}

impl fmt::Display for RPCStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
