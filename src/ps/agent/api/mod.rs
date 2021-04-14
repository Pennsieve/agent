//! Agent api composes the `Pennsieve-rust` crate and the local
//! `Database` instance.

use std::collections::HashMap;
use std::path::PathBuf;
use std::{iter, result};

use futures::*;
use futures::{Future as _Future, IntoFuture};

use pennsieve_rust::api::response;
use pennsieve_rust::{model, Config, Environment as ApiEnvironment, Pennsieve};

use crate::ps::agent;
pub use crate::ps::agent::api::error::{Error, ErrorKind, Result};
use crate::ps::agent::config::api::ProfileConfig;
use crate::ps::agent::config::constants::ENVIRONMENT_OVERRIDE_PROFILE;
use crate::ps::agent::config::Config as AgentConfig;
use crate::ps::agent::database::{Database, UploadRecord, UploadRecords, UserRecord};
use crate::ps::agent::messages::Response;
use crate::ps::agent::{server, upload, Future};
use crate::ps::util::futures::{to_future_trait, PSFuture};
use crate::ps::util::{actor as a, strings as s};

pub mod error;

pub use pennsieve_rust::model::{DatasetId, DatasetNodeId, OrganizationId, PackageId, UserId};

/// A validator for string values.
pub trait Validator: Send + Sync + 'static {
    fn validate(&self, value: &str) -> Result<bool>;
}

// Simplified validators for files that are enqueued via an external process,
// like over a websocket.

struct SimpleDatasetValidator;

impl Validator for SimpleDatasetValidator {
    /// Given a dataset identifier, validate it, returning a boolean indicating
    /// if the dataset is allowed to be created, or an error occurred during
    /// validation.
    fn validate(&self, identifier: &str) -> Result<bool> {
        if s::looks_like_dataset_node_id(identifier) {
            return Err(ErrorKind::DatasetReservedName.into());
        }
        Ok(true)
    }
}

struct SimplePackageValidator;

impl Validator for SimplePackageValidator {
    /// validation.
    fn validate(&self, identifier: &str) -> Result<bool> {
        if s::looks_like_package_node_id(identifier) {
            return Err(ErrorKind::PackageReservedName.into());
        }
        Ok(true)
    }
}

#[derive(Clone)]
pub struct Api {
    ps: Pennsieve,
    db: Database,
    config: AgentConfig,
}

/// The result of a renaming operation
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Renamed {
    pub id: String,
    pub new_name: String,
}

impl Api {
    /// Creates a new `Api` instance.
    pub fn new(db: &Database, config: &AgentConfig, environment: ApiEnvironment) -> Self {
        let ps = Pennsieve::new(Config::new(environment));
        Self {
            ps: ps.clone(),
            db: db.clone(),
            config: config.clone(),
        }
    }

    /// Returns an instance of the Pennsieve platform client.
    pub fn client(&self) -> &Pennsieve {
        &self.ps
    }

    /// Get the record of the currently "active" in user.
    ///
    /// Which user is active is determined as follows:
    ///
    /// - If an API key/secret pair is present in the environment, it will
    ///   take precedence and the user attached to the keypair will be returned.
    ///
    /// - If a prior login exists for on a profile defined in ~/.pennsieve/config.ini,
    ///   a new login will be attempted and the session will be refreshed as
    ///   needed.
    ///
    /// - If the user is not currently logged in, the Future will resolve with
    ///   an error.
    ///
    pub fn get_user_and_refresh(&self) -> Future<UserRecord> {
        let ps = self.ps.clone();
        self.db
            .get_user()
            .map(|user| {
                if self.config.environment_override {
                    self.login_with_profile(ENVIRONMENT_OVERRIDE_PROFILE)
                } else {
                    match user {
                        Some(u) => {
                            if u.is_token_valid() {
                                future::ok(u).into_trait()
                            } else {
                                self.login_with_profile(u.profile)
                            }
                        }
                        None => self.login_default(),
                    }
                }
            })
            .into_future()
            .flatten()
            .and_then(move |user| {
                ps.set_session_token(Some(model::SessionToken::new(user.session_token.clone())));
                ps.set_current_organization(Some(&model::OrganizationId::new(
                    user.organization_id.clone(),
                )));
                future::ok(user)
            })
            .into_trait()
    }

