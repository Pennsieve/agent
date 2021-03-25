//! The database layer that uses SQLite for persistence.

use std::env;
use std::path::{Path, PathBuf};
use std::slice;
use std::str::FromStr;
use std::vec::IntoIter;
use std::{fmt, result};

use log::*;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, Row, NO_PARAMS};
use serde_derive::Serialize;
use time;

mod error;

use pennsieve_rust::Environment as ApiEnvironment;
use pennsieve_migrations::Migrations;

// Re-export:
pub use self::error::{Error, ErrorKind, Result};
use crate::ps::agent::config;

/// Unique id used as a primary key for the user record table.
/// This is used to support only one login at a time.
const USER_INNER_ID: i32 = 1;

/// Source used to configure which backing scheme to use for SQLite.
#[derive(Debug)]
pub enum Source {
    // A file backed database.
    File(PathBuf),
    // In memory database.
    // NOTE: this type should not be used. There seems to be a bug
    // where calling `.clone()`, on the resource pool, creates a new
    // in memory database, as opposed to allowing multiple connections
    // to the same one.
    //Memory,
}

///////////////////////////////////////////////////////////////////////////////
/// BIG TODO: Investigate migrating all database code to use diesel orm.
///////////////////////////////////////////////////////////////////////////////

/// A page record is the backing structure for specifying a reference
/// to a timeseries cache page on the local file system. The `id` is
/// used as a file path. A `nan_filled` value of `true` means that
/// all values that would be contained on that page are NotANumber (NAN).
/// This means that the page does not need to be backed on the local
/// file system.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageRecord {
    pub id: String,
    pub nan_filled: bool,
    pub complete: bool,
    pub size: i64,
    pub last_used: time::Timespec,
}

impl PageRecord {
    pub fn new<I>(id: I, nan_filled: bool, complete: bool, size: i64) -> Self
    where
        I: Into<String>,
    {
        Self {
            id: id.into(),
            nan_filled,
            complete,
            size,
            last_used: time::now().to_timespec(),
        }
    }

    // private - used only in this module
    fn from_row(row: &Row<'_, '_>) -> Result<Self> {
        Ok(Self {
            id: row.get(0),
            nan_filled: row.get(1),
            complete: row.get(2),
            size: row.get(3),
            last_used: row.get(4),
        })
    }

    pub fn str_time(&self) -> String {
        time::strftime("%Y-%m-%dT%H:%M:%SZ", &time::at(self.last_used))
            .unwrap_or_else(|_| String::from("invalid time format"))
    }
}

/// A user record is a login profile that is used for saving
/// user information. This profile is used by cli commands and
/// the background upload worker. Currently, only one of these can
/// exist in the database at a time. This is controlled by the use
/// of a hidden id field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserRecord {
    pub id: String,
    pub name: String,
    pub session_token: String,
    pub profile: String,
    pub environment: ApiEnvironment,
    pub organization_id: String,
    pub organization_name: String,
    pub encryption_key: String,
    pub updated_at: time::Timespec,
}

impl UserRecord {
    #[allow(unknown_lints, clippy::too_many_arguments)]
    pub fn new<I, N, S, P, O, R, E>(
        id: I,
        name: N,
        session_token: S,
        profile: P,
        environment: ApiEnvironment,
        organization_id: O,
        organization_name: R,
        encryption_key: E,
    ) -> Self
    where
        I: Into<String>,
        N: Into<String>,
        S: Into<String>,
        P: Into<String>,
        O: Into<String>,
        R: Into<String>,
        E: Into<String>,
    {
        Self {
            id: id.into(),
            name: name.into(),
            session_token: session_token.into(),
            profile: profile.into(),
            environment,
            organization_id: organization_id.into(),
            organization_name: organization_name.into(),
            encryption_key: encryption_key.into(),
            updated_at: time::now().to_timespec(),
        }
    }

    // private - used only in this module
    fn from_row(row: &Row<'_, '_>) -> Result<Self> {
        row.get::<usize, String>(4)
            .parse::<ApiEnvironment>()
            .map(|env| Self {
                id: row.get(0),
                name: row.get(1),
                session_token: row.get(2),
                profile: row.get(3),
                environment: env,
                organization_id: row.get(5),
                organization_name: row.get(6),
                encryption_key: row.get(7),
                updated_at: row.get(8),
            })
            .map_err(|_| {
                config::Error::invalid_api_config(format!(
                    "invalid environment: {}",
                    row.get::<usize, String>(4)
                ))
                .into()
            })
    }

    /// Returns a boolean value based on whether the Pennsieve
    /// session token is valid or not. The Pennsieve api authorizes these
    /// tokens for two hours, just to be safe, a value of 90 minutes is used
    /// to timeout our representation of the session token.
    pub fn is_token_valid(&self) -> bool {
        // tokens last for 2 hours..just to be safe we will use 90 minutes
        let expires = self.updated_at + time::Duration::minutes(90);

        expires.gt(&time::now().to_timespec())
    }
}

/// Changeable user-specific settings, like persistent dataset, etc.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserSettings {
    pub use_dataset_id: Option<String>,
}

impl Default for UserSettings {
    fn default() -> UserSettings {
        UserSettings {
            use_dataset_id: None,
        }
    }
}

impl UserSettings {
    #[allow(dead_code)]
    fn new<P>(use_dataset_id: Option<P>) -> Self
    where
        P: Into<String>,
    {
        Self {
            use_dataset_id: use_dataset_id.map(Into::into),
        }
    }

    // private - only used in this module
    fn from_row(row: &Row<'_, '_>) -> Result<Self> {
        Ok(Self {
            use_dataset_id: row.get(0),
        })
    }

    pub fn with_dataset(self, use_dataset_id: Option<String>) -> Self {
        Self { use_dataset_id }
    }
}

/// States for upload records.
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum UploadStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
}

impl AsRef<str> for UploadStatus {
    fn as_ref(&self) -> &str {
        use self::UploadStatus::*;
        match *self {
            Queued => "queued",
            InProgress => "in_progress",
            Completed => "completed",
            Failed => "failed",
        }
    }
}

impl From<UploadStatus> for String {
    fn from(s: UploadStatus) -> Self {
        s.as_ref().to_owned()
    }
}

impl FromStr for UploadStatus {
    type Err = Error;
    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "queued" => Ok(UploadStatus::Queued),
            "in_progress" => Ok(UploadStatus::InProgress),
            "completed" => Ok(UploadStatus::Completed),
            "failed" => Ok(UploadStatus::Failed),
            s => Err(Error::status(s)),
        }
    }
}

impl fmt::Display for UploadStatus {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::UploadStatus::*;
        write!(
            fmt,
            "{}",
            match *self {
                Queued => "QUEUED",
                InProgress => "IN_PROGRESS",
                Completed => "COMPLETED",
                Failed => "FAILED",
            }
        )
    }
}

/// An upload record represents a single file, defined on the local filesystem,
/// that will be uploaded to the Pennsieve platform. The cli places records into
/// this table and the upload worker reads them and attempts to upload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UploadRecord {
    pub id: Option<i64>,
    pub file_path: String,
    pub dataset_id: String,
    pub package_id: Option<String>,
    pub import_id: String,
    pub progress: i32,
    pub status: UploadStatus,
    pub created_at: time::Timespec,
    pub updated_at: time::Timespec,
    pub append: bool,
    pub upload_service: bool,
    pub organization_id: String,
    pub chunk_size: Option<u64>,
    pub multipart_upload_id: Option<String>,
}

