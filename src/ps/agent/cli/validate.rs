use log::*;

use crate::ps::agent::api::error::{ErrorKind, Result};
/// Validators for command line input actions.
use crate::ps::agent::api::Validator;
use crate::ps::agent::cli::input;
use crate::ps::util::strings as s;

/// Validates the construction of a dataset.
pub struct Dataset {
    force: bool,
}

impl Dataset {
    pub fn new(force: bool) -> Self {
        Self { force }
    }
}

impl Validator for Dataset {
    /// Given a dataset identifier, validate it, returning a boolean indicating
    /// if the dataset is allowed to be created, or an error occurred during
    /// validation.
    fn validate(&self, identifier: &str) -> Result<bool> {
        if s::looks_like_dataset_node_id(identifier) {
            return Err(ErrorKind::DatasetReservedName.into());
        }
        if self.force {
            return Ok(true);
        }
        println!(
            "\nThe dataset \"{dataset}\" does not exist and will be created.\n",
            dataset = identifier
        );
        input::confirm("Continue?").map(Ok).unwrap_or_else(|e| {
            error!("ps:cli:validate:dataset:confirm ~ {}", e);
            Ok(false)
        })
    }
}

/// Validates the construction of a folder (collection).
pub struct Folder {
    force: bool,
}

impl Folder {
    pub fn new(force: bool) -> Self {
        Self { force }
    }
}

impl Validator for Folder {
    /// Given a dataset identifier, validate it, returning a boolean indicating
    /// if the dataset is allowed to be created, or an error occurred during
    /// validation.
    fn validate(&self, identifier: &str) -> Result<bool> {
        if s::looks_like_package_node_id(identifier) {
            return Err(ErrorKind::PackageReservedName.into());
        }
        if self.force {
            return Ok(true);
        }
        println!(
            "\nThe folder \"{folder}\" does not exist and will be created.\n",
            folder = identifier
        );
        input::confirm("Continue?").map(Ok).unwrap_or_else(|e| {
            error!("ps:cli:validate:folder:confirm ~ {}", e);
            Ok(false)
        })
    }
}
