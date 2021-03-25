use std::cmp::max;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::result;

use futures::Future as _Future;
use futures::*;
use sha2::{Digest, Sha256};

use crate::ps;
pub use crate::ps::agent::api::{
    self, Api, DatasetNodeId, OrganizationId, PackageId, Renamed, UserId, Validator,
};
pub use crate::ps::agent::cli::error::{Error, ErrorKind, Result};
use crate::ps::agent::config::api::Settings as ApiSettings;
use crate::ps::agent::config::{self, Config};
use crate::ps::agent::database::{Database, Error as DBError, UserRecord, UserSettings};
use crate::ps::agent::{self, Future, OutputFormat};
use crate::ps::util::futures::*;

pub mod error;
pub mod input;
mod output;
mod types;
pub mod upload;
mod validate;

pub use self::types::{cli_table as table, CliTable};
pub use self::upload::{StartMode, StopMode, UploadWatcher};

/// A `Cli` is a wrapper around an `Api` and `Database` that
/// often calls api methods and maps the resulting `future`
/// and prints a CLI representation of the response.
pub struct Cli {
    api: Api,
    db: Database,
    output: OutputFormat,
    settings: ApiSettings,
}

impl Cli {
    /// Creates a new `Cli`.
    pub fn new(db: &Database, api: &Api, output: OutputFormat, settings: &ApiSettings) -> Self {
        Self {
            api: api.clone(),
            db: db.clone(),
            output,
            settings: settings.clone(),
        }
    }

    /// Returns the current output format.
    pub fn output(&self) -> &OutputFormat {
        &self.output
    }

    /// Given a `cli:Error`, lift the error into a `cli::Future`.
    pub fn error(err: Error) -> Future<()> {
        future::err(err.into()).into_trait()
    }