impl UploadRecord {
    /// Creates a new upload record and gives it a starting state of
    /// `UploadStatus::Queued`.
    #[allow(clippy::too_many_arguments)]
    pub fn new<P, D, K, I, O>(
        file_path: P,
        dataset_id: D,
        package_id: Option<K>,
        organization_id: O,
        import_id: I,
        append: bool,
        chunk_size: Option<u64>,
        multipart_upload_id: Option<String>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        D: Into<String>,
        K: Into<String>,
        I: Into<String>,
        O: Into<String>,
    {
        if let Some(path) = file_path.as_ref().to_str() {
            Ok(Self {
                id: None,
                file_path: path.into(),
                dataset_id: dataset_id.into(),
                package_id: package_id.map(Into::into),
                import_id: import_id.into(),
                progress: 0,
                status: UploadStatus::Queued,
                created_at: time::now().to_timespec(),
                updated_at: time::now().to_timespec(),
                append,
                upload_service: true,
                organization_id: organization_id.into(),
                chunk_size,
                multipart_upload_id,
            })
        } else {
            Err(Error::path(file_path.as_ref().to_path_buf()))
        }
    }

    // private - only used in this module
    fn from_row(row: &Row<'_, '_>) -> Result<Self> {
        let status: String = row.get(6);
        let status: UploadStatus = status.parse()?;

        let chunk_size: Option<u32> = row.get(12);
        let chunk_size: Option<u64> = chunk_size.map(u64::from);

        Ok(Self {
            id: Some(row.get(0)),
            file_path: row.get(1),
            dataset_id: row.get(2),
            package_id: row.get(3),
            import_id: row.get(4),
            progress: row.get(5),
            status,
            created_at: row.get(7),
            updated_at: row.get(8),
            append: row.get(9),
            upload_service: row.get(10),
            organization_id: row.get(11),
            chunk_size,
            multipart_upload_id: row.get(13),
        })
    }

    /// Returns a boolean specifying whether this upload should be retried.
    /// There's a 1 hour threshold for when records can be retried. This
    /// threshold is based on the records `updated_at` time.
    pub fn should_retry(&self) -> bool {
        // retry if the upload has been "in_progress" for 1 hour
        let threshold = self.updated_at + time::Duration::hours(1);

        time::now().to_timespec().gt(&threshold)
    }

    /// Returns a boolean specifying whether this upload failed.
    /// There's a window of 8 hours that a record can be retried in,
    /// outside of this window it is considered failed.
    /// The threshold is based on the records `created_at` time.
    pub fn should_fail(&self) -> bool {
        // uploads fail if it was created more than 8 hours
        let threshold = self.created_at + time::Duration::hours(8);

        time::now().to_timespec().gt(&threshold)
    }

    /// Tests if the upload failed.
    pub fn is_failed(&self) -> bool {
        use self::UploadStatus::*;
        match self.status {
            Failed => true,
            _ => false,
        }
    }

    /// Tests if the package containing this upload completed successfully.
    pub fn is_package_completed(&self) -> bool {
        use self::UploadStatus::*;
        match self.status {
            Completed => true,
            _ => false,
        }
    }

    /// Tests if the file corresponding to this upload record has
    /// uploaded successfully
    pub fn is_file_upload_completed(&self) -> bool {
        self.progress >= 100
    }

    /// Tests if the upload is still queued for processing.
    pub fn is_queued(&self) -> bool {
        use self::UploadStatus::*;
        match self.status {
            Queued => true,
            _ => false,
        }
    }

    /// Tests if the upload is still being processed.
    pub fn is_in_progress(&self) -> bool {
        use self::UploadStatus::*;
        match self.status {
            InProgress => true,
            _ => false,
        }
    }

    /// Generate a summary of the upload record of the form:
    ///   "{file_path} - {progress}%"
    pub fn summary(&self) -> String {
        format!(
            "{file_path} - {progress}%",
            file_path = self.file_path,
            progress = self.progress
        )
    }
}

/// A container for active (queued and in-progress) upload records.
pub struct UploadRecords {
    pub records: Vec<UploadRecord>,
}

impl UploadRecords {
    /// Returns the total number of uploads, regardless of status.
    pub fn len(&self) -> u64 {
        self.records.len() as u64
    }

    /// Returns whether the total number of uploads is 0
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Tests if the set of files to be uploaded is considered completed.
    /// Completion occurs if:
    ///
    /// - this set is empty
    /// - the package(s) containing all files found in this set are
    ///   considered completed or failed (no queued or in_progress)
    pub fn is_package_completed(&self) -> bool {
        self.records.is_empty()
            || self.records.iter().all(|ref u| {
                u.status == UploadStatus::Failed || u.status == UploadStatus::Completed
            })
    }

    /// Tests if the given file path is part of the upload file set.
    pub fn contains_file(&self, file_path: &str) -> bool {
        self.records.iter().any(|ref u| u.file_path.eq(file_path))
    }
}

impl From<Vec<UploadRecord>> for UploadRecords {
    fn from(records: Vec<UploadRecord>) -> Self {
        Self { records }
    }
}

/// An immutable iterator of `UploadRecord`.
pub struct UploadRecordsIter<'a> {
    inner: &'a UploadRecords,
    iter: slice::Iter<'a, UploadRecord>,
}

impl<'a> UploadRecordsIter<'a> {
    /// Returns a reference to the `UploadRecords` object backing this iterator.
    pub fn uploads(&'a self) -> &'a UploadRecords {
        self.inner
    }
}

/// A mutable iterator of `UploadRecord`.
pub struct UploadRecordsIterMut<'a> {
    iter: slice::IterMut<'a, UploadRecord>,
}

impl UploadRecords {
    pub fn iter(&self) -> UploadRecordsIter<'_> {
        UploadRecordsIter {
            inner: self,
            iter: self.records.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> UploadRecordsIterMut<'_> {
        UploadRecordsIterMut {
            iter: self.records.iter_mut(),
        }
    }

    pub fn into_owned_iter(self) -> IntoIter<UploadRecord> {
        self.records.into_iter()
    }
}

impl<'a> Iterator for UploadRecordsIter<'a> {
    type Item = &'a UploadRecord;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a> Iterator for UploadRecordsIterMut<'a> {
    type Item = &'a mut UploadRecord;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a> IntoIterator for &'a UploadRecords {
    type Item = &'a UploadRecord;
    type IntoIter = UploadRecordsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A type that contains a pool of SQLite connections.
/// This type is not only safe to clone, that is the method in
/// which access to the underlying pool is managed. Calling `.clone()`
/// should be preferred over wrapping this type in a container, like `Rc<Database>`.
#[derive(Clone)]
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Database {{ pool: {:?} }}", self.pool.state())
    }
}

impl Database {
    /// Creates a new database based on the provided source.
    pub fn new(source: &Source) -> Result<Database> {
        let manager = match *source {
            Source::File(ref path) => SqliteConnectionManager::file(path),
        };
        let pool = Pool::new(manager)?;
        let database = Database { pool };

        database.setup()?;
        Ok(database)
    }

