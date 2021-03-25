//! This module contains functionality specific to file uploading.

mod error;
pub mod worker;

use std::collections::HashMap;
use std::fmt;
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::os::windows::prelude::*;
use std::path::{Path, PathBuf};
use std::slice;
use std::vec;

use pretty_bytes::converter::convert as human_bytes;

use walkdir::WalkDir;

use pennsieve_rust::api::response;
use pennsieve_rust::model::{PackagePreview, UploadId};

use crate::ps::agent::cli::input::confirm;
use crate::ps::agent::config::constants::{
    PREVIEW_DISPLAY_MAX_FILES, PREVIEW_DISPLAY_MAX_PACKAGES,
};

pub use self::error::{Error, ErrorKind, Result};
pub use self::worker::{Props, Uploader};

/// A wrapper around `response::UploadPreview`.
pub struct UploadPreview(response::UploadPreview);

impl From<response::UploadPreview> for UploadPreview {
    fn from(preview: response::UploadPreview) -> UploadPreview {
        UploadPreview(preview)
    }
}

impl IntoIterator for UploadPreview {
    type Item = PackagePreview;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl UploadPreview {
    pub fn iter(&self) -> slice::Iter<'_, PackagePreview> {
        self.0.iter()
    }

    /// Confirms a list of files to upload to the Pennsieve platform. This function
    /// will prompt (block) the user to verify a list of files to be upload, and
    /// if cancelled, will return an error indicating cancellation.
    ///
    /// If the force argument is true, no confirmation prompt will be displayed.
    pub fn display_and_confirm(
        self,
        absolute_path_map: &HashMap<UploadId, PathBuf>,
        dataset_label: String,
        folder_label: Option<String>,
        force: bool,
    ) -> Result<UploadPreview> {
        let count = self.0.file_count();
        let display_label = match folder_label {
            Some(folder) => format!("\"{}\" / \"{}\"", dataset_label, folder),
            None => format!("\"{}\"", dataset_label),
        };

        println!(
            "{count} {files} will be uploaded to \"{label}\":\n",
            count = count,
            files = if count == 1 { "file" } else { "files" },
            label = display_label
        );
        for package in self.0.iter().take(PREVIEW_DISPLAY_MAX_PACKAGES) {
            let pkg_type = match package.package_type() {
                Some(p) => format!("{:?}", p),
                None => "Unknown".to_string(),
            };
            println!(
                "[{name}] ({type}, {size})",
                name = package.package_name(),
                type = pkg_type,
                size = human_bytes(*package.group_size() as f64)
            );

            for file in package.files().iter().take(PREVIEW_DISPLAY_MAX_FILES) {
                let preview_path: Option<String> = package.clone().to_owned().preview_path();
                let absolute_path = file
                    .upload_id()
                    .and_then(|id| absolute_path_map.get(id))
                    .ok_or_else(|| Error::missing_upload_id(file.upload_id().cloned()))?;

                println!(
                    "    {:?} ({}) \n  package directory {:?}",
                    absolute_path,
                    human_bytes(file.size() as f64),
                    preview_path.unwrap_or_else(|| "/".to_string()),
                );
            }
            println!();

            if package.file_count() > PREVIEW_DISPLAY_MAX_FILES {
                println!("  NOTE: This package's file list is truncated for display purposes.");
                println!(
                    "  {} files are displayed here, there are {} additional files in this package ({} total).",
                    PREVIEW_DISPLAY_MAX_FILES, package.file_count() - PREVIEW_DISPLAY_MAX_FILES, package.file_count()
                );
                println!();
            }
        }

        if self.0.package_count() > PREVIEW_DISPLAY_MAX_PACKAGES {
            println!("NOTE: The package list is truncated for display purposes.");
            println!(
                "{} packages are displayed here, there are {} additional packages that will be uploaded ({} total).",
                PREVIEW_DISPLAY_MAX_PACKAGES, self.0.package_count() - PREVIEW_DISPLAY_MAX_PACKAGES, self.0.package_count()
            );
            println!();
        }

        if force {
            return Ok(self);
        }

        if confirm("Continue?")? {
            Ok(self)
        } else {
            Err(ErrorKind::UserCancelledError.into())
        }
    }
}

