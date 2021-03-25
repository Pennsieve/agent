//! Config-specific errors
use ini;
use rustyline::error::ReadlineError;
use std::{fmt, io, result, str};

use failure::{Backtrace, Context, Fail};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn invalid_api_config<S: Into<String>>(message: S) -> Error {
        ErrorKind::InvalidApiConfig {
            message: message.into(),
        }
        .into()
    }

    pub fn illegal_operation<S: Into<String>>(message: S) -> Error {
        ErrorKind::IllegalOperation {
            message: message.into(),
        }
        .into()
    }

    pub fn config_file_not_found<S: Into<String>>(message: S) -> Error {
        ErrorKind::ConfigFileNotFound {
            message: message.into(),
        }
        .into()
    }

    pub fn config_value_not_found<S: Into<String>>(key: S) -> Error {
        ErrorKind::MissingConfigValue { key: key.into() }.into()
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
    #[fail(display = "invalid api config: {}", message)]
    InvalidApiConfig { message: String },

    #[fail(display = "illegal operation: {}", message)]
    IllegalOperation { message: String },

    #[fail(display = "config file not found: {}", message)]
    ConfigFileNotFound { message: String },

    #[fail(display = "missing user profile")]
    MissingProfile,

    #[fail(display = "no services defined")]
    NoServicesDefined,

    #[fail(display = "ini parse error: {}", message)]
    IniParseError { message: String },

    #[fail(display = "readline error: {}", error)]
    ReadlineError { error: String },

    #[fail(display = "io error: {}", error)]
    IoError { error: String },

    #[fail(display = "cancelled")]
    UserCancelledError,

    #[fail(display = "configuration value \"{}\" not found", key)]
    MissingConfigValue { key: String },
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

/// map from ini errors
impl From<ini::ini::ParseError> for Error {
    fn from(error: ini::ini::ParseError) -> Error {
        Error::from(Context::new(ErrorKind::IniParseError {
            message: error.to_string(),
        }))
    }
}

/// map from rustyline readline errors
impl From<ReadlineError> for Error {
    fn from(error: ReadlineError) -> Error {
        Error::from(Context::new(ErrorKind::IniParseError {
            message: error.to_string(),
        }))
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
