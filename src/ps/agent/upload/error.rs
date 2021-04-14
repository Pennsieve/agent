
//! Upload related errors
use failure::{Backtrace, Context, Fail};
use std::path::PathBuf;
use std::{fmt, io, result};

use crate::ps::agent::database;
use pennsieve_rust::model::UploadId;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn missing_upload_id(upload_id: Option<UploadId>) -> Error {
        ErrorKind::MissingUploadId { upload_id }.into()
    }

    pub fn file_not_found(missing_file: PathBuf) -> Error {
        ErrorKind::FileNotFound { missing_file }.into()
    }

    pub fn directory_in_file_upload(directory: PathBuf) -> Error {
        ErrorKind::DirectoryInFileUpload { directory }.into()
    }

    pub fn invalid_path<S: Into<String>>(message: S) -> Error {
        ErrorKind::InvalidPath {
            message: message.into(),
        }
        .into()
    }

    pub fn no_parent<S: Into<String>>(path: S) -> Error {
        ErrorKind::NoParent { path: path.into() }.into()
    }

    pub fn upload_failed(cause: pennsieve_rust::Error) -> Error {
        ErrorKind::UploadFailed {
            message: cause.to_string(),
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
    #[fail(display = "UploadId {:?} was not found in path map", upload_id)]
    MissingUploadId { upload_id: Option<UploadId> },

    #[fail(display = "File or directory not found: {:?}", missing_file)]
    FileNotFound { missing_file: PathBuf },

    #[fail(display = "No file(s) given to upload")]
    NoFilesToUpload,

    #[fail(
        display = "When using multiple paths, all paths must be files. A directory was provided: {:?}",
        directory
    )]
    DirectoryInFileUpload { directory: PathBuf },

    #[fail(display = "Invalid path: {}", message)]
    InvalidPath { message: String },

    #[fail(display = "Cancelled")]
    UserCancelledError,

    #[fail(display = "Couldn't extract parent from path: {}", path)]
    NoParent { path: String },

    #[fail(display = "Upload failed: {}", message)]
    UploadFailed { message: String },

    #[fail(display = "{}", kind)]
    Pennsieve { kind: pennsieve_rust::ErrorKind },

    #[fail(display = "Database error: {}", kind)]
    DatabaseError { kind: database::ErrorKind },

    #[fail(display = "I/O error: {}", error)]
    IoError { error: String },

    #[fail(display = "Shell expansion error: {}", error)]
    GlobsetError { error: String },

    #[fail(display = "Directory recursion error: {}", error)]
    WalkdirError { error: String },
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

// map from pennsieve errors
impl From<pennsieve_rust::ErrorKind> for Error {
    fn from(kind: pennsieve_rust::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::Pennsieve { kind }))
    }
}
impl From<pennsieve_rust::Error> for Error {
    fn from(error: pennsieve_rust::Error) -> Error {
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

/// map from IO errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from globset errors
impl From<globset::Error> for Error {
    fn from(error: globset::Error) -> Error {
        Error::from(Context::new(ErrorKind::GlobsetError {
            error: error.to_string(),
        }))
    }
}

/// map from walkdir errors
impl From<walkdir::Error> for Error {
    fn from(error: walkdir::Error) -> Error {
        Error::from(Context::new(ErrorKind::WalkdirError {
            error: error.to_string(),
        }))
    }
}
