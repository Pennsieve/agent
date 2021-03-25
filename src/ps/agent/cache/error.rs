//! Cache-specific errors.
use failure::{Backtrace, Context, Fail};
use std::{fmt, io, path, result};

use crate::ps::agent::database;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn required_parameter<S: Into<String>>(parameter: S) -> Error {
        ErrorKind::RequiredParameter {
            parameter: parameter.into(),
        }
        .into()
    }

    pub fn invalid_page<P: Into<path::PathBuf>>(page: P) -> Error {
        ErrorKind::InvalidPage { page: page.into() }.into()
    }

    pub fn invalid_channel<S: Into<String>>(channel: S) -> Error {
        ErrorKind::InvalidChannel {
            channel: channel.into(),
        }
        .into()
    }

    pub fn no_space<S: Into<String>>(message: S) -> Error {
        ErrorKind::NoSpace {
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
    #[fail(display = "missing required parameter: {}", parameter)]
    RequiredParameter { parameter: String },

    #[fail(display = "invalid page: {:?}", page)]
    InvalidPage { page: path::PathBuf },

    #[fail(display = "invalid channel: {}", channel)]
    InvalidChannel { channel: String },

    #[fail(display = "collector cancelled")]
    CollectorCancelled,

    #[fail(display = "no available space: {}", message)]
    NoSpace { message: String },

    #[fail(display = "io error: {}", error)]
    IoError { error: String },

    #[fail(display = "protobuf error: {}", error)]
    ProtobufError { error: String },

    #[fail(display = "database error: {}", kind)]
    DatabaseError { kind: database::ErrorKind },
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

/// map from IO errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from protobuf errors
impl From<protobuf::ProtobufError> for Error {
    fn from(error: protobuf::ProtobufError) -> Error {
        Error::from(Context::new(ErrorKind::ProtobufError {
            error: error.to_string(),
        }))
    }
}
