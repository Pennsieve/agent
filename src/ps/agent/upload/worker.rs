//! Upload worker that acts as the background worker for
//! persisting packages to the Pennsieve platform.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use actix::prelude::*;
use futures::{future, stream, Future as _Future, IntoFuture, Stream};
use itertools::Itertools;
use log::*;
use tokio::timer::{Delay, Interval};

use pennsieve_rust::api::{ProgressCallback, ProgressUpdate};
use pennsieve_rust::model;
use pennsieve_macros::try_future;

use crate::ps::agent::api::Api;
use crate::ps::agent::database::{Database, UploadRecord, UploadStatus};
use crate::ps::agent::messages::{QueueUpload, Response, WorkerStartup};
use crate::ps::agent::types::{ServiceId, WithProps, Worker};
use crate::ps::agent::upload::{Error, Result};
use crate::ps::agent::{self, config, server, Future};

use crate::ps::util::futures::*;
use crate::ps::util::{actor as a, futures as f};

type ImportGroup = (String, Vec<UploadRecord>);

#[derive(Clone)]
pub struct DatabaseUpdater {
    db: Database,
}

impl DatabaseUpdater {
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }
}

// We can use the `ProgressCallback` interface to create a callback that will
// update the database for a particular file when progress is made:
impl ProgressCallback for DatabaseUpdater {
    fn on_update(&self, update: &ProgressUpdate) {
        let import_id = update.import_id().clone().take();
        let file_path = update.file_path();
        let bytes_sent = update.bytes_sent();
        let part_number = update.part_number();
        let size = update.size();
        let is_done = update.is_done();
        let percent_done = update.percent_done() as i32;

        debug!(
            "DatabaseUpdater::on_update({}) : {:?} => {}",
            import_id, file_path, percent_done
        );

        // Send a status update:
        a::send_unconditionally::<server::StatusServer, _>(Response::upload_progress(
            import_id.clone(),
            file_path.to_path_buf(),
            part_number,
            bytes_sent,
            size,
            percent_done,
            is_done,
        ));

        if let Err(e) = self
            .db
            .update_file_progress(&import_id, &file_path, percent_done)
        {
            error!("upload-worker/database-updater :: {:?}", e);
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

trait IntoS3File {
    // Converts a type into Pennsieve `S3File`.
    fn into_s3_file(&self) -> Result<model::S3File>;
}

impl IntoS3File for UploadRecord {
    fn into_s3_file(&self) -> Result<model::S3File> {
        let s3_file = model::S3File::from_file_path(self.file_path.clone(), None, None)?;

        Ok(s3_file
            .with_chunk_size(self.chunk_size)
            .with_multipart_upload_id(self.multipart_upload_id.clone().map(Into::into)))
    }
}

/// Updates the upload status for a collection of `import_id`s.
/// Each `import_id` can map to one or many records.
fn update_upload_statuses(
    db: &Database,
    records: &HashMap<String, Vec<UploadRecord>>,
    status: UploadStatus,
) -> Result<()> {
    if status == UploadStatus::Failed {
        let keys = records.keys();
        if keys.len() > 0 {
            warn!(
                "Transitioning the following import_ids to {}: {:?}",
                status,
                records.keys()
            );
        }
        for key in keys {
            db.update_import_status_and_progress(key, status, 0)?;
        }
    } else if !records.is_empty() {
        debug!(
            "Transitioning the following import_ids to {}: {:?}",
            status,
            records.keys()
        );
        for key in records.keys() {
            db.update_import_status(key, status)?;
        }
    }

    Ok(())
}

/// Updates the upload status for all records with the given `import_id`.
fn update_import_status(
    db: &Database,
    import_id: &model::ImportId,
    status: UploadStatus,
    progress: Option<i32>,
) -> Result<()> {
    debug!("Transitioning import_id {:?} to {}", import_id, status);

    if let Some(progress) = progress {
        db.update_import_status_and_progress(import_id.borrow(), status, progress)
            .map(|_| ())
            .map_err(Into::into)
    } else {
        db.update_import_status(import_id.borrow(), status)
            .map(|_| ())
            .map_err(Into::into)
    }
}

/// Update an upload as failed, returning the original error in a future
fn fail_upload_with_error<T: 'static + Send>(
    db: &Database,
    import_id: &model::ImportId,
    e: Error,
) -> Result<T> {
    match update_import_status(db, import_id, UploadStatus::Failed, None) {
        Ok(_) => Err(e),          // return the previous error
        Err(other) => Err(other), // otherwise, the new error
    }
}

/// Given a upload record, extract the path, dataset and package IDs.
fn extract_identifiers(
    record: Option<&UploadRecord>,
) -> Result<(PathBuf, model::DatasetNodeId, Option<model::PackageId>)> {
    // A collection of files associated with one `import_id` cannot span datasets or packages.
    // We use this fact to derive the path, dataset, and package information based on the
    // first record.
    let base_path = record.map(|u| u.file_path.as_ref()).unwrap_or_else(|| "");
    let base_path = Path::new(base_path)
        .parent()
        .ok_or_else(|| Error::no_parent(base_path))?
        .to_owned();
    let dataset_id = record.map(|u| u.dataset_id.as_ref()).unwrap_or_else(|| "");
    let dataset_id = model::DatasetNodeId::new(dataset_id);
    let package_id = record
        .map(|u| u.package_id.as_ref())
        .unwrap_or(None)
        .map(|p| model::PackageId::new(p.to_string()));

    Ok((base_path, dataset_id, package_id))
}

#[allow(clippy::too_many_arguments)]
/// Upload using the new platform upload service.
fn upload_recursive(
    db: Database,
    api: Api,
    s3_files: Vec<model::S3File>,
    import_id: model::ImportId,
    organization_id: model::OrganizationId,
    dataset_id: model::DatasetNodeId,
    package_id: Option<model::PackageId>,
    base_path: PathBuf,
    append: bool,
    retry_number: u16,
    parallelism: usize,
) -> Future<()> {
    // the maximum amount of times we will refresh the user's token
    // during a single upload. a single upload cannot run
    // uninterrupted for more than 90 * MAX_RETRIES minutes.
    const MAX_RETRIES: u16 = 10;

    let updater = DatabaseUpdater::new(&db);

    // clone all arguments in case we need to retry this function
    let api_retry = api.clone();
    let db_retry = db.clone();
    let import_id_retry = import_id.clone();
    let organization_id_retry = organization_id.clone();
    let dataset_id_retry = dataset_id.clone();
    let package_id_retry = package_id.clone();
    let s3_files_retry = s3_files.clone();

    api.client()
        .upload_file_chunks_with_retries(
            &organization_id.clone(),
            &import_id.clone(),
            &base_path,
            s3_files,
            updater,
            parallelism,
        )
        .for_each(|import_id| {
            debug!("Done uploading {:?}", import_id);
            Ok(())
        })
        // If one file that is part of a collection of files
        // associated with an import Id fails, the whole batch
        // has to fail.
        .or_else(move |e| {
            debug!("Upload error => {:?}", e);
            match e.kind() {
                pennsieve_rust::ErrorKind::ApiError {
                    status_code: hyper::StatusCode::UNAUTHORIZED,
                    ..
                } if retry_number < MAX_RETRIES => {
                    debug!(
                        "Token expired, refreshing [{}/{}]...",
                        retry_number + 1,
                        MAX_RETRIES
                    );
                    let api_retry_clone = api_retry.clone();
                    Delay::new(Instant::now() + Duration::from_secs(10))
                        .map_err(Into::into)
                        .and_then(move |_| api_retry_clone.get_user_and_refresh())
                        .and_then(move |_| {
                            upload_recursive(
                                db_retry,
                                api_retry,
                                s3_files_retry,
                                import_id_retry,
                                organization_id_retry,
                                dataset_id_retry,
                                package_id_retry,
                                base_path,
                                append,
                                retry_number + 1,
                                parallelism,
                            )
                        })
                        .into_trait()
                }
                _ => fail_upload_with_error(&db_retry, &import_id_retry, Error::upload_failed(e))
                    .map_err(Into::into)
                    .into_future()
                    .into_trait(),
            }
        })
        .map(move |_| {
            (
                api.client().clone(),
                db.clone(),
                import_id.clone(),
                dataset_id.clone(),
                organization_id,
            )
        })
        .and_then(move |(ps, db, import_id, dataset_id, organization_id)| {
            debug!("Completing (platform): {:?}", import_id);
            let import_id_copy = import_id.clone();
            let db_copy = db.clone();
            ps.complete_upload(
                &organization_id,
                &import_id,
                &dataset_id,
                package_id.as_ref(),
                append,
            )
            .or_else(move |e| fail_upload_with_error(&db, &import_id, Error::upload_failed(e)))
            .map_err(Into::into)
            .map(|_| (db_copy, import_id_copy))
        })
        .and_then(move |(db, import_id)| {
            debug!("Completing (db): {:?}", import_id);
            update_import_status(&db, &import_id, UploadStatus::Completed, Some(100))
                .map_err(Into::into)
        })
        .into_trait()
}

/// Performs the actual file uploading operation for a given import group.
/// (An import group is tuple: an import ID + a vector of associated files to
/// upload.
fn upload(
    db: Database,
    api: Api,
    group: ImportGroup,
    parallelism: usize,
) -> Future<model::ImportId> {
    let (import_id, uploads) = group;

    // Only append if all `append` properties in the import group are also
    // set to `append=true`:
    let append = uploads.iter().all(|ref u| u.append);

    // As a sanity check, if some of the files in the import group are marked
    // as `append=true`, issue a warning:
    if !append {
        let any_append = uploads.iter().any(|ref u| u.append);
        if any_append {
            warn!(
                "Import group {import_id} is marked as `append = false`, but some \
                 uploads in it have `append = true` set!.",
                import_id = import_id.to_string()
            );
        }
    }

    let import_id: model::ImportId = model::ImportId::new(import_id);
    let (base_path, dataset_id, package_id) = match extract_identifiers(uploads.first()) {
        Ok(ids) => ids,
        Err(e) => return future::err(e.into()).into_trait(),
    };
    let organization_id: model::OrganizationId = uploads
        .first()
        .map(|rec| rec.organization_id.clone())
        .unwrap_or_else(|| String::from(""))
        .into();
    let s3_files: Result<Vec<model::S3File>> = uploads
        .iter()
        .map(|upload| upload.into_s3_file())
        .collect::<Result<_>>();
    let s3_files = match s3_files {
        Ok(s3_files) => s3_files,
        Err(e) => return future::err(e.into()).into_trait(),
    };

    info!(
        "Uploading import_id: {:?} with {} files; append = {}",
        import_id,
        uploads.len(),
        append
    );

    try_future!(update_import_status(
        &db,
        &import_id,
        UploadStatus::InProgress,
        Some(0)
    ));

    let completed_import_id = import_id.clone();

    upload_recursive(
        db.clone(),
        api.clone(),
        s3_files,
        import_id,
        organization_id,
        dataset_id,
        package_id,
        base_path,
        append,
        0,
        parallelism,
    )
    .and_then(|_| Ok(completed_import_id))
    .into_trait()
}

// Note: The implemention of the `step` function was moved into a private,
// top-level function due to the restrictions placed on `Future`s by the
// new version of Tokio (>0.17). `Future`s must have a 'static lifetime and
// be `Send`able. The use of `self` in a closure of the returned `Future`
// made returning a `Future` with a 'static lifetime was not possible.

/// Runs one upload step. One step consists of the following:
/// - Get queued and in_progress upload records.
/// - Merge and group by import_id.
/// - Get grant access to s3.
/// - Perform upload to s3.
/// - Call api /complete endpoint.
fn step(db: Database, api: &Api, parallelism: usize) -> Future<()> {
    // Get all uploads that are of `UploadStatus::Queued` status.
    let queued: Result<HashMap<String, Vec<UploadRecord>>> = db
        .get_queued_uploads()
        .map(|uploads| {
            uploads
                .into_owned_iter()
                .map(|upload| (upload.import_id.clone(), upload))
                .into_group_map()
        })
        .map_err(Into::<Error>::into);

    let queued: HashMap<String, Vec<UploadRecord>> = match queued {
        Ok(queued) => queued,
        Err(e) => return future::err(e.into()).into_trait(),
    };

    // Get all uploads that are of `UploadStatus::InProgress` status
    // and filter the results to only include records that will attempt
    // a retry. Records that will not be retried will be transitioned
    // to a `UploadStatus::Failed` status.
    let in_progress: Result<HashMap<String, Vec<UploadRecord>>> = db
        .get_in_progress_uploads()
        .map(|uploads| {
            uploads
                .into_owned_iter()
                .map(|upload| (upload.import_id.clone(), upload))
                .into_group_map()
                .into_iter()
        })
        .map(|iter| {
            iter.filter(|&(_, ref records)| {
                records
                    .first()
                    .map_or(false, |record| record.should_retry())
            })
            .partition(|&(_, ref records)| {
                records.first().map_or(true, |record| record.should_fail())
            })
        })
        .map_err(Into::<Error>::into)
        .and_then(|(failed, retry)| {
            update_upload_statuses(&db, &failed, UploadStatus::Failed).map(|_| retry)
        });

    let in_progress: HashMap<String, Vec<UploadRecord>> = match in_progress {
        Ok(in_progress) => in_progress,
        Err(e) => return future::err(e.into()).into_trait(),
    };

    // If there are no queued or in-progress uploads, bail out early:
    if queued.is_empty() && in_progress.is_empty() {
        return Ok(()).into_future().into_trait();
    }

    // Get an active Pennsieve API user and combine the `queued` and
    // `in_progress` records into one stream. The records are combined
    // by grouping by their `import_id`. Each grouping gets uploaded.
    let inner_api = api.clone();
    api.get_user_and_refresh()
        .and_then(move |_| {
            let mut pending = queued;
            pending.extend(in_progress);
            Ok(stream::iter_ok::<_, Error>(pending))
        })
        .and_then(move |stream| {
            stream
                .map_err(Into::<agent::Error>::into)
                .for_each(move |import_group| {
                    upload(
                        db.clone(),
                        inner_api.clone(),
                        import_group.clone(),
                        parallelism,
                    )
                    .map_err(move |e| {
                        let (import_id, _) = import_group;
                        a::send_unconditionally::<server::StatusServer, _>(Response::upload_error(
                            e.clone(),
                            import_id,
                        ));
                        e
                    })
                    .and_then(|import_id| {
                        a::send_unconditionally::<server::StatusServer, _>(
                            Response::upload_complete(import_id),
                        );
                        Ok(())
                    })
                })
                .map(|_| ())
        })
        .into_trait()
}

/// A type used to define an upload worker.
/// The `config` specifies what environment of the
/// Pennsieve platform this worker will be uploading to.
#[derive(Default)]
pub struct Uploader;

#[derive(Clone)]
pub struct Props {
    pub api: Api,
    pub db: Database,
    pub parallelism: usize,
}

impl Actor for Uploader {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} actor", self.id());
    }
}

