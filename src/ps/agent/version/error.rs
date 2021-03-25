use std::{fmt, result, string};

use failure::{Backtrace, Context, Fail};
use hyper;
use semver;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn http_error(status: hyper::StatusCode, uri: hyper::Uri) -> Error {
        ErrorKind::HttpError { status, uri }.into()
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
    #[fail(display = "HTTP {}: ({})", status, uri)]
    HttpError {
        status: hyper::StatusCode,
        uri: hyper::Uri,
    },

    #[fail(display = "hyper error: {}", error)]
    HyperError { error: String },

    #[fail(display = "semver error: {}", error)]
    SemVerError { error: String },

    #[fail(display = "invalid UTF-8: {}", error)]
    FromUtf8Error { error: String },

    #[fail(display = "invalid URI: {}", error)]
    InvalidUri { error: String },
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

/// map from hyper errors
impl From<hyper::Error> for Error {
    fn from(error: hyper::Error) -> Error {
        Error::from(Context::new(ErrorKind::HyperError {
            error: error.to_string(),
        }))
    }
}

/// map from semver errors
impl From<semver::SemVerError> for Error {
    fn from(error: semver::SemVerError) -> Error {
        Error::from(Context::new(ErrorKind::SemVerError {
            error: error.to_string(),
        }))
    }
}

/// map from utf-8 errors
impl From<string::FromUtf8Error> for Error {
    fn from(error: string::FromUtf8Error) -> Error {
        Error::from(Context::new(ErrorKind::FromUtf8Error {
            error: error.to_string(),
        }))
    }
}

/// map from actix errors
impl From<actix_web::http::uri::InvalidUri> for Error {
    fn from(error: actix_web::http::uri::InvalidUri) -> Error {
        Error::from(Context::new(ErrorKind::InvalidUri {
            error: error.to_string(),
        }))
    }
}
