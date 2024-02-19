use arrow::error::ArrowError;
use std::backtrace::{Backtrace, BacktraceStatus};
use std::error::Error;
use std::fmt;

pub type Result<T, E = RayexecError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct RayexecError {
    /// Message for the error.
    pub msg: String,

    /// Source of the error.
    pub source: Option<Box<dyn Error>>,

    /// Captured backtrace for the error.
    ///
    /// Enable with the RUST_BACKTRACE env var.
    pub backtrace: Backtrace,
}

impl RayexecError {
    pub fn new(msg: impl Into<String>) -> Self {
        RayexecError {
            msg: msg.into(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    pub fn with_source(msg: impl Into<String>, source: Box<dyn Error>) -> Self {
        RayexecError {
            msg: msg.into(),
            source: Some(source),
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<ArrowError> for RayexecError {
    fn from(value: ArrowError) -> Self {
        Self::with_source("Arrow error", Box::new(value))
    }
}

impl fmt::Display for RayexecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)?;
        if let Some(source) = &self.source {
            write!(f, "Error source: {}", source)?;
        }

        if matches!(self.backtrace.status(), BacktraceStatus::Captured) {
            write!(f, "Backtrace:\n{}", self.backtrace)?;
        }

        Ok(())
    }
}

impl Error for RayexecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref())
    }
}

/// An extension trait for adding context to the Error variant of a result.
pub trait ResultExt<T, E> {
    fn context(self: Self, msg: &'static str) -> Result<T>;
}

impl<T, E: Error + 'static> ResultExt<T, E> for std::result::Result<T, E> {
    fn context(self: Self, msg: &'static str) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(RayexecError::with_source(msg, Box::new(e))),
        }
    }
}