impl WithProps for Uploader {
    type Props = Props;
}

impl Supervised for Uploader {}

impl SystemService for Uploader {
    fn service_started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} system service", self.id());
    }
}

// It is also possible to return a Future here as well (see `ServiceFuture`):
impl Handler<WorkerStartup> for Uploader {
    type Result = ();

    fn handle(&mut self, _msg: WorkerStartup, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id();
        Arbiter::spawn(self.run().map_err(move |e| {
            e.render_with_context(id);
            a::send_unconditionally::<server::StatusServer, _>(Response::error(e));
        }))
    }
}

// Handle requests for queueing uploads:
impl Handler<QueueUpload> for Uploader {
    type Result = ();

    fn handle(&mut self, msg: QueueUpload, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id();
        let msg = msg.clone();
        self.borrow_props(|props: Option<&Props>| {
            let props: &Props = props.unwrap_or_else(|| panic!("{:?}: missing props", id));
            let api: &Api = &props.api;
            let msg = msg.clone();

            info!(
                "Handler: queued {count} files for upload",
                count = msg.files.len()
            );

            let queue_future = api.queue_uploads_simple(
                msg.dataset,
                msg.package,
                msg.files,
                msg.append.unwrap_or(false),
                msg.recursive.unwrap_or(false),
            );

            let f = queue_future
                .map_err(move |e| {
                    e.render_with_context(id);
                    a::send_unconditionally::<server::StatusServer, _>(Response::error(e));
                })
                .and_then(|_| Ok(()));

            Arbiter::spawn(f);
        });
    }
}