    /// Prints a `config.ini` template to stdout.
    pub fn print_config_example() -> Future<()> {
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/config.ini.sample"
        ));
        future::lazy(move || {
            println!("{}", template);
            Ok(())
        })
        .into_trait()
    }

    /// Prints the user's `config.ini` to stdout.
    ///
    /// If the config.ini cannot be found, the config wizard will be
    /// started.
    pub fn print_or_create_config(db: Database) -> Future<()> {
        ps::config_file()
            .and_then(|config_file| {
                File::open(config_file)
                    .and_then(|mut file| {
                        let mut contents = String::new();
                        file.read_to_string(&mut contents)?;
                        println!("{}", contents);
                        Ok(())
                    })
                    .map_err(Into::into)
            })
            .into_future()
            .or_else(move |_| {
                Self::start_config_wizard(db)
                    .map(|_| ())
                    .map_err(Into::into)
            })
            .into_trait()
    }

    /// Start the configuration wizard
    /// This is static because this command needs to be able to be run
    /// before `config.ini` is initialized.
    pub fn start_config_wizard(db: Database) -> Future<()> {
        config::start_config_wizard()
            .map_err(Into::into)
            .into_future()
            .and_then(move |config| {
                let profile = config.api_settings.default_profile();
                let api = api::Api::new(&db, &config, profile.environment);
                api.login(profile).map(|_| ()).into_trait()
            })
            .into_trait()
    }

    /// Prints `config.ini` settings as "<key>:\t<value>" pairs
    pub fn print_settings_key_values(&self) -> Future<()> {
        let global_settings = self.settings.global_settings.clone().take();
        let mut settings = self.settings.agent_settings.clone().take();
        settings.extend(
            global_settings
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );

        future::lazy(move || {
            let settings = Into::<output::CliSettings>::into(settings);
            println!("{}", settings);
            Ok(())
        })
        .into_trait()
    }

    /// Prints the value of the `config.ini` setting key, if it exists.
    pub fn print_settings_value<S>(&self, key: S) -> Future<()>
    where
        S: Into<String>,
    {
        let global_settings = self.settings.global_settings.clone().take();
        let mut settings = self.settings.agent_settings.clone().take();
        settings.extend(
            global_settings
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );

        let key = key.into();
        match settings.get(&key) {
            Some(value) => {
                println!("{}", value.to_string());
                Ok(())
            }
            None => Err(config::Error::config_value_not_found(key)),
        }
        .map_err(Into::into)
        .into_future()
        .into_trait()
    }

    /// Print the current `agent.db` schema version
    pub fn print_schema_version(&self) -> Future<()> {
        self.db
            .get_schema_version()
            .map_err(Into::into)
            .into_future()
            .and_then(|version| {
                println!("{}", version);
                Ok(())
            })
            .into_trait()
    }

    /// Sets the `agent.db` schema version to the version provided,
    /// printing the set version on success.
    pub fn set_schema_version(&self, new_version: usize) -> Future<()> {
        let new_version = max(0, new_version);
        self.db
            .set_schema_version(new_version)
            .map_err(Into::into)
            .into_future()
            .and_then(move |_| {
                println!("{}", new_version);
                Ok(())
            })
            .into_trait()
    }

    /// Print account details of the currently logged in user.
    pub fn print_whoami(&self) -> Future<()> {
        self.api
            .get_user_and_refresh()
            .and_then(|response| {
                println!("{}", response);
                Ok(())
            })
            .into_trait()
    }

    /// Queues files for upload to the Pennsieve platform, printing status
    /// upon success.
    #[allow(clippy::too_many_arguments)]
    pub fn queue_uploads<F, D, P>(
        &self,
        files: Vec<F>,
        dataset_id_or_name: Option<D>,
        package_id_or_name: Option<P>,
        append: bool,
        force: bool,
        recursive: bool,
    ) -> Future<()>
    where
        F: Into<String>,
        D: Into<String>,
        P: Into<String>,
    {
        self.api
            .queue_uploads(
                files,
                dataset_id_or_name,
                package_id_or_name,
                append,
                force,
                recursive,
                validate::Dataset::new(force),
                validate::Folder::new(force),
            )
            .and_then(|queued| {
                let n = queued.len();
                println!(
                    "\nQueued {n} {thing}\n",
                    n = n,
                    thing = if n == 1 { "file" } else { "files" }
                );
                Ok(())
            })
            .into_trait()
    }

    /// Requeues the specified file uploads.
    pub fn requeue_failed_uploads(&self, upload_ids: Vec<String>) -> Future<()> {
        let db = self.db.clone();
        future::lazy(move || {
            upload_ids
                .into_iter()
                .map(|id| {
                    db.resume_failed_upload(&id).map(|success| {
                        if !success {
                            eprintln!(
                                "Could not retry upload with id {}. \
                                 Only failed uploads that were interrupted midway can be retried.",
                                id
                            )
                        }
                    })
                })
                .collect::<result::Result<Vec<_>, _>>()
                .map_err(Into::into)
                .map(|_| ())
        })
        .into_trait()
    }

    /// Cancels the specified file uploads.
    pub fn cancel_uploads(&self, upload_ids: Vec<String>) -> Future<()> {
        let db = self.db.clone();
        future::lazy(move || {
            let ids = upload_ids
                .into_iter()
                .map(|id| db.cancel_upload(&id).map(|success| (id, success)))
                .collect::<result::Result<Vec<_>, _>>()?;
            ids.into_iter().for_each(|(id, success)| {
                if success {
                    println!("Cancelled upload {}", id);
                }
            });
            Ok(())
        })
        .into_trait()
    }

    /// Cancels the specified file uploads.
    pub fn cancel_pending_uploads(&self) -> Future<()> {
        let db = self.db.clone();
        db.cancel_queued_uploads()
            .map_err(Into::into)
            .and_then(|count| {
                println!(
                    "Cancelled {count} {action}",
                    count = count,
                    action = if count == 1 { "upload" } else { "uploads" }
                );
                Ok(())
            })
            .into_future()
            .into_trait()
    }

    /// Cancels all file uploads, regardless of status.
    pub fn cancel_all_uploads(&self) -> Future<()> {
        let db = self.db.clone();
        db.cancel_all_uploads()
            .map_err(Into::into)
            .and_then(|count| {
                println!(
                    "Cancelled {count} {action}",
                    count = count,
                    action = if count == 1 { "upload" } else { "uploads" }
                );
                Ok(())
            })
            .into_future()
            .into_trait()
    }

    /// Prints the details of active uploads (queued and in-progress).
    pub fn active_uploads(&self) -> Future<()> {
        let db = self.db.clone();
        future::lazy(move || {
            let uploads = db.get_active_uploads()?;
            if uploads.is_package_completed() {
                println!("No uploads");
            } else {
                println!("{}\n", Into::<output::CliUploadRecords>::into(uploads));
            }
            Ok(())
        })
        .into_trait()
    }

    /// Prints the details of the NUM most recent uploads.
    pub fn most_recently_completed_uploads(&self, num: usize) -> Future<()> {
        let db = self.db.clone();
        future::lazy(move || {
            let uploads = db.get_completed_uploads(num)?;
            if uploads.is_empty() {
                println!("No completed uploads");
            } else {
                println!("{}\n", Into::<output::CliUploadRecords>::into(uploads));
            }
            Ok(())
        })
        .into_trait()
    }

    /// Prints the details of failed uploads
    pub fn failed_uploads(&self) -> Future<()> {
        let db = self.db.clone();
        future::lazy(move || {
            let uploads = db.get_failed_uploads()?;
            if uploads.is_empty() {
                println!("No uploads");
            } else {
                println!("{}\n", Into::<output::CliUploadRecords>::into(uploads));
            }
            Ok(())
        })
        .into_trait()
    }

    fn compute_multichunk_hash(mut file: File, chunk_size: u64) -> Result<String> {
        let mut chunk_hashes: Vec<String> = vec![];
        let mut total_bytes_read: u64 = 0;
        let mut buffer = vec![0; chunk_size as usize];

        // Multi-chunk case:

        loop {
            let mut hasher = Sha256::new();

            file.seek(SeekFrom::Start(total_bytes_read))?;
            let bytes_read = file.read(&mut buffer)?;
            total_bytes_read += bytes_read as u64;

            if bytes_read > 0 {
                hasher.update(&buffer[..bytes_read]);
                chunk_hashes.push(format!("{:x}", hasher.finalize()));
            } else {
                break;
            }
        }

        Ok(format!(
            "{:x}",
            chunk_hashes
                .into_iter()
                .fold(Sha256::new(), |mut acc, hash| {
                    acc.update(hash);
                    acc
                })
                .finalize()
        ))
    }

    fn compute_simple_hash(mut file: File, file_size: u64) -> Result<String> {
        let mut buffer = vec![0; file_size as usize];
        let mut hasher = Sha256::new();

        file.seek(SeekFrom::Start(0))?;
        let bytes_read: usize = file.read(&mut buffer)?;

        hasher.update(&buffer[..bytes_read]);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Verify the specified file upload.
    pub fn verify_upload(&self, upload_id: usize, file_path: Option<PathBuf>) -> Future<()> {
        let db = self.db.clone();
        self.api
            .get_upload_file_hash(upload_id)
            .and_then(move |hash| {
                // if get_upload_file_hash succeeded, then this upload
                // must exist in the database
                let upload = db.get_upload_by_upload_id(upload_id).unwrap();

                let verify_against = if let Some(file_path) = file_path {
                    file_path
                } else {
                    PathBuf::from(upload.file_path.clone())
                };

                File::open(verify_against.clone())
                    .map_err(Into::into)
                    .and_then(|file| {
                        upload
                            .chunk_size
                            .ok_or_else(|| {
                                Into::<agent::Error>::into(DBError::upload_without_chunk_size(
                                    upload_id,
                                ))
                            })
                            .map(|chunk_size| (file, chunk_size))
                    })
                    .and_then(|(file, chunk_size)| {
                        let file_size: u64 = file.metadata()?.len();
                        let computed_hash: String = if file_size > chunk_size {
                            Cli::compute_multichunk_hash(file, chunk_size)?
                        } else {
                            Cli::compute_simple_hash(file, chunk_size)?
                        };

                        if computed_hash != hash.hash {
                            Err(Error::upload_does_not_match(verify_against).into())
                        } else {
                            Ok(())
                        }
                    })
                    .into_future()
                    .into_trait()
            })
            .into_trait()
    }

    /// Prints all organizations the current user is a member of.
    pub fn print_organizations(&self) -> Future<()> {
        self.api
            .get_organizations()
            .and_then(|response| {
                println!("{}", Into::<output::CliOrganizations>::into(response));
                Ok(())
            })
            .into_trait()
    }

    /// Print all members that are part of the current organization.
    pub fn print_members(&self) -> Future<()> {
        self.api
            .get_members()
            .and_then(|members| {
                println!(
                    "{}",
                    Into::<output::CliUsers>::into(members).table_without_roles()
                );
                Ok(())
            })
            .into_trait()
    }

    /// Print all teams that are part of the current organization.
    pub fn print_teams(&self) -> Future<()> {
        self.api
            .get_teams()
            .and_then(|response| Ok(response.into_iter().map(|t| t.take()).collect::<Vec<_>>()))
            .and_then(|teams| {
                println!("{}", Into::<output::CliTeams>::into(teams));
                Ok(())
            })
            .into_trait()
    }

    /// Prints all datasets the current user has access to.
    pub fn print_datasets(&self) -> Future<()> {
        self.api
            .get_datasets()
            .map(|response| -> Vec<output::CliDataset> {
                response
                    .into_iter()
                    .map(Into::<output::CliDataset>::into)
                    .collect()
            })
            .and_then(|response| {
                println!("{}", Into::<output::CliDatasets>::into(response));
                Ok(())
            })
            .into_trait()
    }

    /// Create a new dataset.
    pub fn create_dataset<P, Q>(&self, name: P, description: Option<Q>) -> Future<()>
    where
        P: Into<String>,
        Q: Into<String>,
    {
        let name = name.into();
        let description = description.map(Into::into);
        self.api
            .create_dataset(name.clone(), description)
            .and_then(move |dataset| {
                println!(
                    "Created dataset {name} ({id})",
                    name = name,
                    id = dataset.take().id()
                );
                Ok(())
            })
            .into_trait()
    }

    /// Delete a dataset by its ID.
    pub fn delete_dataset<P>(&self, id: P) -> Future<()>
    where
        P: Into<String>,
    {
        let id = id.into();
        self.api
            .delete_dataset(id.clone())
            .and_then(move |_| {
                println!("Deleted {id}", id = id);
                Ok(())
            })
            .into_trait()
    }

    /// Prints a specific dataset.
    pub fn print_dataset<P>(&self, id_or_name: P) -> Future<()>
    where
        P: Into<String>,
    {
        self.api
            .get_dataset(id_or_name)
            .and_then(|response| {
                println!("{}", Into::<output::CliDataset>::into(response));
                Ok(())
            })
            .into_trait()
    }

    /// Print the user collaborators for a dataset.
    pub fn print_dataset_user_collaborators<P: Into<String>>(&self, id_or_name: P) -> Future<()> {
        let api = self.api.clone();
        self.api
            .get_dataset(id_or_name)
            .and_then(move |ds| api.get_dataset_user_collaborators(ds.take().id().clone()))
            .and_then(|response| {
                print!(
                    "{}",
                    Into::<output::CliUsers>::into(response).table_with_roles()
                );
                Ok(())
            })
            .into_trait()
    }

    /// Print the team collaborators for a dataset.
    pub fn print_dataset_team_collaborators<P: Into<String>>(&self, id_or_name: P) -> Future<()> {
        let api = self.api.clone();
        self.api
            .get_dataset(id_or_name)
            .and_then(move |ds| api.get_dataset_team_collaborators(ds.take().id().clone()))
            .and_then(|response| {
                print!("{}", Into::<output::CliTeams>::into(response));
                Ok(())
            })
            .into_trait()
    }

    /// Print the organization collaborators for a dataset.
    pub fn print_dataset_organization_role<P: Into<String>>(&self, id_or_name: P) -> Future<()> {
        let api = self.api.clone();
        self.api
            .get_dataset(id_or_name)
            .and_then(move |ds| api.get_dataset_organization_role(ds.take().id().clone()))
            .and_then(|response| {
                print!("{}", Into::<output::CliOrganizationRoles>::into(response));
                Ok(())
            })
            .into_trait()
    }

    /// Print all collaborators for a dataset.
    pub fn print_all_dataset_collaborators<P: Into<String>>(&self, id_or_name: P) -> Future<()> {
        let api = self.api.clone();
        self.api
            .get_dataset(id_or_name)
            .map(|ds| ds.take().id().clone())
            .and_then(move |dataset_id| {
                api.get_dataset_organization_role(dataset_id.clone())
                    .map(|org| (api, dataset_id, org))
            })
            .and_then(|(api, dataset_id, org)| {
                api.get_dataset_team_collaborators(dataset_id.clone())
                    .map(|teams| (api, dataset_id, org, teams))
            })
            .and_then(|(api, dataset_id, org, teams)| {
                api.get_dataset_user_collaborators(dataset_id.clone())
                    .map(|users| (org, teams, users))
            })
            .and_then(|(org, teams, users)| {
                let cli_collaborators = output::CliCollaborators {
                    organizations: Into::<output::CliOrganizationRoles>::into(org),
                    teams: Into::<output::CliTeams>::into(teams),
                    users: Into::<output::CliUsers>::into(users),
                };

                print!("{}", cli_collaborators);
                Ok(())
            })
            .into_trait()
    }

    /// Creates a new, empty collection.
    pub fn create_collection<P, Q>(&self, name: P, destination: Q) -> Future<()>
    where
        P: Into<String>,
        Q: Into<String>,
    {
        let name = name.into();
        let api = self.api.clone();
        let dataset_id_or_name = destination.into();
        self.api
            .get_dataset(dataset_id_or_name.clone())
            .and_then(move |dataset| {
                api.create_collection(name.clone(), dataset.take().id().clone())
                    .into_trait()
            })
            .and_then(move |package| {
                let package = package.take();
                println!(
                    "Created collection {name} ({id})",
                    name = package.name(),
                    id = package.id()
                );
                Ok(())
            })
            .into_trait()
    }

    /// Prints the collection associated with the provided collection ID.
    pub fn print_collection<P>(&self, id: P) -> Future<()>
    where
        P: Into<PackageId>,
    {
        self.api
            .get_collection(id)
            .and_then(|response| {
                println!("{}", Into::<output::CliCollection>::into(response));
                Ok(())
            })
            .into_trait()
    }

    /// TODO download:
    pub fn download<P>(&self, id: P) -> Future<()>
    where
        P: Into<PackageId>,
    {
        self.api
            .get_package_sources(id)
            .and_then(|response| {
                let files = response.take();
                for file in files {
                    println!("- {}", file.s3_url())
                }
                Ok(())
            })
            .into_trait()
    }

    /// Given an object ID, try to resolve it as a dataset or failing that,
    /// a collection.
    pub fn where_<P>(&self, id: P) -> Future<()>
    where
        P: Into<String>,
    {
        let id = id.into();
        let print_dataset = self.print_dataset(id.clone());
        let print_collection = self.print_collection(id);
        print_dataset
            .or_else(move |_| print_collection)
            .into_trait()
    }

    /// Rename a dataset or package.
    pub fn rename<P, Q>(&self, id: P, new_name: Q) -> Future<()>
    where
        P: Into<String>,
        Q: Into<String>,
    {
        self.api
            .rename(id, new_name)
            .and_then(move |renamed: Renamed| {
                println!(
                    "Renamed \"{id}\" to \"{new_name}\"",
                    id = renamed.id,
                    new_name = renamed.new_name
                );
                Ok(())
            })
            .into_trait()
    }

    /// Move packages around.
    /// If destination is None, move the package to the dataset root
    pub fn move_package<P, Q>(&self, source: P, destination: Option<Q>) -> Future<()>
    where
        P: Into<PackageId>,
        Q: Into<PackageId>,
    {
        let destination = destination.map(Into::into);
        self.api
            .move_packages(vec![source], destination.clone())
            .and_then(move |response| {
                response
                    .success()
                    .iter()
                    .for_each(|success| match &destination {
                        Some(dest) => println!("Moved {} to {}", success, dest),
                        None => println!("Moved {} to dataset root", success),
                    });

                if !response.failures().is_empty() {
                    let msg = response
                        .failures()
                        .iter()
                        .map(|failure| format!("{}: {}", failure.id(), failure.error()))
                        .collect::<Vec<String>>()
                        .join("\n");

                    Err(Error::move_error(msg).into())
                } else {
                    Ok(())
                }
            })
            .into_trait()
    }

    /// Retrieve the user and get user's settings
    pub fn get_user_and_settings(&self) -> Future<(UserRecord, UserSettings)> {
        let db = self.db.clone();
        self.api
            .get_user_and_refresh()
            .map_err(|e| Error::invalid_login(e).into())
            .and_then(move |user| {
                db.get_or_create_user_settings(user.id.as_ref(), user.profile.as_ref())
                    .map_err(Into::into)
                    .map(|settings| (user, settings))
            })
            .into_trait()
    }

    /// Prints the persistent dataset based on the user's current profile.
    pub fn print_settings_dataset(&self) -> Future<()> {
        self.get_user_and_settings()
            .and_then(|(_, settings)| {
                if let Some(id) = settings.use_dataset_id {
                    println!("Using dataset \"{}\"", id);
                } else {
                    println!("No dataset");
                }
                Ok(())
            })
            .into_trait()
    }

    /// Sets the persistent dataset based on the user's current profile.
    fn update_settings_dataset<P>(&self, id: Option<P>) -> Future<()>
    where
        P: Into<String>,
    {
        let api = self.api.clone();
        let db = self.db.clone();
        let id = id.map(Into::into);
        self.get_user_and_settings()
            .and_then(move |(user, settings)| {
                // Validate the dataset exists, if provided
                match id {
                    Some(id) => api
                        .get_dataset(id.clone())
                        .map(|dataset| Some(dataset.id().to_string()))
                        .into_trait(),
                    None => future::ok(None).into_trait(),
                }
                .map(|dataset_id| (user, settings, dataset_id))
            })
            .and_then(move |(user, settings, dataset_id)| {
                let new_settings = settings.with_dataset(dataset_id);
                let profile = user.profile.clone();
                db.upsert_user_settings(user.id.as_ref(), user.profile.as_ref(), &new_settings)
                    .map_err(Into::into)
                    .map(|_| (profile, new_settings))
            })
            .and_then(|(profile, settings)| {
                if let Some(id) = settings.use_dataset_id {
                    println!(
                        "Using dataset \"{id}\" for \"{profile}\".",
                        id = id,
                        profile = profile
                    );
                } else {
                    println!("Cleared dataset for \"{profile}\".", profile = profile);
                }
                Ok(())
            })
            .into_trait()
    }

    /// Create a new profile, prompting the user to enter information.
    /// Once created, log in and switch to this profile.
    /// If no config.ini exists, drop into the config creation wizard.
    ///
    /// This is static because this command needs to be able to be run
    /// before config.ini is initialized.
    pub fn create_profile_prompt(db: Database) -> Future<()> {
        ps::config_file()
            .map_err(|e| config::Error::config_file_not_found(e.to_string()))
            .and_then(|path| {
                if path.exists() {
                    Config::from_config_file_and_environment().and_then(|mut config| {
                        config::api::create_profile_prompt(&mut config.api_settings).and_then(
                            |profile| config.write_to_config_file().map(|_| (config, profile)),
                        )
                    })
                } else {
                    config::start_config_wizard()
                        .map(|config| (config.clone(), config.api_settings.default_profile()))
                }
            })
            .into_future()
            .map_err(Into::into)
            .and_then(move |(config, profile)| {
                let api = api::Api::new(&db, &config, profile.environment);
                api.login(profile).map(|_| ()).into_trait()
            })
            .map_err(Into::into)
            .into_trait()
    }

    /// Sets the persistent dataset based on the user's current profile.
    pub fn set_settings_dataset<P>(&self, id: P) -> Future<()>
    where
        P: Into<String>,
    {
        self.update_settings_dataset(Some(id))
    }

    /// Clears the persistent dataset based on the user's current profile.
    pub fn clear_settings_dataset(&self) -> Future<()> {
        self.update_settings_dataset(None as Option<String>)
    }
}