    /// Log into the Pennsieve platform using the default profile in config.ini.
    /// If successful, the Future will resolve with the corresponding user record.
    pub fn login_default(&self) -> Future<UserRecord> {
        self.login(self.config.api_settings.default_profile())
    }

    /// Log into the Pennsieve platform using a given profile that corresponds to
    /// a profile defined in config.ini.
    /// If successful, the Future will resolve with the corresponding user record.
    pub fn login_with_profile<S: Into<String>>(&self, profile: S) -> Future<UserRecord> {
        match self.config.api_settings.get_profile(profile) {
            Some(profile_config) => self.login(profile_config),
            None => Err(ErrorKind::NoUserProfileError.into())
                .into_future()
                .into_trait(),
        }
    }

    pub fn login(&self, profile: ProfileConfig) -> Future<UserRecord> {
        let db = self.db.clone();
        let api_key = profile.token.clone();
        let api_secret = profile.secret.clone();
        let ps = self.ps.clone();
        ps.set_environment(profile.environment);

        ps.login(api_key, api_secret)
            .and_then(move |session| {
                ps.get_organization_by_id(model::OrganizationId::new(
                    session.organization().clone(),
                ))
                .map(|org| (ps, session, org))
            })
            .and_then(|(ps, session, org)| ps.get_user().map(|user| (session, user, org)))
            .map_err(Into::<agent::Error>::into)
            .and_then(
                move |(session, user, org)| -> future::FutureResult<_, agent::Error> {
                    let o = org.organization();
                    let mut user = UserRecord::new(
                        user.id(),
                        user.email().clone(),
                        session.session_token(),
                        profile.profile,
                        profile.environment,
                        o.id(),
                        o.name().clone(),
                        o.encryption_key_id(),
                    );
                    db.upsert_user(&mut user)
                        .map(|_| user)
                        .map_err(Into::into)
                        .into()
                },
            )
            .into_trait()
    }

