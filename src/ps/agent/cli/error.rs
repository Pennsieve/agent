//! Cli related errors

use failure::{Backtrace, Context, Fail};
use std::{fmt, io, path::PathBuf, result};

use tokio::timer;

use crate::ps::agent::{self, config, database};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn invalid_login(error: agent::Error) -> Error {
        ErrorKind::InvalidLogin {
            message: error.to_string(),
        }
        .into()
    }

    pub fn upload_does_not_match(path: PathBuf) -> Error {
        ErrorKind::UploadDoesNotMatch { path }.into()
    }

    pub fn upload_error<S: Into<String>>(message: S) -> Error {
        ErrorKind::UploadError {
            message: message.into(),
        }
        .into()
    }

    pub fn move_error<S: Into<String>>(message: S) -> Error {
        ErrorKind::MoveError {
            message: message.into(),
        }
        .into()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.ctx.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.ctx.backtrace()
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        self.kind().clone().into()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ctx.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Fail)]
pub enum ErrorKind {
    #[fail(display = "Invalid login: {}", message)]
    InvalidLogin { message: String },

    #[fail(
        display = "Local file did not match corresponding file on the platform: {:?}",
        path
    )]
    UploadDoesNotMatch { path: PathBuf },

    #[fail(display = "Config error: {}", kind)]
    ConfigError { kind: config::ErrorKind },

    #[fail(display = "I/O error: {}", error)]
    IoError { error: String },

    #[fail(display = "Timer error: {}", error)]
    TokioTimerError { error: String },

    #[fail(display = "Database error: {}", kind)]
    DatabaseError { kind: database::ErrorKind },

    #[fail(display = "Upload error: {}", message)]
    UploadError { message: String },

    #[fail(display = "Move error: {}", message)]
    MoveError { message: String },
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::from(Context::new(kind))
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(ctx: Context<ErrorKind>) -> Error {
        Error { ctx }
    }
}

/// map from IO errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from tokio timer errors
impl From<timer::Error> for Error {
    fn from(error: timer::Error) -> Error {
        Error::from(Context::new(ErrorKind::TokioTimerError {
            error: error.to_string(),
        }))
    }
}

/// map from config errors
impl From<config::ErrorKind> for Error {
    fn from(kind: config::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::ConfigError { kind }))
    }
}
impl From<config::Error> for Error {
    fn from(error: config::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from database errors
impl From<database::ErrorKind> for Error {
    fn from(kind: database::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::DatabaseError { kind }))
    }
}
impl From<database::Error> for Error {
    fn from(error: database::Error) -> Error {
        error.kind().clone().into()
    }
}
