//! Database specific errors.
use failure::{Backtrace, Context, Fail};
use std::{fmt, io, path, result};

use crate::ps::agent::config;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn status<S: Into<String>>(status: S) -> Error {
        ErrorKind::Status {
            status: status.into(),
        }
        .into()
    }

    pub fn path(path: path::PathBuf) -> Error {
        ErrorKind::Path { path }.into()
    }

    pub fn upload_not_found(upload_id: usize) -> Error {
        ErrorKind::UploadNotFound { upload_id }.into()
    }

    pub fn upload_without_chunk_size(upload_id: usize) -> Error {
        ErrorKind::UploadWithoutChunkSize { upload_id }.into()
    }

    pub fn migration<S: Into<String>, T: Into<String>>(version: usize, error: T, sql: S) -> Error {
        ErrorKind::Migration {
            version,
            error: error.into(),
            sql: sql.into(),
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
    #[fail(display = "Invalid path: {:?}", path)]
    Path { path: path::PathBuf },

    #[fail(display = "Invalid status: {}", status)]
    Status { status: String },

    #[fail(display = "Migration error: version {}: {}: {}", version, error, sql)]
    Migration {
        version: usize,
        error: String,
        sql: String,
    },

    #[fail(display = "r2d2 error: {}", error)]
    R2d2Error { error: String },

    #[fail(display = "rusqlite error: {}", error)]
    RusqliteError { error: String },

    #[fail(display = "Query returned no rows")]
    QueryReturnedNoRows,

    #[fail(display = "No upload found with ID: {}", upload_id)]
    UploadNotFound { upload_id: usize },

    #[fail(display = "Upload does not have a chunk size set: {}", upload_id)]
    UploadWithoutChunkSize { upload_id: usize },

    #[fail(display = "I/O error: {}", error)]
    IoError { error: String },

    #[fail(display = "Config error: {}", kind)]
    ConfigError { kind: config::ErrorKind },
}

/// map from IO errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from r2d2 errors
impl From<r2d2::Error> for Error {
    fn from(error: r2d2::Error) -> Error {
        Error::from(Context::new(ErrorKind::R2d2Error {
            error: error.to_string(),
        }))
    }
}

/// map from rusqlite errors
impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Error {
        Error::from(Context::new(ErrorKind::RusqliteError {
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
