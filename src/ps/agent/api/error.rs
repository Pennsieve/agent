//! Api related errors.
use failure::{Backtrace, Context, Fail};
use std::{fmt, result};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn invalid_folder<S: Into<String>>(folder: S) -> Error {
        ErrorKind::InvalidFolder {
            folder: folder.into(),
        }
        .into()
    }

    pub fn invalid_upload_response<S: Into<String>>(message: S) -> Error {
        ErrorKind::InvalidUploadResponse {
            message: message.into(),
        }
        .into()
    }

    pub fn invalid_upload<S: Into<String>>(message: S) -> Error {
        ErrorKind::InvalidUpload {
            message: message.into(),
        }
        .into()
    }

    pub fn invalid_user_profile<S: Into<String>>(profile: S) -> Error {
        ErrorKind::InvalidUserProfile {
            profile: profile.into(),
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
    #[fail(display = "Invalid command: {}", command)]
    InvalidCommandError { command: String },

    #[fail(display = "No current user")]
    NoUserError,

    #[fail(display = "No user profile found")]
    NoUserProfileError,

    #[fail(display = "Cancelled")]
    UserCancelledError,

    #[fail(display = "User's profile does not exist: {}", profile)]
    InvalidUserProfile { profile: String },

    #[fail(
        display = "Folder does not belong to the specified dataset: {}",
        folder
    )]
    InvalidFolder { folder: String },

    #[fail(display = "Dataset names cannot begin with the reserved string \"N:dataset\"")]
    DatasetReservedName,

    #[fail(display = "Package names cannot begin with the reserved string \"N:package\"")]
    PackageReservedName,

    #[fail(display = "A package must already exist when appending")]
    PackageMustExistForAppending,

    #[fail(display = "A package must be a timeseries package in order to append to it")]
    MustBeATimeseriesPackageToAppendTo,

    #[fail(display = "A dataset or package ID is required")]
    MissingDatasetPackage,

    #[fail(display = "{}", message)]
    InvalidUploadResponse { message: String },

    #[fail(display = "Invalid upload: {}", message)]
    InvalidUpload { message: String },

    #[fail(display = "Pennsieve error: {:?}", error)]
    Pennsieve { error: String },
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