    // Creates the database tables based on `CREATE TABLE IF NOT EXISTS` logic.
    fn setup(&self) -> Result<usize> {
        let conn = self.pool.get()?;

        let mut count = conn.execute(
            "CREATE TABLE IF NOT EXISTS page_record (
                id VARCHAR(255) PRIMARY KEY,
                nan_filled BOOLEAN,
                complete BOOLEAN,
                size INTEGER,
                last_used VARCHAR(255) NOT NULL
            )",
            NO_PARAMS,
        )?;
        count += conn.execute(
            "CREATE INDEX IF NOT EXISTS page_record_i1 ON page_record (nan_filled, last_used)",
            NO_PARAMS,
        )?;
        count += conn.execute(
            "CREATE TABLE IF NOT EXISTS user_record (
                inner_id INTEGER PRIMARY KEY,
                id VARCHAR(255) NOT NULL,
                name VARCHAR(255) NOT NULL,
                session_token VARCHAR(255) NOT NULL,
                profile VARCHAR(255) NOT NULL,
                environment VARCHAR(10) NOT NULL,
                organization_id VARCHAR(255) NOT NULL,
                organization_name VARCHAR(255) NOT NULL,
                encryption_key VARCHAR(255) NOT NULL,
                updated_at VARCHAR(255) NOT NULL
            )",
            NO_PARAMS,
        )?;
        count += conn.execute(
            "CREATE TABLE IF NOT EXISTS user_settings (
                    user_id VARCHAR(255) NOT NULL,
                    profile VARCHAR(255) NOT NULL,
                    use_dataset_id VARCHAR(255) NULL,
                    PRIMARY KEY (user_id, profile)
             )",
            NO_PARAMS,
        )?;
        count += conn.execute(
            "CREATE TABLE IF NOT EXISTS upload_record (
                id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL,
                dataset_id VARCHAR(255) NOT NULL,
                package_id VARCHAR(255),
                import_id VARCHAR(255) NOT NULL,
                progress INTEGER,
                status VARCHAR(255) NOT NULL,
                created_at VARCHAR(255) NOT NULL,
                updated_at VARCHAR(255) NOT NULL
            )",
            NO_PARAMS,
        )?;
        count += conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_updates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checked_at VARCHAR(255) NOT NULL
            )",
            NO_PARAMS,
        )?;

        // Sometimes if a user's agent.db file is messed up, we need to forego
        // running migrations so a manual repair can take place.
        let disable_migrations = env::var("DISABLE_MIGRATIONS").is_ok();

        if disable_migrations {
            debug!("DISABLE RUNNING MIGRATIONS");
        } else {
            Self::run_migrations(&conn)?;
        }

        count += conn.execute(
            "CREATE INDEX IF NOT EXISTS upload_record_i1 ON upload_record (import_id, file_path)",
            NO_PARAMS,
        )?;

        count += conn.execute(
            "CREATE INDEX IF NOT EXISTS upload_record_i2 ON upload_record (status, created_at)",
            NO_PARAMS,
        )?;

        count += conn.execute(
            "CREATE INDEX IF NOT EXISTS agent_updates_i1 ON agent_updates (checked_at)",
            NO_PARAMS,
        )?;

        Ok(count as usize)
    }

    /// Get the version of the schema using SQLite's "PRAGMA user_version"
    /// feature.
    fn internal_get_schema_version(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Result<usize> {
        conn.query_row("PRAGMA user_version", NO_PARAMS, |row| {
            let version: u32 = row.get(0);
            version as usize
        })
        .map_err(Into::into)
    }

    /// Get the version of the CLI database schema.
    pub fn get_schema_version(&self) -> Result<usize> {
        Self::internal_get_schema_version(&self.pool.get()?)
    }

    /// Increment the schema version, returning the new version.
    fn internal_set_schema_version(
        conn: &PooledConnection<SqliteConnectionManager>,
        version: usize,
    ) -> Result<usize> {
        conn.execute_named(format!("PRAGMA user_version = {}", version).as_str(), &[])
            .map_err(Into::into)
    }

    /// Increment the schema version, returning the new version.
    pub fn set_schema_version(&self, version: usize) -> Result<usize> {
        Self::internal_set_schema_version(&self.pool.get()?, version)
    }

    /// Run the migrations in the `<PROJECT_ROOT>/migrations/sql` directory.
    fn run_migrations(conn: &PooledConnection<SqliteConnectionManager>) -> Result<usize> {
        let mut latest_version: usize = 0;

        // NOTE: `i` starts from 0; by default SQLite's `PRAGMA user_version` is
        // 0 as well. The name of the migration is just used for sorting order;
        // its name does not factor into versioning.
        //
        // Example logging output:
        //
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: 000001_upload_records_add_append.sql@0
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: CURRENT VERSION = 0
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: Running 000001_upload_records_add_append.sql@0
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: LATEST VERSION = 1
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: 000002_upload_records_add_upload_service.sql@1
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: CURRENT VERSION = 1
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: Running 000002_upload_records_add_upload_service.sql@1
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: LATEST VERSION = 2
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: 000003_upload_records_add_org_id.sql@2
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: CURRENT VERSION = 2
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: Running 000003_upload_records_add_org_id.sql@2
        // [DEBUG][pennsieve_rust::ps::agent::database][main] MIGRATION: LATEST VERSION = 3
        for (i, (filename, contents)) in Migrations::get_all().enumerate() {
            debug!(
                "MIGRATION: {filename}@{version}",
                filename = filename,
                version = i
            );
            // ^ using `for` vs `.for_each()` allows `?` shortcircuiting to work:
            let current_version = Self::internal_get_schema_version(conn)?;
            debug!(
                "MIGRATION: CURRENT VERSION = {version}",
                version = current_version
            );
            if current_version <= i {
                debug!(
                    "MIGRATION: Running {filename}@{version}",
                    filename = filename,
                    version = i
                );
                conn.execute_batch(contents.as_ref())
                    .map_err(|e| Error::migration(current_version, e.to_string(), contents))?;
                latest_version = i + 1;
                Self::internal_set_schema_version(conn, latest_version)?;
                debug!(
                    "MIGRATION: LATEST VERSION = {version}",
                    version = latest_version
                );
            } else {
                debug!(
                    "MIGRATION: Skipping {filename}@{version}",
                    filename = filename,
                    version = i
                );
            }
        }

        Ok(latest_version)
    }

    // ----------
    // start of page_record table functions
    // ----------

    /// Insert a page into the database. Ignores records that already exist.
    pub fn upsert_page(&self, record: &PageRecord) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO page_record (id, nan_filled, complete, size, last_used)
             VALUES (:id, :nan_filled, :complete, :size, :last_used)",
        )?;

        stmt.execute_named(&[
            (":id", &record.id),
            (":nan_filled", &record.nan_filled),
            (":complete", &record.complete),
            (":size", &record.size),
            (":last_used", &record.last_used),
        ])
        .map(|count| count as usize)
        .map_err(Into::into)
    }

    /// Writes a NaN filled page to the database. Replaces records that already exist,
    /// this is done to override records that had a temp record inserted. A NaN filled page
    /// is a terminal page state, it cannot go from NaN filled to non NaN filled.
    pub fn write_nan_filled(&self, id: &str, complete: bool) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO page_record (id, nan_filled, complete, size, last_used)
             VALUES (:id, :nan_filled, :complete, :size, :last_used)",
        )?;

        stmt.execute_named(&[
            (":id", &id),
            (":nan_filled", &true),
            (":complete", &complete),
            (":size", &0),
            (":last_used", &time::now().to_timespec()),
        ])
        .map(|count| count as usize)
        .map_err(Into::into)
    }

    /// Updates the `last_used` field, to the current time, for the
    /// provided `id`.
    pub fn touch_last_used(&self, id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("UPDATE page_record SET last_used = :last_used WHERE id = :id")?;

        stmt.execute_named(&[(":id", &id), (":last_used", &time::now().to_timespec())])
            .map(|count| count as usize)
            .map_err(Into::into)
    }

    /// Return a page record based on the provided `id`.
    pub fn get_page(&self, id: &str) -> Result<PageRecord> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, nan_filled, complete, size, last_used
             FROM page_record WHERE id = :id",
        )?;
        let mut rows = stmt.query_named(&[(":id", &id)])?;

        rows.next()
            .ok_or_else(|| Into::<Error>::into(ErrorKind::QueryReturnedNoRows))
            .and_then(|res| {
                res.map(|r| PageRecord {
                    id: r.get(0),
                    nan_filled: r.get(1),
                    complete: r.get(2),
                    size: r.get(3),
                    last_used: r.get(4),
                })
                .map_err(Into::into)
            })
    }

    /// Returns the total size of the cached pages on the local filesystem,
    /// in bytes.
    pub fn get_total_size(&self) -> Result<i64> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT COALESCE(SUM(size), 0) FROM page_record")?;
        let mut rows = stmt.query(NO_PARAMS)?;

        if let Some(res) = rows.next() {
            res.map(|r| r.get(0)).map_err(Into::into)
        } else {
            Ok(0)
        }
    }

    /// Deletes the provided page record from the database.
    pub fn delete_page(&self, record: &PageRecord) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("DELETE FROM page_record WHERE id = :id")?;

        stmt.execute_named(&[(":id", &record.id)])
            .map(|count| count as usize)
            .map_err(Into::into)
    }

    fn get_aged_pages_helper(&self, threshold: &time::Timespec) -> Result<IntoIter<PageRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, nan_filled, complete, size, last_used
             FROM page_record
             WHERE nan_filled = :false AND last_used < :threshold
             ORDER BY last_used ASC",
        )?;
        let rows = stmt.query_and_then_named(
            &[(":false", &false), (":threshold", threshold)],
            PageRecord::from_row,
        )?;

        let mut records = Vec::new();
        for record in rows {
            records.push(record?);
        }

        Ok(records.into_iter())
    }

    /// Gets cached pages that have a `last_used` time of greater than a week
    /// ago.
    pub fn get_soft_aged_pages(&self) -> Result<IntoIter<PageRecord>> {
        let threshold = time::now().to_timespec() - time::Duration::weeks(1);

        self.get_aged_pages_helper(&threshold)
    }

    /// Gets cached pages that have a `last_used` time of greater than 12
    /// hours ago.
    pub fn get_hard_aged_pages(&self) -> Result<IntoIter<PageRecord>> {
        let threshold = time::now().to_timespec() - time::Duration::hours(12);

        self.get_aged_pages_helper(&threshold)
    }

    /// Returns a boolean based on if the provided `id` is associated with
    /// a NaN filled page.
    pub fn is_page_nan(&self, id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT nan_filled FROM page_record WHERE id = :id")?;
        let mut rows = stmt.query_named(&[(":id", &id)])?;

        if let Some(res) = rows.next() {
            res.map(|r| r.get(0)).map_err(Into::into)
        } else {
            Ok(false)
        }
    }

    /// Returns a boolean based on if the provided `id` is associated with
    /// a record that is present and has a complete field of `true`. Having
    /// a complete field of `false` means that that page is on the local
    /// filesystem, but the end time for the page was greater than the
    /// current time, at the time the data was requested. This means that
    /// there could have been additions to it since it was cached.
    pub fn is_page_cached(&self, id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT complete FROM page_record WHERE id = :id")?;
        let mut rows = stmt.query_named(&[(":id", &id)])?;

        if let Some(res) = rows.next() {
            res.map(|r| r.get(0)).map_err(Into::into)
        } else {
            Ok(false)
        }
    }

    // ----------
    // start of `user_record` and `user_settings` table functions
    // ----------

    /// Upserts the provided user into the database.
    pub fn upsert_user(&self, record: &mut UserRecord) -> Result<usize> {
        let conn = self.pool.get()?;
        record.updated_at = time::now().to_timespec();

        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO user_record (inner_id,
                                                 id,
                                                 name,
                                                 session_token,
                                                 profile,
                                                 environment,
                                                 organization_id,
                                                 organization_name,
                                                 encryption_key,
                                                 updated_at)
             VALUES (:inner_id,
                     :id,
                     :name,
                     :session_token,
                     :profile,
                     :environment,
                     :organization_id,
                     :organization_name,
                     :encryption_key,
                     :updated_at)",
        )?;

        stmt.execute_named(&[
            (":inner_id", &USER_INNER_ID),
            (":id", &record.id),
            (":name", &record.name),
            (":session_token", &record.session_token),
            (":profile", &record.profile),
            (":environment", &record.environment.to_string()),
            (":organization_id", &record.organization_id),
            (":organization_name", &record.organization_name),
            (":encryption_key", &record.encryption_key),
            (":updated_at", &record.updated_at),
        ])
        .map(|count| count as usize)
        .map_err(Into::into)
    }

    /// Returns the user record that is currently in the database.
    /// There is only 0 or 1 in the database at any point in time.
    pub fn get_user(&self) -> Result<Option<UserRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    name,
                    session_token,
                    profile,
                    environment,
                    organization_id,
                    organization_name,
                    encryption_key,
                    updated_at
             FROM user_record
             WHERE inner_id = :inner_id
             LIMIT 1",
        )?;
        let mut rows =
            stmt.query_and_then_named(&[(":inner_id", &USER_INNER_ID)], UserRecord::from_row)?;

        rows.next().map_or(Ok(None), |u| u.map(Some))
    }

    pub fn delete_user(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM user_record", NO_PARAMS)?;
        Ok(())
    }

    /// Returns the user settings that is currently in the database.
    /// There is only 0 or 1 in the database at any point in time.
    fn get_user_settings(&self, user_id: &str, profile: &str) -> Result<Option<UserSettings>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT use_dataset_id
             FROM user_record U INNER JOIN user_settings S ON U.id = S.user_id
             WHERE S.user_id = :user_id AND S.profile = :profile
             LIMIT 1",
        )?;
        let mut rows = stmt.query_and_then_named(
            &[(":user_id", &user_id), (":profile", &profile)],
            UserSettings::from_row,
        )?;

        rows.next().map_or(Ok(None), |u| u.map(Some))
    }

    /// Like `get_user_settings`, but creates and stores a new `UserSettings`
    /// object if no settings are found for the specified user.
    pub fn get_or_create_user_settings(
        &self,
        user_id: &str,
        profile: &str,
    ) -> Result<UserSettings> {
        self.get_user_settings(user_id, profile)
            .and_then(|maybe_settings| match maybe_settings {
                Some(settings) => Ok(settings),
                None => {
                    let settings: UserSettings = Default::default();
                    self.upsert_user_settings(user_id, profile, &settings)
                        .map(|_| settings)
                }
            })
    }

    /// Updates the settings associated with a user.
    pub fn upsert_user_settings(
        &self,
        user_id: &str,
        profile: &str,
        user_settings: &UserSettings,
    ) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO user_settings (user_id, profile, use_dataset_id)
                     VALUES (:user_id, :profile, :use_dataset_id)",
        )?;
        stmt.execute_named(&[
            (":user_id", &user_id),
            (":profile", &profile),
            (":use_dataset_id", &user_settings.use_dataset_id),
        ])
        .map(|count| count as usize)
        .map_err(Into::into)
    }

    // ----------
    // start of upload_record table functions
    // ----------

    /// Update upload records with the provided `status`, for all records
    /// associated with the provided `import_id`. On success, returns the
    /// number of updated records.
    pub fn update_import_status(&self, import_id: &str, status: UploadStatus) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "UPDATE upload_record
             SET status = :status, updated_at = :updated_at
             WHERE import_id = :import_id",
        )?;

        stmt.execute_named(&[
            (":import_id", &import_id),
            (":status", &Into::<String>::into(status)),
            (":updated_at", &time::now().to_timespec()),
        ])
        .map(|count| count as usize)
        .map_err(Into::into)
    }

    /// Update upload records with the provided `status` and `progress`,
    /// for all records associated with the provided `import_id`.
    /// On success, returns the number of updated records.
    pub fn update_import_status_and_progress(
        &self,
        import_id: &str,
        status: UploadStatus,
        progress: i32,
    ) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "UPDATE upload_record
             SET status = :status, updated_at = :updated_at, progress = :progress
             WHERE import_id = :import_id",
        )?;

        stmt.execute_named(&[
            (":import_id", &import_id),
            (":progress", &progress),
            (":status", &Into::<String>::into(status)),
            (":updated_at", &time::now().to_timespec()),
        ])
        .map(|count| count as usize)
        .map_err(Into::into)
    }

    /// Updates the upload record associated with a particular file
    /// with the provided `progress` value, only if the provided value
    /// is greater than the existing value in the database (progress
    /// is not allowed to go down). On success, returns the number of
    /// updated records.
    pub fn update_file_progress<P>(
        &self,
        import_id: &str,
        file_path: P,
        progress: i32,
    ) -> Result<usize>
    where
        P: AsRef<Path>,
    {
        if let Some(path) = file_path.as_ref().to_str() {
            let conn = self.pool.get()?;
            let mut stmt = conn.prepare(
                "UPDATE upload_record
                 SET updated_at = :updated_at, status = 'in_progress', progress = :progress
                 WHERE import_id = :import_id AND file_path = :file_path AND progress < :progress",
            )?;

            stmt.execute_named(&[
                (":import_id", &import_id),
                (":file_path", &Into::<String>::into(path)),
                (":progress", &progress),
                (":updated_at", &time::now().to_timespec()),
            ])
            .map(|count| count as usize)
            .map_err(Into::into)
        } else {
            Err(Error::path(file_path.as_ref().to_path_buf()))
        }
    }

    /// Inserts the provided upload into the database. On success, returns the
    /// identifier of the inserted record.
    pub fn insert_upload(&self, record: &UploadRecord) -> Result<i64> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "INSERT INTO upload_record (file_path, dataset_id, package_id, import_id, progress, status, created_at, updated_at, append, upload_service, organization_id, chunk_size, multipart_upload_id)
             VALUES (:file_path, :dataset_id, :package_id, :import_id, :progress, :status, :created_at, :updated_at, :append, :upload_service, :organization_id, :chunk_size, :multipart_upload_id)"
        )?;

        stmt.execute_named(&[
            (":file_path", &record.file_path),
            (":dataset_id", &record.dataset_id),
            (":package_id", &record.package_id),
            (":import_id", &record.import_id),
            (":progress", &record.progress),
            (":status", &Into::<String>::into(record.status)),
            (":created_at", &record.created_at),
            (":updated_at", &record.updated_at),
            (":append", &record.append),
            (":upload_service", &record.upload_service),
            (":organization_id", &record.organization_id),
            (":chunk_size", &record.chunk_size.map(|c| c.to_string())),
            (":multipart_upload_id", &record.multipart_upload_id),
        ])
        .map_err(Into::into)
        .and_then(|_| Ok(conn.last_insert_rowid()))
    }

    /// Resets uploads that are "stalled" with an `in_progress` status back
    /// to that of `queued`. This is meant to be used when the Pennsieve agent
    /// is stopped mid-upload.
    ///
    /// If the upload is not meant for the upload_service, reset the
    /// progress back to 0 as well.
    pub fn reset_stalled_uploads(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut global_stmt = conn.prepare(
            "UPDATE upload_record
             SET status = 'queued'
             WHERE status = 'in_progress'",
        )?;
        let mut non_upload_service_stmt = conn.prepare(
            "UPDATE upload_record
             SET progress = 0
             WHERE status = 'in_progress' AND upload_service = false",
        )?;

        global_stmt
            .execute(NO_PARAMS)
            .map(|count| count as usize)
            .map_err(Into::into)
            .and_then(|global_count| {
                non_upload_service_stmt
                    .execute(NO_PARAMS)
                    .map(|_| global_count)
                    .map_err(Into::into)
            })
    }

    /// Returns all upload records associated with the provided `import_id`.
    pub fn get_uploads_by_import_id(&self, import_id: &str) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE import_id = :import_id",
        )?;
        let records = stmt
            .query_and_then_named(&[(":import_id", &import_id)], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Returns the upload record associated with the provided `upload_id`.
    pub fn get_upload_by_upload_id(&self, upload_id: usize) -> Result<UploadRecord> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE id = :upload_id",
        )?;

        let result_rows = stmt
            .query_and_then_named(
                &[(":upload_id", &upload_id.to_string())],
                UploadRecord::from_row,
            )?
            .collect::<Result<Vec<_>>>()?;

        if result_rows.is_empty() {
            Err(Error::upload_not_found(upload_id))
        } else {
            Ok(result_rows[0].clone())
        }
    }

    /// Returns all `UploadStatus::InProgress` upload records.
    pub fn get_in_progress_uploads(&self) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE status = 'in_progress'
             ORDER by created_at",
        )?;
        let records = stmt
            .query_and_then_named(&[], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Returns all `UploadStatus::Queued` upload records.
    pub fn get_queued_uploads(&self) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE status = 'queued'
             ORDER by created_at",
        )?;
        let records = stmt
            .query_and_then_named(&[], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Returns all `UploadStatus::Queued` and `UploadStatus::InProgress`
    /// upload records.
    pub fn get_active_uploads(&self) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE status IN ('in_progress', 'queued')
             ORDER by status, created_at",
        )?;
        let records = stmt
            .query_and_then_named(&[], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Returns all `UploadStatus::Failed` upload records.
    pub fn get_failed_uploads(&self) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE status = 'failed'
             ORDER by created_at",
        )?;
        let records = stmt
            .query_and_then_named(&[], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Returns NUM most recently completed uploads.
    pub fn get_completed_uploads(&self, num: usize) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE status = 'completed'
             ORDER BY updated_at DESC
             LIMIT :num",
        )?;
        let records = stmt
            .query_and_then_named(&[(":num", &num.to_string())], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Resumes the specified upload. Note: Only failed uploads that have a progress > 0 can be retried.
    pub fn resume_failed_upload(&self, id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare(
                "UPDATE upload_record SET status = 'queued' WHERE ID = :id AND status = 'failed' AND progress > 0",
            )?;
        stmt.execute_named(&[(":id", &id)])
            .map(|count| count >= 1)
            .map_err(Into::into)
    }

    /// Cancels the specified upload. Note: only queued or in-progress
    /// uploads can be cancelled.
    pub fn cancel_upload(&self, id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "DELETE FROM upload_record WHERE ID = :id AND status IN ('queued', 'in_progress')",
        )?;
        stmt.execute_named(&[(":id", &id)])
            .map(|count| count >= 1)
            .map_err(Into::into)
    }

    /// Cancels all queued uploads, leaving in-progress uploads to finish.
    pub fn cancel_queued_uploads(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("DELETE FROM upload_record WHERE status = 'queued'")?;
        stmt.execute_named(&[]).map_err(Into::into)
    }

    /// Cancels all uploads, regardless of status.
    pub fn cancel_all_uploads(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("DELETE FROM upload_record WHERE status IN ('queued', 'in_progress')")?;
        stmt.execute_named(&[]).map_err(Into::into)
    }

    /// Gets all active uploads that began since a given date.
    pub fn get_active_uploads_started_since(&self, since: time::Timespec) -> Result<UploadRecords> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id,
                    file_path,
                    dataset_id,
                    package_id,
                    import_id,
                    progress,
                    status,
                    created_at,
                    updated_at,
                    append,
                    upload_service,
                    organization_id,
                    chunk_size,
                    multipart_upload_id
             FROM upload_record
             WHERE status IN ('in_progress', 'queued')
                    OR created_at >= :since
             ORDER by status, created_at",
        )?;
        let records = stmt
            .query_and_then_named(&[(":since", &since)], UploadRecord::from_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(UploadRecords { records })
    }

    /// Get the last time the agent checked for an update
    pub fn get_last_version_check(&self) -> Result<Option<time::Timespec>> {
        let conn = self.pool.get()?;

        let mut stmt =
            conn.prepare("SELECT checked_at FROM agent_updates ORDER BY checked_at DESC LIMIT 1")?;

        stmt.query_row(NO_PARAMS, |row| row.get(0))
            .optional()
            .map_err(Into::into)
    }

    /// Record that the agent just checked for updates
    pub fn add_version_check(&self) -> Result<time::Timespec> {
        let conn = self.pool.get()?;

        let now = time::now().to_timespec();

        let mut stmt =
            conn.prepare("INSERT INTO agent_updates (checked_at) VALUES (:checked_at)")?;

        stmt.execute_named(&[(":checked_at", &now)])
            .map_err(Into::into)
            .and_then(|_| Ok(now))
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::ps::util;

    #[test]
    fn creating_users_with_settings_succeeds() {
        let mut user = UserRecord::new(
            "N:user:foo".to_string(),               // id
            "Joe Schmoe".to_string(),               // name
            "token".to_string(),                    // token
            "default".to_string(),                  // profile
            ApiEnvironment::NonProduction,          // environment
            "N:organization:pennsieve".to_string(), // org id
            "Pennsieve".to_string(),                // org name,
            "encryption_key".to_string(),           // encryption_key
        );
        let db = util::database::temp().unwrap();
        db.upsert_user(&mut user).unwrap();

        let found_user = db.get_user().unwrap().unwrap();
        assert_eq!(found_user.id, user.id);

        // Update the dataset:
        let settings1 = UserSettings::new(Some("foo"));
        db.upsert_user_settings(&found_user.id, &found_user.profile, &settings1)
            .unwrap();
        assert_eq!(
            db.get_user_settings(&found_user.id, &found_user.profile)
                .unwrap(),
            Some(settings1.clone())
        );

        // Update the dataset again for a different profile:
        let settings2 = UserSettings::new(Some("bar"));
        db.upsert_user_settings(&found_user.id, "other", &settings2)
            .unwrap();
        assert_eq!(
            db.get_user_settings(&found_user.id, "other").unwrap(),
            Some(settings2)
        );

        // The old dataset should persist for the original profile:
        assert_eq!(
            db.get_user_settings(&found_user.id, &found_user.profile)
                .unwrap(),
            Some(settings1)
        );
    }

    #[test]
    fn is_cached_row_exists_complete_false() {
        let db = util::database::temp().unwrap();
        let key = String::from("c1.100.1");
        let record = PageRecord::new(key.clone(), false, false, 0);
        db.upsert_page(&record).unwrap();
        assert!(!db.is_page_cached(&key).unwrap());
    }

    #[test]
    fn is_record_nan_is_true() {
        let db = util::database::temp().unwrap();
        let key = String::from("c1.10.1");
        let record = PageRecord::new(key.clone(), true, false, 0);
        db.upsert_page(&record).unwrap();
        assert!(db.is_page_nan(&key).unwrap());
    }

    #[test]
    fn is_cached_row_exists_complete_true() {
        let db = util::database::temp().unwrap();
        let key = String::from("c1.100.2");
        db.write_nan_filled(&key, true).unwrap();
        assert!(db.is_page_cached(&key).unwrap());
    }

    #[test]
    fn is_row_cached_row_doesnt_exist() {
        let db = util::database::temp().unwrap();
        let key = String::from("c1.100.3");
        assert!(!db.is_page_cached(&key).unwrap());
    }

    #[test]
    fn test_touch_last_used() {
        let db = util::database::temp().unwrap();
        let key = String::from("c1.100.10");
        let starting_time = time::now().to_timespec();
        let record = PageRecord {
            id: key.clone(),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: starting_time,
        };

        // Make sure the timestamps from the time of createtion and upsert/touch
        // actually differ:
        thread::sleep(Duration::from_millis(1));

        db.upsert_page(&record).unwrap();
        db.touch_last_used(&key).unwrap();
        assert_ne!(db.get_page(&key).unwrap().last_used, starting_time);
    }

    #[test]
    fn get_total_size_default() {
        let db = util::database::temp().unwrap();
        assert_eq!(db.get_total_size().unwrap(), 0);
    }

    #[test]
    fn get_total_size_positive() {
        let db = util::database::temp().unwrap();
        let record = PageRecord::new(String::from("c1.100.1"), false, false, 0);
        db.upsert_page(&record).unwrap();
        let record = PageRecord::new(String::from("c1.100.2"), false, false, 100);
        db.upsert_page(&record).unwrap();
        let record = PageRecord::new(String::from("c1.100.3"), false, false, 501);
        db.upsert_page(&record).unwrap();
        assert_eq!(db.get_total_size().unwrap(), 601);
    }

    #[test]
    fn delete_record() {
        let db = util::database::temp().unwrap();
        let key = String::from("c1.100.10");
        let record = PageRecord::new(key.clone(), false, false, 10);
        db.upsert_page(&record).unwrap();
        db.delete_page(&record).unwrap();
        assert!(db.get_page(&key).is_err());
    }

    #[test]
    fn test_soft_aged_records() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let record1 = PageRecord {
            id: String::from("c1.100.1"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::weeks(15),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("c1.100.2"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::weeks(20),
        };
        db.upsert_page(&record2).unwrap();
        let record3 = PageRecord {
            id: String::from("c1.100.3"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::weeks(10),
        };
        db.upsert_page(&record3).unwrap();
        let record4 = PageRecord {
            id: String::from("c1.100.4"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::days(3),
        };
        db.upsert_page(&record4).unwrap();
        assert_eq!(
            db.get_soft_aged_pages()
                .unwrap()
                .collect::<Vec<PageRecord>>(),
            vec![record2, record1, record3]
        );
    }

    #[test]
    fn test_hard_aged_pages() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let record1 = PageRecord {
            id: String::from("c1.100.1"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::days(15),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("c1.100.2"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::days(20),
        };
        db.upsert_page(&record2).unwrap();
        let record3 = PageRecord {
            id: String::from("c1.100.3"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::days(10),
        };
        db.upsert_page(&record3).unwrap();
        let record4 = PageRecord {
            id: String::from("c1.100.4"),
            nan_filled: false,
            complete: false,
            size: 0,
            last_used: now - time::Duration::hours(3),
        };
        db.upsert_page(&record4).unwrap();
        assert_eq!(
            db.get_hard_aged_pages()
                .unwrap()
                .collect::<Vec<PageRecord>>(),
            vec![record2, record1, record3]
        );
    }

    #[test]
    fn test_get_user() {
        let db = util::database::temp().unwrap();
        let mut record = UserRecord::new(
            String::from("user_1"),
            String::from("name_1"),
            String::from("session_token_1"),
            String::from("dev"),
            ApiEnvironment::NonProduction,
            String::from("org_id_1"),
            String::from("org_1"),
            String::from("org_1"),
        );
        db.upsert_user(&mut record).unwrap();
        assert_eq!(db.get_user().unwrap().unwrap(), record);
    }

    #[test]
    fn test_get_user_none() {
        let db = util::database::temp().unwrap();
        assert_eq!(db.get_user().unwrap(), None);
    }

    #[test]
    fn test_delete_user() {
        let db = util::database::temp().unwrap();
        let mut record = UserRecord::new(
            String::from("user_1"),
            String::from("name_1"),
            String::from("session_token_1"),
            String::from("dev"),
            ApiEnvironment::NonProduction,
            String::from("org_id_1"),
            String::from("org_1"),
            String::from("org_1"),
        );
        db.upsert_user(&mut record).unwrap();

        db.delete_user().unwrap();
        assert_eq!(db.get_user().unwrap(), None);
    }

    #[test]
    fn test_limit_of_one_user() {
        let db = util::database::temp().unwrap();
        let mut record1 = UserRecord::new(
            String::from("user_1"),
            String::from("name_1"),
            String::from("session_token_1"),
            String::from("dev"),
            ApiEnvironment::NonProduction,
            String::from("org_id_1"),
            String::from("org_1"),
            String::from("org_1"),
        );
        db.upsert_user(&mut record1).unwrap();
        let mut record2 = UserRecord::new(
            String::from("user_2"),
            String::from("name_2"),
            String::from("session_token_2"),
            String::from("dev"),
            ApiEnvironment::NonProduction,
            String::from("org_id_2"),
            String::from("org_2"),
            String::from("org_2"),
        );
        db.upsert_user(&mut record2).unwrap();
        let mut record3 = UserRecord::new(
            String::from("user_3"),
            String::from("name_3"),
            String::from("session_token_3"),
            String::from("dev"),
            ApiEnvironment::NonProduction,
            String::from("org_id_3"),
            String::from("org_3"),
            String::from("org_3"),
        );
        db.upsert_user(&mut record3).unwrap();
        assert_eq!(db.get_user().unwrap().unwrap(), record3);
    }

    #[test]
    fn test_user_token_expiration() {
        let mut record = UserRecord::new(
            String::from("user_1"),
            String::from("name_1"),
            String::from("session_token_1"),
            String::from("dev"),
            ApiEnvironment::NonProduction,
            String::from("org_id_1"),
            String::from("org_1"),
            String::from("org_1"),
        );
        assert!(record.is_token_valid());
        record.updated_at = time::now().to_timespec() - time::Duration::hours(3);
        assert!(!record.is_token_valid());
    }

    #[test]
    fn test_get_queued_uploads() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(1),
            updated_at: now - time::Duration::weeks(1),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record).unwrap();
        let mut record2 = UploadRecord {
            id: Some(2),
            file_path: String::from("file/path/2"),
            dataset_id: String::from("ds_2"),
            import_id: String::from("import_2"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record2).unwrap();
        let mut record3 = UploadRecord::new(
            String::from("file/path/3"),
            String::from("ds_3"),
            Some(String::from("package_3")),
            String::from("import_3"),
            String::from("organization_1"),
            false,
            Some(100),
            Some(String::from("multipart_upload_id")),
        )
        .unwrap();
        record3.status = UploadStatus::Completed;
        db.insert_upload(&mut record3).unwrap();
        let coll = db.get_queued_uploads().unwrap();
        assert_eq!(coll.iter().collect::<Vec<_>>(), vec![&record2, &record]);
    }

    #[test]
    fn test_cancel_queued_uploads() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(1),
            updated_at: now - time::Duration::weeks(1),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record).unwrap();
        let mut record2 = UploadRecord {
            id: Some(2),
            file_path: String::from("file/path/2"),
            dataset_id: String::from("ds_2"),
            import_id: String::from("import_2"),
            package_id: None,
            progress: 0,
            status: UploadStatus::InProgress,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record2).unwrap();
        let mut record3 = UploadRecord {
            id: Some(3),
            file_path: String::from("file/path/3"),
            dataset_id: String::from("ds_3"),
            import_id: String::from("import_3"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record3).unwrap();
        let records = db.get_queued_uploads().unwrap();
        assert_eq!(records.len(), 2);
        let cancelled_count = db.cancel_queued_uploads().unwrap();
        assert_eq!(cancelled_count, 2);
        let records = db.get_active_uploads().unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_get_in_progress_uploads() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(1),
            updated_at: now - time::Duration::weeks(1),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record).unwrap();
        let mut record2 = UploadRecord {
            id: Some(2),
            file_path: String::from("file/path/2"),
            dataset_id: String::from("ds_2"),
            import_id: String::from("import_2"),
            package_id: None,
            progress: 0,
            status: UploadStatus::InProgress,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record2).unwrap();
        let mut record3 = UploadRecord::new(
            String::from("file/path/3"),
            String::from("ds_3"),
            Some(String::from("package_3")),
            String::from("import_3"),
            String::from("organization_1"),
            false,
            Some(100),
            Some(String::from("multipart_upload_id")),
        )
        .unwrap();
        record3.status = UploadStatus::Completed;
        db.insert_upload(&mut record3).unwrap();
        let mut record4 = UploadRecord {
            id: Some(4),
            file_path: String::from("file/path/4"),
            dataset_id: String::from("ds_4"),
            import_id: String::from("import_4"),
            package_id: None,
            progress: 0,
            status: UploadStatus::InProgress,
            created_at: now - time::Duration::weeks(4),
            updated_at: now - time::Duration::weeks(4),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record4).unwrap();
        let coll = db.get_in_progress_uploads().unwrap();
        assert_eq!(coll.iter().collect::<Vec<_>>(), vec![&record4, &record2]);
    }

    #[test]
    fn test_get_active_uploads() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(1),
            updated_at: now - time::Duration::weeks(1),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record).unwrap();
        let mut record2 = UploadRecord {
            id: Some(2),
            file_path: String::from("file/path/2"),
            dataset_id: String::from("ds_2"),
            import_id: String::from("import_2"),
            package_id: None,
            progress: 0,
            status: UploadStatus::InProgress,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record2).unwrap();
        let mut record3 = UploadRecord::new(
            String::from("file/path/3"),
            String::from("ds_3"),
            Some(String::from("package_3")),
            String::from("import_3"),
            String::from("organization_1"),
            false,
            Some(100),
            Some(String::from("multipart_upload_id")),
        )
        .unwrap();
        record3.status = UploadStatus::Completed;
        db.insert_upload(&mut record3).unwrap();
        let mut record4 = UploadRecord {
            id: Some(4),
            file_path: String::from("file/path/4"),
            dataset_id: String::from("ds_4"),
            import_id: String::from("import_4"),
            package_id: None,
            progress: 0,
            status: UploadStatus::InProgress,
            created_at: now - time::Duration::weeks(4),
            updated_at: now - time::Duration::weeks(4),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record4).unwrap();
        let coll = db.get_active_uploads().unwrap();
        assert_eq!(
            coll.iter().collect::<Vec<_>>(),
            vec![&record4, &record2, &record]
        );
    }

    #[test]
    fn test_get_completed_uploads() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record1 = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Completed,
            created_at: now - time::Duration::weeks(1),
            updated_at: now - time::Duration::weeks(1),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record1).unwrap();
        let mut record2 = UploadRecord {
            id: Some(2),
            file_path: String::from("file/path/2"),
            dataset_id: String::from("ds_2"),
            import_id: String::from("import_2"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Failed,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_2"),
            chunk_size: Some(200),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record2).unwrap();
        let mut record3 = UploadRecord {
            id: Some(3),
            file_path: String::from("file/path/3"),
            dataset_id: String::from("ds_3"),
            import_id: String::from("import_3"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Completed,
            created_at: now - time::Duration::weeks(3),
            updated_at: now - time::Duration::weeks(3),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_3"),
            chunk_size: Some(300),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record3).unwrap();
        let mut record4 = UploadRecord {
            id: Some(4),
            file_path: String::from("file/path/4"),
            dataset_id: String::from("ds_4"),
            import_id: String::from("import_4"),
            package_id: None,
            progress: 0,
            status: UploadStatus::InProgress,
            created_at: now - time::Duration::weeks(4),
            updated_at: now - time::Duration::weeks(4),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_4"),
            chunk_size: Some(400),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record4).unwrap();
        let coll = db.get_completed_uploads(10).unwrap();
        assert_eq!(coll.iter().collect::<Vec<_>>(), vec![&record1, &record3]);

        let limited_coll = db.get_completed_uploads(1).unwrap();
        assert_eq!(limited_coll.iter().collect::<Vec<_>>(), vec![&record1]);
    }

    #[test]
    fn test_update_upload_status() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(1),
            updated_at: now - time::Duration::weeks(1),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record).unwrap();
        let mut record2 = UploadRecord {
            id: Some(2),
            file_path: String::from("file/path/2"),
            dataset_id: String::from("ds_2"),
            import_id: String::from("import_1"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(2),
            updated_at: now - time::Duration::weeks(2),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record2).unwrap();
        let mut record3 = UploadRecord::new(
            String::from("file/path/3"),
            String::from("ds_3"),
            Some(String::from("package_3")),
            String::from("import_3"),
            String::from("organization_1"),
            false,
            Some(100),
            Some(String::from("multipart_upload_id")),
        )
        .unwrap();
        record3.status = UploadStatus::Queued;
        db.insert_upload(&mut record3).unwrap();
        let mut record4 = UploadRecord {
            id: Some(4),
            file_path: String::from("file/path/4"),
            dataset_id: String::from("ds_4"),
            import_id: String::from("import_4"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(4),
            updated_at: now - time::Duration::weeks(4),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record4).unwrap();
        assert_eq!(
            db.update_import_status(&record.import_id, UploadStatus::InProgress)
                .unwrap(),
            2
        );
        assert_eq!(db.get_queued_uploads().unwrap().records.len(), 2);
        assert_eq!(db.get_in_progress_uploads().unwrap().records.len(), 2);
    }

    #[test]
    fn test_upload_should_retry() {
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            progress: 0,
            package_id: None,
            status: UploadStatus::Queued,
            created_at: now,
            updated_at: now,
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        assert!(!record.should_retry());
        record.updated_at = now - time::Duration::minutes(30);
        assert!(!record.should_retry());
        record.updated_at = now - time::Duration::minutes(90);
        assert!(record.should_retry());
    }

    #[test]
    fn test_upload_should_fail() {
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/1"),
            dataset_id: String::from("ds_1"),
            import_id: String::from("import_1"),
            progress: 0,
            package_id: None,
            status: UploadStatus::Queued,
            created_at: now,
            updated_at: now,
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        assert!(!record.should_fail());
        record.created_at = now - time::Duration::hours(5);
        assert!(!record.should_fail());
        record.created_at = now - time::Duration::hours(10);
        assert!(record.should_fail());
    }

    #[test]
    fn test_get_uploads_by_import_id() {
        let db = util::database::temp().unwrap();
        let mut record = UploadRecord::new(
            String::from("file/path/1"),
            String::from("ds_1"),
            Some(String::from("package_1")),
            String::from("organization_1"),
            String::from("import_1"),
            false,
            Some(100),
            Some(String::from("multipart_upload_id")),
        )
        .unwrap();
        db.insert_upload(&mut record).unwrap();
        record.import_id = String::from("import_2");
        db.insert_upload(&mut record).unwrap();
        db.insert_upload(&mut record).unwrap();
        {
            let coll = db
                .get_uploads_by_import_id(&String::from("import_1"))
                .unwrap();
            assert_eq!(coll.iter().collect::<Vec<_>>().len(), 1);
        }
        {
            let coll = db
                .get_uploads_by_import_id(&String::from("import_2"))
                .unwrap();
            assert_eq!(coll.iter().collect::<Vec<_>>().len(), 2);
        }
    }

    #[test]
    fn test_get_upload_by_upload_id() {
        let db = util::database::temp().unwrap();
        let now = time::now().to_timespec();
        let mut record = UploadRecord {
            id: Some(1),
            file_path: String::from("file/path/4"),
            dataset_id: String::from("ds_4"),
            import_id: String::from("import_4"),
            package_id: None,
            progress: 0,
            status: UploadStatus::Queued,
            created_at: now - time::Duration::weeks(4),
            updated_at: now - time::Duration::weeks(4),
            append: false,
            upload_service: false,
            organization_id: String::from("organization_1"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        };
        db.insert_upload(&mut record).unwrap();

        let db_record = db.get_upload_by_upload_id(1).unwrap();
        assert_eq!(record, db_record);

        let failed_lookup = db.get_upload_by_upload_id(2);
        assert!(failed_lookup.is_err());
    }

    #[test]
    fn test_get_active_uploads_started_since() {
        let watch_started_at = time::now().to_timespec();

        let db = util::database::temp().unwrap();
        let record = UploadRecord::new(
            String::from("file/path/1"),
            String::from("ds_1"),
            Some(String::from("package_1")),
            String::from("organization_1"),
            String::from("import_1"),
            false,
            Some(100),
            Some(String::from("multipart_upload_id")),
        )
        .unwrap();

        let thirty_weeks_ago = (time::now() - time::Duration::weeks(30)).to_timespec();

        // a recent active upload
        let mut contemporary_record = record.clone();

        // an old completed upload
        let mut old_completed_record = record.clone();
        old_completed_record.status = UploadStatus::Completed;
        old_completed_record.created_at = thirty_weeks_ago;

        // a new completed upload
        let mut new_completed_record = record.clone();
        old_completed_record.status = UploadStatus::Completed;

        // an old active upload
        let mut old_active_record = record.clone();
        old_active_record.created_at = thirty_weeks_ago;

        // insert three old completed uploads
        db.insert_upload(&mut old_completed_record).unwrap();
        db.insert_upload(&mut old_completed_record).unwrap();
        db.insert_upload(&mut old_completed_record).unwrap();

        // insert three active uploads started now
        db.insert_upload(&mut contemporary_record).unwrap();
        db.insert_upload(&mut contemporary_record).unwrap();
        db.insert_upload(&mut contemporary_record).unwrap();

        // insert three active uploads started way before now
        db.insert_upload(&mut old_active_record).unwrap();
        db.insert_upload(&mut old_active_record).unwrap();
        db.insert_upload(&mut old_active_record).unwrap();

        // 9 were inserted, 6 should get returned because 3 of the 9 are completed;
        let coll = db
            .get_active_uploads_started_since(watch_started_at)
            .unwrap();
        assert_eq!(coll.iter().collect::<Vec<_>>().len(), 6);

        // insert 3 new ones
        db.insert_upload(&mut contemporary_record).unwrap();
        db.insert_upload(&mut contemporary_record).unwrap();
        db.insert_upload(&mut contemporary_record).unwrap();

        // the new ones should be included
        let coll = db
            .get_active_uploads_started_since(watch_started_at)
            .unwrap();
        assert_eq!(coll.iter().collect::<Vec<_>>().len(), 9);

        // insert 3 new completed ones
        db.insert_upload(&mut new_completed_record).unwrap();
        db.insert_upload(&mut new_completed_record).unwrap();
        db.insert_upload(&mut new_completed_record).unwrap();

        // the new ones should be included
        let coll = db
            .get_active_uploads_started_since(watch_started_at)
            .unwrap();
        assert_eq!(coll.iter().collect::<Vec<_>>().len(), 12);
    }

    #[test]
    fn test_version_checks() {
        let db = util::database::temp().unwrap();
        assert_eq!(db.get_last_version_check().unwrap(), None);

        let first = db.add_version_check().unwrap();
        assert_eq!(db.get_last_version_check().unwrap(), Some(first));

        thread::sleep(Duration::from_millis(1));

        let second = db.add_version_check().unwrap();
        assert_eq!(db.get_last_version_check().unwrap(), Some(second));
    }
}