impl Worker for Uploader {
    fn id(&self) -> ServiceId {
        ServiceId("Uploader")
    }
}

impl Uploader {
    /// Runs one upload step. One step consists of the following:
    /// - Get queued and in_progress upload records.
    /// - Merge and group by import_id.
    /// - Get grant access to s3.
    /// - Perform upload to s3.
    /// - Call api /complete endpoint.
    pub fn step(&self) -> Future<()> {
        debug!("Running upload step");

        let id = self.id();
        self.borrow_props(|props: Option<&Props>| {
            let props: &Props = props.unwrap_or_else(|| panic!("{:?}: missing props", id));
            debug!("Running upload step");
            step(props.db.clone(), &props.api, props.parallelism)
        })
    }

    fn run(&self) -> Future<()> {
        let id = self.id();
        let props: Props = self
            .get_props()
            .unwrap_or_else(|| panic!("{:?}: missing props", id));
        let api = props.api;
        let db = props.db;
        let parallelism = props.parallelism;

        // run one upload step every N seconds:
        let timer = Interval::new(
            Instant::now(),
            Duration::from_secs(config::constants::UPLOAD_WORKER_RUN_INTERVAL_SECS),
        );

        info!(
            "Configuring Uploader on a {} second timer",
            config::constants::UPLOAD_WORKER_RUN_INTERVAL_SECS
        );

        // Any uploads that were marked as 'in_progress' the last time the
        // upload worker was running, but did not fail or complete
        let _reset = match db.reset_stalled_uploads() {
            Ok(_reset) => _reset,
            Err(e) => return future::err(e.into()).into_trait(),
        };

        // Create a future based stream that will perform one upload
        // step based on the timer. This future will always return the
        // `Ok(())`, this is because `stream::for_each` terminates the stream
        // on `Err` conditions.
        let f = timer
            .for_each(move |_| {
                step(db.clone(), &api, parallelism).then(|res| match res {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        warn!("Uploader step failed: {:?}", e);
                        Ok(())
                    }
                })
            })
            .map_err(Into::into);

        f::to_future_trait(f)
    }
}