/// An opaque type containing the results of a file preview.
pub struct PreviewFiles {
    // The optional base directory that contains all file_paths
    #[allow(dead_code)]
    path: Option<Box<Path>>,
    // The full paths of the files to upload, paired with their
    // respective upload ids
    #[allow(dead_code)]
    file_paths: Vec<(UploadId, PathBuf)>,
}

/// An entry returned by the file preview iterator.
pub struct PreviewItem<'a> {
    // A reference back to the base directory to upload files from:
    #[allow(dead_code)]
    path: Option<&'a Path>,
    // The full path of the file
    full_file_path: PathBuf,
    // The identifier for this file
    upload_id: UploadId,
}

impl<'a> PreviewItem<'a> {
    /// Returns the full path of the preview file.
    pub fn full_path(&self) -> &PathBuf {
        &self.full_file_path
    }

    /// Returns the full path of the preview file.
    pub fn upload_id(&self) -> UploadId {
        self.upload_id
    }
}

impl<'a> fmt::Display for PreviewItem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.full_path())
    }
}

// Iterator state over `PreviewFiles`
pub struct PreviewFilesIter<'a> {
    inner: &'a PreviewFiles,
    file_paths_iter: slice::Iter<'a, (UploadId, PathBuf)>,
}

impl<'a> Iterator for PreviewFilesIter<'a> {
    type Item = PreviewItem<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((id, file_path)) = self.file_paths_iter.next() {
            let path = self.inner.path();
            Some(PreviewItem {
                path,
                full_file_path: file_path.to_path_buf(),
                upload_id: *id,
            })
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a PreviewFiles {
    type Item = PreviewItem<'a>;
    type IntoIter = PreviewFilesIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PreviewFiles {
    pub fn new(path: Option<Box<Path>>, file_paths: Vec<(UploadId, PathBuf)>) -> Result<Self> {
        if let Some(ref unwrapped_path) = path {
            for (_, file_path) in file_paths.clone() {
                if !file_path.starts_with(unwrapped_path) {
                    return Err(Error::invalid_path(format!(
                        "file_path {:?} did not start with provided parent path {:?}",
                        file_path, unwrapped_path
                    )));
                }
            }
        }
        Ok(Self { path, file_paths })
    }

    // https://blog.guillaume-gomez.fr/articles/2017-03-09+Little+tour+of+multiple+iterators+implementation+in+Rust
    /// Returns an immutable iterator of preview files.
    pub fn iter(&self) -> PreviewFilesIter<'_> {
        PreviewFilesIter {
            inner: self,
            file_paths_iter: self.file_paths.iter(),
        }
    }

    /// Returns the base directory the preview was generated from, e.g. `/home/foo/bar`.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(|p| p.as_ref())
    }

    /// Returns a collection of fully resolved file names.
    pub fn file_paths(&self) -> &Vec<(UploadId, PathBuf)> {
        &self.file_paths
    }
}

fn is_hidden_dot_file<P>(file: P) -> bool
where
    P: AsRef<Path>,
{
    match file.as_ref().file_name().and_then(|s| s.to_str()) {
        Some(s) => s.starts_with("."),
        None => false,
    }
}

#[cfg(windows)]
fn is_windows_fs_hidden_file<P>(file: P) -> bool
where
    P: AsRef<Path>,
{
    match fs::metadata(file).ok() {
        Some(metadata) => {
            // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
            let attributes = metadata.file_attributes();
            let FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            (attributes & FILE_ATTRIBUTE_HIDDEN) != 0
        }
        None => false,
    }
}

#[cfg(not(windows))]
fn is_hidden_file<P>(file: P) -> bool
where
    P: AsRef<Path>,
{
    is_hidden_dot_file(file.as_ref())
}

#[cfg(windows)]
fn is_hidden_file<P>(file: P) -> bool
where
    P: AsRef<Path>,
{
    is_windows_fs_hidden_file(file.as_ref()) || is_hidden_dot_file(file.as_ref())
    // ignore dotfiles as well
}

/// Generates a list of files to be uploaded to the Pennsieve platform.
pub fn generate_file_preview<P>(files: Vec<P>, recursive: bool) -> Result<PreviewFiles>
where
    P: AsRef<Path>,
{
    // Canonicalize the given paths:
    let path_bufs: Vec<PathBuf> = files
        .iter()
        .map(|file| {
            file.as_ref()
                .canonicalize()
                .or_else(|_| Err(Error::file_not_found(file.as_ref().to_path_buf())))
        })
        .collect::<Result<Vec<PathBuf>>>()?;

    for buf in &path_bufs {
        if !buf.exists() {
            return Err(Error::file_not_found(buf.to_path_buf()));
        }
    }

    if path_bufs.is_empty() {
        Err(ErrorKind::NoFilesToUpload.into())
    } else if path_bufs.len() == 1 && path_bufs[0].is_dir() {
        // if a single path argument was provided, and that path
        // argument is a directory, we upload all files in the
        // directory (recursively if 'recursive' is true)

        let buf = &path_bufs[0];

        let walk_dir = WalkDir::new(buf).min_depth(1);
        let walk_dir = if recursive {
            walk_dir
        } else {
            walk_dir.max_depth(1)
        };

        // WalkDir returns an iterator over results.
        let file_paths: Vec<PathBuf> = walk_dir
            .into_iter()
            .map(|dir_entry_result| {
                dir_entry_result
                    .map_err(Into::<Error>::into)
                    .and_then(|dir_entry| dir_entry.path().canonicalize().map_err(Into::into))
            })
            .filter(|path_result| {
                path_result
                    .as_ref()
                    .map(|path| path.is_file())
                    .unwrap_or(true)
            })
            .collect::<Result<Vec<PathBuf>>>()?
            .into_iter()
            .filter(|file| !is_hidden_file(file))
            .collect();

        // If we didn't match anything, it should probably be reported as an error:
        if file_paths.is_empty() {
            return Err(ErrorKind::NoFilesToUpload.into());
        }

        let enumerated_file_paths: Vec<(UploadId, PathBuf)> = file_paths
            .into_iter()
            .enumerate()
            .map(|(id, path)| (UploadId::from(id as u64), path))
            .collect();
        PreviewFiles::new(Some(buf.clone().into_boxed_path()), enumerated_file_paths)
    } else {
        for buf in &path_bufs {
            if buf.is_dir() {
                return Err(Error::directory_in_file_upload(buf.to_path_buf()));
            }
        }
        let enumerated_path_bufs = path_bufs
            .into_iter()
            .filter(|file| !is_hidden_file(file))
            .enumerate()
            .map(|(id, path)| (UploadId::from(id as u64), path))
            .collect();
        PreviewFiles::new(None, enumerated_path_bufs)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pennsieve_macros::{path, src_path, test_resources_path};

    #[test]
    fn bad_path_fails() {
        assert!(
            generate_file_preview(vec![src_path!("ps", "agent", "not-real", "upload")], false)
                .is_err()
        );
    }

    #[test]
    fn nonrecursive_include_wildcard_works() {
        let preview =
            generate_file_preview(vec![src_path!("ps", "agent", "upload")], false).unwrap();
        let expected_files = vec![
            src_path!("ps", "agent", "upload", "error.rs"),
            src_path!("ps", "agent", "upload", "mod.rs"),
            src_path!("ps", "agent", "upload", "worker.rs"),
        ];
        let expected_files: Vec<PathBuf> = expected_files
            .iter()
            .map(|f| f.canonicalize().unwrap())
            .collect();
        let mut actual_files: Vec<PathBuf> = preview
            .file_paths()
            .into_iter()
            .map(|(_id, path)| path.clone().canonicalize().unwrap())
            .collect();
        actual_files.sort();
        assert_eq!(&actual_files, &expected_files);
    }

    #[test]
    fn nonrecursive_include_works() {
        let preview =
            generate_file_preview(vec![src_path!("ps", "agent", "upload")], false).unwrap();
        let expected_files = vec![
            src_path!("ps", "agent", "upload", "error.rs"),
            src_path!("ps", "agent", "upload", "mod.rs"),
            src_path!("ps", "agent", "upload", "worker.rs"),
        ];
        let expected_files: Vec<PathBuf> = expected_files
            .iter()
            .map(|f| f.canonicalize().unwrap())
            .collect();
        let mut actual_files: Vec<PathBuf> = preview
            .file_paths()
            .into_iter()
            .map(|(_id, path)| path.clone().canonicalize().unwrap())
            .collect();
        actual_files.sort();
        assert_eq!(&actual_files, &expected_files);
    }

    #[test]
    fn recursive_include_works() {
        let preview =
            generate_file_preview(vec![test_resources_path!("upload_test")], true).unwrap();

        let expected_files = vec![
            test_resources_path!("upload_test/1.txt"),
            test_resources_path!("upload_test/2.txt"),
            test_resources_path!("upload_test/3.txt"),
            test_resources_path!("upload_test/4.txt"),
            test_resources_path!("upload_test/5.txt"),
            test_resources_path!("upload_test/6.txt"),
            test_resources_path!("upload_test/7.txt"),
            test_resources_path!("upload_test/recursive/8.txt"),
            test_resources_path!("upload_test/recursive/layer/layer/9.txt"),
        ];
        let expected_files: Vec<PathBuf> = expected_files
            .iter()
            .map(|f| f.canonicalize().unwrap())
            .collect();

        let mut actual_files: Vec<PathBuf> = preview
            .file_paths()
            .into_iter()
            .map(|(_id, path)| path.clone().canonicalize().unwrap())
            .collect();
        actual_files.sort();
        assert_eq!(&actual_files, &expected_files);
    }

    #[test]
    fn recursive_include_works_for_deeply_nested_directories() {
        let preview =
            generate_file_preview(vec![test_resources_path!("upload_test/recursive")], true)
                .unwrap();

        let expected_files = vec![
            test_resources_path!("upload_test/recursive/8.txt"),
            test_resources_path!("upload_test/recursive/layer/layer/9.txt"),
        ];
        let mut expected_files: Vec<PathBuf> = expected_files
            .iter()
            .map(|f| f.canonicalize().unwrap())
            .collect();
        expected_files.sort_by(|a, b| a.cmp(b));

        let mut actual_files: Vec<PathBuf> = preview
            .file_paths()
            .into_iter()
            .map(|(_id, path)| path.clone().canonicalize().unwrap())
            .collect();

        actual_files.sort_by(|a, b| a.cmp(b));
        assert_eq!(&actual_files, &expected_files);

        assert_eq!(
            preview
                .path()
                .unwrap()
                .to_path_buf()
                .canonicalize()
                .unwrap(),
            test_resources_path!("upload_test/recursive")
                .canonicalize()
                .unwrap()
        );
    }

    #[test]
    fn recursive_include_creates_expected_file_names() {
        let base_path = test_resources_path!("upload_test/recursive");
        let preview = generate_file_preview(vec![base_path], true).unwrap();

        let mut expected_files: Vec<PathBuf> = vec![
            test_resources_path!("upload_test/recursive/layer/layer/9.txt"),
            test_resources_path!("upload_test/recursive/8.txt"),
        ];
        expected_files.sort_by(|a, b| a.cmp(b));

        let mut expected_files: Vec<PathBuf> = expected_files
            .iter()
            .map(|f| f.canonicalize().unwrap())
            .collect();
        expected_files.sort_by(|a, b| a.cmp(b));

        let mut actual_files: Vec<PathBuf> = preview
            .file_paths()
            .into_iter()
            .map(|(_id, path)| path.clone().canonicalize().unwrap())
            .collect();
        actual_files.sort_by(|a, b| a.cmp(b));

        assert_eq!(&actual_files, &expected_files);
    }
}
