//! Server related errors
use std::{fmt, io, result, sync};

use failure::{Backtrace, Context, Fail};

use futures::sync::mpsc;

use crate::ps::agent::cache;
use crate::ps::agent::types::ServiceId;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn invalid_message_type<S: Into<String>>(message_type: S) -> Error {
        ErrorKind::InvalidMessageType {
            message_type: message_type.into(),
        }
        .into()
    }

    pub fn port_already_in_use(port: u16, service_id: ServiceId) -> Error {
        ErrorKind::PortAlreadyInUseError { port, service_id }.into()
    }

    pub fn startup<S: Into<String>>(message: S) -> Error {
        ErrorKind::StartupError {
            message: message.into(),
        }
        .into()
    }

    pub fn io_error<S: Into<String>>(message: S) -> Error {
        ErrorKind::IoError {
            error: message.into(),
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
    #[fail(display = "Server operation cancelled")]
    Cancelled,

    #[fail(display = "Invalid port: {}:{}", hostname, port)]
    InvalidPort { hostname: String, port: u16 },

    #[fail(display = "Port already in use: {}", port)]
    PortAlreadyInUseError { port: u16, service_id: ServiceId },

    #[fail(display = "Invalid message type: {}", message_type)]
    InvalidMessageType { message_type: String },

    #[fail(display = "Startup error: {}", message)]
    StartupError { message: String },

    #[fail(display = "Server shutdown unexpectedly")]
    ShutdownError,

    #[fail(display = "Protobuf error: {}", error)]
    ProtobufError { error: String },

    #[fail(display = "Tungstenite websocket error: {}", error)]
    TungsteniteError { error: String },

    #[fail(display = "std::sync::PoisonError: {}", error)]
    SyncPoisonError { error: String },

    #[fail(display = "MPSC send error: {}", error)]
    MpscSendError { error: String },

    #[fail(display = "Empty timeseries message segment")]
    EmptyMessage,

    #[fail(display = "JSON error: {}", error)]
    JsonError { error: String },

    #[fail(display = "URL parse error: {}", error)]
    UrlParseError { error: String },

    #[fail(display = "I/O error: {}", error)]
    IoError { error: String },

    #[fail(display = "Cache error: {}", kind)]
    CacheError { kind: cache::ErrorKind },
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

/// map from cache errors
impl From<cache::ErrorKind> for Error {
    fn from(kind: cache::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::CacheError { kind }))
    }
}
impl From<cache::Error> for Error {
    fn from(error: cache::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from io errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from url parse errors
impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Error {
        Error::from(Context::new(ErrorKind::UrlParseError {
            error: error.to_string(),
        }))
    }
}

/// map from serde_json errors
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        Error::from(Context::new(ErrorKind::JsonError {
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

/// map from tungstenite errors
impl From<tungstenite::Error> for Error {
    fn from(error: tungstenite::Error) -> Error {
        Error::from(Context::new(ErrorKind::TungsteniteError {
            error: error.to_string(),
        }))
    }
}

/// map from mpsc errors
impl<T> From<mpsc::SendError<T>> for Error {
    fn from(error: mpsc::SendError<T>) -> Error {
        Error::from(Context::new(ErrorKind::MpscSendError {
            error: error.to_string(),
        }))
    }
}

/// map from sync PoisonError
impl<T> From<sync::PoisonError<T>> for Error {
    fn from(error: sync::PoisonError<T>) -> Error {
        Error::from(Context::new(ErrorKind::SyncPoisonError {
            error: error.to_string(),
        }))
    }
}