    // Resolution rules for dataset/package identifer combinations
    // -----------------------------------------------------------
    //
    // 1. Dataset string + Package string given:
    //
    //   * Resolve the dataset string:
    //
    //   ** Dataset exists:
    //
    //      (1) package string is a name. See if it exists and if so, use it.
    //          Otherwise, create a new package with a name equal to the
    //          string.
    //      (2) package string is an ID. See if it exists. Verify that the
    //          dataset that owns it is the same as the one that was given.
    //          Otherwise, error.
    //
    //   ** Dataset does not exist:
    //
    //      (1) Dataset string must be a name and not an ID. If valid,
    //          create a new dataset with the given name. Otherwise, error.
    //      (2) Package string must be a name and not an ID (since a package
    //          with an assigned ID *cannot* exist as it must already be linked
    //          to an existing dataset). If valid, create a package in the
    //          dataset with the given name. Otherwise, error.
    //
    // 2. Only dataset name/ID given:
    //
    //    * Resolve the dataset string:
    //
    //    ** Dataset exists: use it.
    //
    //    ** Dataset does not exist:
    //
    //      (1) Check that the ID does not resemble a dataset node ID.
    //        If not, error.
    //      (2) Create a dataset with a name equal to the given string.
    //
    // 3. Only package name/ID given:
    //
    //    * Resolve the package string:
    //
    //    ** Package exists: use its parent dataset.
    //
    //    ** Package does not exist: error.
    //
    // 4. Neither given:
    //
    //    * Error in all cases.
    //
    fn resolve_dataset_and_package<D, P, VD, VF>(
        &self,
        dataset_name_or_id: Option<D>,
        package_name_or_id: Option<P>,
        validate_dataset: VD,
        validate_package: VF,
    ) -> Future<(model::DatasetNodeId, Option<model::PackageId>)>
    where
        D: Into<String>,
        P: Into<String>,
        VD: Validator,
        VF: Validator,
    {
        match (dataset_name_or_id, package_name_or_id) {
            (Some(d), Some(p)) => {
                let ds_ident: String = d.into();
                let pkg_ident: String = p.into();
                let pkg_name: String = pkg_ident.clone();
                let pkg_name_clone: String = pkg_name.clone();
                let pkg_id: model::PackageId = pkg_ident.into();
                let this = self.clone();
                self.get_or_create_dataset(ds_ident, validate_dataset)
                    .and_then(move |ds_dto: response::Dataset| {
                        // If the packages matches something we have in the dataset, return it
                        // immediately:
                        if let Some(pkg_dto) = ds_dto.get_package_by_name(pkg_name.clone()) {
                            let ds_id: model::DatasetNodeId = ds_dto.id().clone();
                            let pkg_id: model::PackageId = pkg_dto.id().clone();
                            return Ok((ds_id, Some(pkg_id))).into_future().into_trait();
                        }

                        let ds_id: model::DatasetNodeId = ds_dto.id().clone();
                        let ds_id_inner = ds_id.clone();

                        // Otherwise, try to resolve the package based on the identifier
                        // (at this point, ID) we've been given.
                        //
                        // Note: we need to use the API to look up the package directly
                        // because the dataset `get_package_by_id()` method will
                        // only return top-level child packages contained within it.
                        this.get_collection(pkg_id)
                            .and_then(move |pkg_dto: response::Package| {
                                // If found, verify that the package's dataset matches that
                                // of the the dataset we found earlier:
                                if ds_id != pkg_dto.dataset_id().clone() {
                                    Err(Error::invalid_folder(pkg_name).into())
                                } else {
                                    let pkg_id: model::PackageId = pkg_dto.id().clone();
                                    Ok((ds_id, Some(pkg_id)))
                                }
                            })
                            .or_else(move |_| {
                                match validate_package.validate(&pkg_name_clone) {
                                    Ok(allowed) => {
                                        if !allowed {
                                            return future::err::<_, agent::Error>(
                                                ErrorKind::UserCancelledError.into(),
                                            )
                                            .into_trait();
                                        }
                                        // Create a new collection under the current dataset:
                                        this.create_collection(pkg_name_clone, ds_id_inner)
                                            .map(|pkg_dto: response::Package| {
                                                let pkg: model::Package = pkg_dto.take();
                                                let ds_id: model::DatasetNodeId =
                                                    pkg.dataset_id().clone();
                                                let pkg_id: model::PackageId = pkg.id().clone();
                                                (ds_id, Some(pkg_id))
                                            })
                                            .into_trait()
                                    }
                                    Err(e) => future::err::<_, agent::Error>(e.into()).into_trait(),
                                }
                            })
                            .into_trait()
                    })
                    .into_trait()
            }
            (Some(d), None) => {
                // Look up the dataset by its identifier:
                self.get_or_create_dataset(d, validate_dataset)
                    .map(|ds: response::Dataset| {
                        let ds: model::Dataset = ds.take();
                        let ds_id: model::DatasetNodeId = ds.id().clone();
                        (ds_id, None)
                    })
                    .into_trait()
                // otherwise, fail
            }
            (None, Some(p)) => {
                // Look up the package by its identifier:
                self.get_collection(Into::<String>::into(p))
                    .and_then(|pkg: response::Package| {
                        // Package exists; get its parent dataset and return:
                        let p: model::Package = pkg.take();
                        let ds_id: model::DatasetNodeId = p.dataset_id().clone();
                        let pkg_id: model::PackageId = p.id().clone();
                        Ok((ds_id, Some(pkg_id)))
                    })
                    .into_trait()
                // otherwise, fail
            }
            _ => Err(ErrorKind::MissingDatasetPackage.into())
                .into_future()
                .into_trait(),
        }
    }

