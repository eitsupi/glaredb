use parquet::errors::ParquetError;
use std::backtrace::{Backtrace, BacktraceStatus};
use std::error::Error;
use std::fmt;

pub type Result<T, E = RayexecError> = std::result::Result<T, E>;

/// Helper macros for returning an error for currently unimplemented items.
///
/// This should generally be used in place of the `unimplemented` macro.
#[macro_export]
macro_rules! not_implemented {
    ($($arg:tt)+) => {{
        let msg = format!($($arg)+);
        return Err($crate::RayexecError::new(format!("Not yet implemented: {msg}")));
    }};
}

#[derive(Debug)]
pub struct RayexecError {
    /// Message for the error.
    pub msg: String,

    /// Source of the error.
    pub source: Option<Box<dyn Error + Send + Sync>>,

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

    pub fn with_source(msg: impl Into<String>, source: Box<dyn Error + Send + Sync>) -> Self {
        RayexecError {
            msg: msg.into(),
            source: Some(source),
            backtrace: Backtrace::capture(),
        }
    }

    pub fn get_backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl From<ParquetError> for RayexecError {
    fn from(value: ParquetError) -> Self {
        Self::with_source("Parquet error", Box::new(value))
    }
}

impl From<fmt::Error> for RayexecError {
    fn from(value: fmt::Error) -> Self {
        Self::with_source("Format error", Box::new(value))
    }
}

impl From<std::io::Error> for RayexecError {
    fn from(value: std::io::Error) -> Self {
        Self::with_source("IO error", Box::new(value))
    }
}

// TODO: This loses a bit of context surrounding the source of the error. What
// was the value? What were we converting to?
//
// Likely this should be removed once we're in the polishing phase.
impl From<std::num::TryFromIntError> for RayexecError {
    fn from(value: std::num::TryFromIntError) -> Self {
        Self::with_source("Int convert error", Box::new(value))
    }
}

impl fmt::Display for RayexecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)?;
        if let Some(source) = &self.source {
            write!(f, "\nError source: {}", source)?;
        }

        if self.backtrace.status() == BacktraceStatus::Captured {
            write!(f, "\nBacktrace: {}", self.backtrace)?
        }

        Ok(())
    }
}

impl Error for RayexecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}

/// An extension trait for adding context to the Error variant of a result.
pub trait ResultExt<T, E> {
    /// Wrap an error with a static context string.
    fn context(self, msg: &'static str) -> Result<T>;

    /// Wrap an error with a context string generated from a function.
    fn context_fn<F: Fn() -> String>(self, f: F) -> Result<T>;
}

impl<T, E: Error + Send + Sync + 'static> ResultExt<T, E> for std::result::Result<T, E> {
    fn context(self, msg: &'static str) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(RayexecError::with_source(msg, Box::new(e))),
        }
    }

    fn context_fn<F: Fn() -> String>(self, f: F) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(RayexecError::with_source(f(), Box::new(e))),
        }
    }
}