    /// A simplified file queueing inteface intended to be called from
    /// the status server upon a file or files being enqueued by an external
    /// process. This method assumes the upload case where (1) a target dataset
    /// exists, and (2) an optional package name or ID is provided.
    pub fn queue_uploads_simple<F, D, P>(
        &self,
        dataset_id_or_name: D,
        package_id_or_name: Option<P>,
        files: Vec<F>,
        append: bool,
        recursive: bool,
    ) -> Future<UploadRecords>
    where
        D: Into<String>,
        P: Into<String>,
        F: Into<String>,
    {
        self.queue_uploads(
            files,
            Some(dataset_id_or_name.into()),
            package_id_or_name, // package_id_or_name
            append,             // append
            true,               // force
            recursive,          // recursive
            SimpleDatasetValidator,
            SimplePackageValidator,
        )
    }

    /// Queues matching files for upload to the Pennsieve platform given a
    /// path and inclusion/exclusion pattern globs.
    #[allow(clippy::too_many_arguments)]
    pub fn queue_uploads<F, D, P, VD, VF>(
        &self,
        files: Vec<F>,
        dataset_id_or_name: Option<D>,
        package_id_or_name: Option<P>,
        append: bool,
        force: bool,
        recursive: bool,
        validate_dataset: VD,
        validate_folder: VF,
    ) -> Future<UploadRecords>
    where
        F: Into<String>,
        D: Into<String>,
        P: Into<String>,
        VD: Validator,
        VF: Validator,
    {
        let files: Vec<String> = files.into_iter().map(|f| f.into()).collect();
        let dataset_id_or_name: Option<String> = dataset_id_or_name.map(Into::into);
        // Packages are handled in the following manner:
        //
        // If `package_id_or_name` is defined:
        //
        //   (1) Attempt to treat the string value as a package ID and
        //     resolve it to a package in the Pennsieve platform.
        //     If successful, check that the dataset that owns the package is
        //     the same as the dataset assigned to the dataset
        //     `dataset_id_or_name` resolves to. If not, abort queueing
        //     files for upload.
        //
        //   (2) Treat `package_id_or_name` as a folder name at the top level
        //     of the dataset, and attempt to resolve it to a package contained
        //     in the dataset `dataset_id_or_name` resolves to. If it exists,
        //     use the resolved package. Otherwise, create a new package
        //     with a name equal to `package_id_or_name` and make the
        //     dataset resolved from `dataset_id_or_name` its parent.
        let package_id_or_name: Option<String> = package_id_or_name.map(Into::into);
        let ps = self.ps.clone();
        let db = self.db.clone();
        let this = self.clone();

        let preview_dataset_id_or_name = dataset_id_or_name.clone();
        let preview_package_id_or_name = package_id_or_name.clone();

        // Step 1: Make sure a valid session exists:
        self.get_user_and_refresh()
            .map(move |user| {
                let organization_id: OrganizationId = user.organization_id.into();
                (ps, dataset_id_or_name, package_id_or_name, organization_id)
            })
            // Step 2: Resolve the given dataset name or ID and package name or ID
            // to a real dataset and package objects in the Pennsieve system:
            .and_then(move |(ps, dataset_id, package_id_or_name, organization_id)| {
                this.resolve_dataset_and_package(dataset_id, package_id_or_name, validate_dataset, validate_folder)
                    .map(|(dataset_id, package_id)| (ps, dataset_id, package_id, organization_id))
            })
            // Step 3. Refresh the dataset data to get at the int ID of the dataset itself.
            // This is needed so we can call out to the upload service.
            .and_then(|(ps, dataset_id, package_id, organization_id)| {
                ps.get_dataset(dataset_id)
                    .map(|ds| (ps, ds.take(), package_id, organization_id))
                    .map_err(Into::into)
            })
            // Step 3A. If append = true, check that the package is both defined
            // and a timeseries package:
            .and_then(move |(ps, dataset_id, package_id, organization_id)| {
                match (append, package_id.clone()) {
                    (true, None) => {
                        return future::err::<_, agent::Error>(ErrorKind::PackageMustExistForAppending.into())
                            .into_future()
                            .into_trait();
                    },
                    (true, Some(pkg_id)) => {
                      ps.get_package_by_id(pkg_id)
                        .map_err(Into::into)
                        .and_then(|pkg_dto: response::Package| {
                          let pkg: model::Package = pkg_dto.take();
                          let pkg_type_s: Option<String> = pkg.package_type().map(|p| p.to_lowercase());
                          let pkg_type_s: Option<&str> = pkg_type_s.as_ref().map(|x| &**x);
                          match pkg_type_s {
                            Some("timeseries") => future::ok::<_, agent::Error>((ps, dataset_id, package_id, organization_id)),
                            _ => future::err::<_, agent::Error>(ErrorKind::MustBeATimeseriesPackageToAppendTo.into())
                          }
                        })
                        .into_trait()
                    },
                    _ => future::ok::<_, agent::Error>((ps, dataset_id, package_id, organization_id))
                      .into_trait()
                }
                .into_trait()
            })
            // Step 4. Generate a normalized and canonicalized list of files:
            .and_then(move |(ps, dataset, package_id, organization_id)| {
                upload::generate_file_preview(files, recursive)
                    .map(|preview| (ps, dataset, package_id, organization_id, preview))
                    .map_err(Into::into)
            })
            // Step 5. Register the preview with the Pennsieve platform:
            .and_then(
                move |(ps, dataset, package_id, organization_id, agent_preview)| {
                    let dataset_int_id: model::DatasetId = dataset.int_id().clone();
                    let dataset_id: model::DatasetNodeId = dataset.id().clone();
                    ps.preview_upload(
                        &organization_id,
                        &dataset_int_id,
                        agent_preview.path(),
                        agent_preview.file_paths(),
                        append,
                        recursive,
                    ).map_err(Into::into)
                     .map(|pennsieve_preview| (pennsieve_preview, agent_preview, dataset_id, package_id, organization_id))
                }
            )
            // Step 6. Confirm the files will actually be uploaded:
            .and_then(
                move |(pennsieve_preview, agent_preview, dataset_id, package_id, organization_id)| {
                    // build a map from uploadId to the absolute path of each file
                    let agent_preview_file_map: HashMap<model::UploadId, PathBuf> = agent_preview.into_iter()
                        .map(|preview_item| (preview_item.upload_id(), preview_item.full_path().clone()))
                        .collect();

                    Into::<upload::UploadPreview>::into(pennsieve_preview)
                        .display_and_confirm(
                            &agent_preview_file_map,
                            // this should always be defined by this point anyway
                            preview_dataset_id_or_name.unwrap_or_else(|| dataset_id.to_string()),
                            preview_package_id_or_name,
                            force,
                        )
                        .map(|pennsieve_preview| {
                            (pennsieve_preview, agent_preview_file_map, dataset_id, package_id, organization_id)
                        })
                        .map_err(Into::into)
                },
            )
            // Step 7. Generate a record of each file to be uploaded for storage in
            // the agent database:
            .map(
                move |(pennsieve_preview, agent_preview_file_map, dataset_id, package_id, organization_id)| {
                    pennsieve_preview
                        .iter()
                        .flat_map(|ref p| {
                            let files = p.files();
                            let n = files.len();

                            files
                                .iter()
                                .zip(iter::repeat(p.import_id()).take(n)) // pair each file with a copy of the import ID
                                .map(|(ref s3_file, import_id)| {
                                    s3_file.upload_id()
                                        .ok_or_else(|| Into::<agent::Error>::into(
                                            Error::invalid_upload_response("Response did not contain an upload id.")
                                        ))
                                        .and_then(|upload_id| {
                                            agent_preview_file_map.get(upload_id)
                                                .ok_or_else(|| {
                                                    Error::invalid_upload_response(
                                                        format!("Response contained an unexpected upload_id: {:?}", upload_id)
                                                    ).into()
                                                })
                                        })
                                        .and_then(|file_path| {
                                            // Send a status update:
                                            a::send_unconditionally::<server::StatusServer, _>(
                                                Response::file_queued_for_upload(file_path.clone(), import_id.clone()),
                                            );
                                            UploadRecord::new(
                                                file_path,
                                                dataset_id.clone(),
                                                package_id.clone(),
                                                organization_id.clone(),
                                                import_id,
                                                append,
                                                s3_file
                                                    .chunked_upload()
                                                    .map(|properties| properties.chunk_size),
                                                s3_file.multipart_upload_id().map(Into::into),
                                            ).map_err(Into::into)
                                        })
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                },
            )
            // Step 8. Store the records:
            .and_then(|upload_records| {
                stream::iter_result(upload_records)
                    .map(move |mut record| {
                        db.insert_upload(&record).map(|id| {
                            record.id = Some(id as i64);
                            record
                        })
                    })
                    .map_err(Into::into)
                    .collect()
            })
            // Done
            .and_then(|success| {
                success
                    .into_iter()
                    .collect::<result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
                    .into_future()
            })
            .and_then(|records| Ok(Into::<UploadRecords>::into(records)))
            .into_trait()
    }

    pub fn get_upload_file_hash(&self, upload_id: usize) -> Future<response::FileHash> {
        let ps = self.ps.clone();
        let db = self.db.clone();
        let db_clone = db.clone();
        self.get_user_and_refresh()
            .and_then(move |_| {
                db_clone
                    .get_upload_by_upload_id(upload_id)
                    .map_err(Into::<agent::Error>::into)
                    .into_future()
            })
            .and_then(move |upload| {
                if upload.is_package_completed() {
                    let file_path = PathBuf::from(upload.file_path);

                    // Since this path is stored in the DB, we know it is
                    // a path to a file (so file_name() will work) and we
                    // know that it contains valid unicode (so to_str()
                    // will also work)
                    let file_name = file_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap();

                    ps.get_upload_hash(&upload.import_id.into(), file_name)
                        .map_err(Into::<agent::Error>::into)
                        .into_trait()
                } else {
                    future::failed(
                        Error::invalid_upload(format!(
                            "upload {} ({}) is not complete",
                            upload_id, upload.status
                        ))
                        .into(),
                    )
                    .into_trait()
                }
            })
            .into_trait()
    }

    /// Get all organizations the current user is member of.
    pub fn get_organizations(&self) -> Future<response::Organizations> {
        let ps = self.ps.clone();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_organizations().map_err(Into::into))
            .into_trait()
    }

    /// Get the members that belong to the users organization.
    pub fn get_members(&self) -> Future<Vec<model::User>> {
        let ps = self.ps.clone();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_members().map_err(Into::into))
            .into_trait()
    }

    /// Get the teams that belong to the users organization.
    pub fn get_teams(&self) -> Future<Vec<response::Team>> {
        let ps = self.ps.clone();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_teams().map_err(Into::into))
            .into_trait()
    }

    /// Create a new package.
    pub fn create_package<D, N, P>(
        &self,
        name: N,
        type_: P,
        dataset: D,
    ) -> Future<response::Package>
    where
        D: Into<DatasetNodeId>,
        N: Into<String>,
        P: Into<String>,
    {
        let ps = self.ps.clone();
        let name = name.into();
        let type_ = type_.into();
        let dataset_id = dataset.into();
        self.get_user_and_refresh()
            .and_then(move |_| {
                // TODO: allow creating nested collections
                ps.create_package(name.clone(), type_, dataset_id, None as Option<String>)
                    .map_err(Into::into)
            })
            .into_trait()
    }

    /// Get the source files of a package.
    pub fn get_package_sources<P>(&self, id: P) -> Future<response::Files>
    where
        P: Into<PackageId>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_package_sources(id.clone()).map_err(Into::into))
            .into_trait()
    }

    /// Updates an existing package.
    pub fn update_package<P, Q>(&self, id: P, new_name: Q) -> Future<response::Package>
    where
        P: Into<PackageId>,
        Q: Into<String>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        let name = new_name.into();
        self.get_user_and_refresh()
            .and_then(move |_| ps.update_package(id.clone(), name).map_err(Into::into))
            .into_trait()
    }

    /// Get a specific collection.
    pub fn get_collection<P>(&self, id: P) -> Future<response::Package>
    where
        P: Into<PackageId>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_package_by_id(id.clone()).map_err(Into::into))
            .into_trait()
    }

    /// Move packages to a new destination
    /// If destination is None, move packages to the top level of the dataset
    pub fn move_packages<P, Q>(
        &self,
        targets: Vec<P>,
        destination: Option<Q>,
    ) -> Future<response::MoveResponse>
    where
        P: Into<PackageId>,
        Q: Into<PackageId>,
    {
        let targets = targets.into_iter().map(Into::into).collect::<Vec<_>>();
        let destination = destination.map(Into::into);
        let ps = self.ps.clone();
        self.get_user_and_refresh()
            .and_then(move |_| ps.mv(targets, destination).map_err(Into::into))
            .into_trait()
    }

    /// Create a new collection.
    pub fn create_collection<P, Q>(&self, name: P, dataset: Q) -> Future<response::Package>
    where
        P: Into<String>,
        Q: Into<DatasetNodeId>,
    {
        self.create_package(name, "Collection", dataset)
    }

    /// Create a new dataset.
    pub fn create_dataset<P, Q>(&self, name: P, description: Option<Q>) -> Future<response::Dataset>
    where
        P: Into<String>,
        Q: Into<String>,
    {
        let ps = self.ps.clone();
        let name = name.into();
        let description = description.map(Into::into);
        self.get_user_and_refresh()
            .and_then(move |_| {
                ps.create_dataset(name.clone(), description)
                    .map_err(Into::into)
            })
            .into_trait()
    }

    /// Get all datasets.
    pub fn get_datasets(&self) -> Future<Vec<response::Dataset>> {
        let ps = self.ps.clone();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_datasets().map_err(Into::into))
            .into_trait()
    }

    /// Get a specific dataset, either by id or by name.
    pub fn get_dataset<P>(&self, id_or_name: P) -> Future<response::Dataset>
    where
        P: Into<String>,
    {
        let ps = self.ps.clone();
        let id_or_name = id_or_name.into();
        self.get_user_and_refresh()
            .and_then(move |_| ps.get_dataset(id_or_name.clone()).map_err(Into::into))
            .into_trait()
    }

    /// Attempts to get a dataset by its name or ID.
    ///
    /// If the dataset exists, it will be returned.
    ///
    /// If it does not exist, the specified name will be passed to a validator
    /// function. If the validator function evaluates to true, the dataset will
    /// be created, otherwise the operation will fail.
    pub fn get_or_create_dataset<P, V>(
        &self,
        id_or_name: P,
        validate: V,
    ) -> Future<response::Dataset>
    where
        P: Into<String>,
        V: Validator,
    {
        let id_or_name = id_or_name.into();
        let ps = self.ps.clone();
        self.get_dataset(id_or_name.clone())
            .then(move |result| {
                match result {
                    Ok(dataset) => Ok(dataset).into_future().into_trait(),
                    // if not, try to create it:
                    Err(_) => match validate.validate(&id_or_name) {
                        Ok(allowed) => {
                            if !allowed {
                                future::err::<_, agent::Error>(ErrorKind::UserCancelledError.into())
                                    .into_trait()
                            } else {
                                to_future_trait(
                                    ps.create_dataset(id_or_name, None as Option<String>)
                                        .map_err(Into::<agent::Error>::into),
                                )
                            }
                        }
                        Err(e) => future::err::<_, agent::Error>(e.into()).into_trait(),
                    },
                }
            })
            .into_trait()
    }

    /// Get the user collaborators of the dataset.
    pub fn get_dataset_user_collaborators<P>(&self, id: P) -> Future<Vec<model::User>>
    where
        P: Into<DatasetNodeId>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        self.get_user_and_refresh()
            .and_then(move |_| {
                ps.get_dataset_user_collaborators(id.clone())
                    .map_err(Into::into)
            })
            .into_trait()
    }

    /// Get the team collaborators of the dataset.
    pub fn get_dataset_team_collaborators<P>(&self, id: P) -> Future<Vec<model::Team>>
    where
        P: Into<DatasetNodeId>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        self.get_user_and_refresh()
            .and_then(move |_| {
                ps.get_dataset_team_collaborators(id.clone())
                    .map_err(Into::into)
            })
            .into_trait()
    }

    /// Get the team collaborators of the dataset.
    pub fn get_dataset_organization_role<P>(&self, id: P) -> Future<response::OrganizationRole>
    where
        P: Into<DatasetNodeId>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        self.get_user_and_refresh()
            .and_then(move |_| {
                ps.get_dataset_organization_role(id.clone())
                    .map_err(Into::into)
            })
            .into_trait()
    }

    /// Update an existing dataset.
    pub fn update_dataset<P, Q, R>(
        &self,
        id: P,
        new_name: Q,
        new_description: Option<R>,
    ) -> Future<response::Dataset>
    where
        P: Into<DatasetNodeId>,
        Q: Into<String>,
        R: Into<String>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        let name = new_name.into();
        let description = new_description.map(Into::into);
        to_future_trait(self.get_user_and_refresh().and_then(move |_| {
            ps.update_dataset(id.clone(), name, description)
                .map_err(Into::into)
        }))
    }

    /// Delete an existing dataset.
    pub fn delete_dataset<P>(&self, id: P) -> Future<()>
    where
        P: Into<DatasetNodeId>,
    {
        let ps = self.ps.clone();
        let id = id.into();
        to_future_trait(
            self.get_user_and_refresh()
                .and_then(move |_| ps.delete_dataset(id.clone()).map_err(Into::into)),
        )
    }

    /// Given a string, attempts to rename the specified object. The object will
    /// be interpreted as a dataset ID, dataset name, or a package ID.
    pub fn rename<P, Q>(&self, id_or_name: P, new_name: Q) -> Future<Renamed>
    where
        P: Into<String>,
        Q: Into<String>,
    {
        let ps = self.ps.clone();
        let id_or_name = id_or_name.into();
        let new_name = new_name.into();
        let renamed = Renamed {
            id: id_or_name.clone(),
            new_name: new_name.clone(),
        };
        self.get_user_and_refresh()
            .and_then(move |_| {
                let ps_inner = ps.clone();
                let id_inner = id_or_name.clone();
                let new_name_inner = new_name.clone();

                // Find the requested dataset or package
                ps.get_dataset(id_or_name.clone())
                    .map(move |dataset| (ps, Some(dataset), None))
                    .map_err(Into::<agent::Error>::into)
                    .or_else(move |_| {
                        ps_inner
                            .get_package_by_id(PackageId::new(id_inner.clone()))
                            .map(move |package| (ps_inner, None, Some(package)))
                            .map_err(Into::into)
                    })
                    // then rename it
                    .and_then(move |(ps, maybe_dataset, maybe_package)| {
                        if let Some(dataset) = maybe_dataset {
                            to_future_trait(
                                ps.update_dataset(
                                    dataset.id().clone(),
                                    new_name_inner.clone(),
                                    None as Option<String>,
                                )
                                .map(|_| ())
                                .map_err(Into::into),
                            )
                        } else {
                            // maybe_package.is_some() == true
                            let package = maybe_package.unwrap();
                            to_future_trait(
                                ps.update_package(package.id().clone(), new_name_inner.clone())
                                    .map(|_| ())
                                    .map_err(Into::into),
                            )
                        }
                    })
            })
            .and_then(|_| Ok(renamed))
            .into_trait()
    }
}
