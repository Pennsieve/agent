use std::cell::RefCell;
use std::collections::HashMap;
use std::result;
use std::time::{Duration, Instant};

use actix::prelude::*;
use futures::future;
use futures::{Future as _Future, *};
use futures_cpupool::CpuPool;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use log::*;
use time;
use tokio::timer::Interval;

use pennsieve_macros::try_future;

use crate::ps::agent::config::constants::UPLOAD_PROGRESS_MAX_BARS;
use crate::ps::agent::database::{Database, UploadRecords, UploadStatus};
use crate::ps::agent::messages::{Response, SystemShutdown, WorkerStartup};
use crate::ps::agent::types::{OutputFormat, ServiceFuture, ServiceId, WithProps, Worker};
use crate::ps::agent::{self, config, server, Future};
use crate::ps::util::actor as a;
use crate::ps::util::futures::*;

use super::{Error, Result};

lazy_static! {
    static ref PROGRESS_BAR_STYLE: ProgressStyle = ProgressStyle::default_bar()
        .template(config::constants::UPLOAD_PROGRESS_BAR_FORMAT)
        .progress_chars(config::constants::UPLOAD_PROGRESS_CHARACTERS);
    static ref ERROR_PROGRESS_BAR_STYLE: ProgressStyle = ProgressStyle::default_bar()
        .template(config::constants::UPLOAD_ERROR_PROGRESS_BAR_FORMAT)
        .progress_chars(config::constants::UPLOAD_PROGRESS_CHARACTERS);
}

// key to identify the single bar used for displaying the progress of
// ManyFiles uploads
const TOTAL_BAR_KEY: &str = "total";

/// Thread local actor state:
thread_local! {
    static MULTI_PROGRESS_BAR: RefCell<Option<MultiProgress>> = RefCell::new(None);
}

/// The state of the progress display loop
struct UpdateState {
    bars: HashMap<String, ProgressBar>,
    upload_started_at: time::Timespec,
    mode: RenderMode,
}

/// An enumeration that controls watcher behavior when it first starts up.
///
/// # Variants
///
/// * NoEmptyQueue - When the watcher starts, if there are no active uploads
///     (queued and in-progress), the watcher will exit immediately.
///
/// * AllowEmptyQueue - When the watcher starts, if there are no active
///     uploads (queued and in-progress), the watcher will wait indefinitely
///     until an upload is queued. An optional port can also be given where
///     new upload requests can be requested.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum StartMode {
    NoEmptyQueue,
    AllowEmptyQueue(Option<u16>),
}

impl StartMode {
    /// Tests if an empty upload queue is disallowed.
    pub fn no_empty_queue(self) -> bool {
        self == StartMode::NoEmptyQueue
    }

    /// Tests if an empty upload queue is allowed.
    pub fn allow_empty_queue(self) -> bool {
        match self {
            StartMode::AllowEmptyQueue(_) => true,
            _ => false,
        }
    }
}

impl Default for StartMode {
    fn default() -> Self {
        StartMode::NoEmptyQueue
    }
}

/// An enumeration that controls watcher stopping behavior.
///
/// # Variants
///
/// * Never - The watcher will watch forever and never terminate, regardless
///     of whether there are queued files present, no queued files,
///     or uploads that just completed.
///
/// * OnFinish - The watcher will terminate as soon as there are
///     no longer any active (queued and in-progress) uploads.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum StopMode {
    Never,
    OnFinish,
}

impl StopMode {
    pub fn never(self) -> bool {
        self == StopMode::Never
    }

    pub fn on_finish(self) -> bool {
        self == StopMode::OnFinish
    }
}

impl Default for StopMode {
    fn default() -> Self {
        StopMode::OnFinish
    }
}

/// An enumeration of all watcher rendering modes.
///
/// # Variants
///
/// * FewFiles - The number of files in this upload is small enough
///     that it is reasonable to give each file its own progress bar.
///
/// * ManyFiles - The number of files is potentially very large, we
///     should only display the bare minimum amount of information to show
///     the user how much progress has been made.
enum RenderMode {
    FewFiles,
    ManyFiles,
}

impl RenderMode {
    // Given an amount of files to be uploaded, get the monitor mode
    // that should be used.
    fn get_mode(number_of_files: u64) -> Self {
        if number_of_files > UPLOAD_PROGRESS_MAX_BARS {
            RenderMode::ManyFiles
        } else {
            RenderMode::FewFiles
        }
    }
}

///

/// An enumeration of all watcher types
#[derive(Copy, Clone, Default, Hash, PartialEq)]
pub struct UploadWatcher;

#[derive(Clone)]
pub struct Props {
    pub db: Database,
    pub output: OutputFormat,
    pub interval_ms: u64,
    pub parallelism: usize,
    pub start_mode: StartMode,
    pub stop_mode: StopMode,
}

impl Actor for UploadWatcher {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} actor", self.id());
    }
}

impl WithProps for UploadWatcher {
    type Props = Props;
}

impl Supervised for UploadWatcher {}

impl SystemService for UploadWatcher {
    fn service_started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} system service", self.id());
    }
}

// It is also possible to return a Future here as well (see `ServiceFuture`):
impl Handler<WorkerStartup> for UploadWatcher {
    type Result = ();

    fn handle(&mut self, _msg: WorkerStartup, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id();
        Arbiter::spawn(ServiceFuture::wrap(self.run()).map_err(move |e| {
            e.render_with_context(id);
            a::send_unconditionally::<server::StatusServer, _>(Response::error(e));
        }))
    }
}

impl Worker for UploadWatcher {
    fn id(&self) -> ServiceId {
        ServiceId("UploadWatcher")
    }
}

impl UploadWatcher {
    /// Get the filepath of an upload in the given uploads list with
    /// the status of InProgress
    fn get_in_progress_upload(uploads: UploadRecords) -> Option<String> {
        uploads
            .into_iter()
            .filter(|u| u.status == UploadStatus::InProgress && !u.is_file_upload_completed())
            .map(|u| u.file_path.clone())
            .next()
    }

    /// A function that initializes the state of all progress bars
    fn initialize_progress_bars(output: OutputFormat, uploads: UploadRecords) -> UpdateState {
        MULTI_PROGRESS_BAR.with(|multi| multi.replace(Some(MultiProgress::new())));
        let mut bars: HashMap<String, ProgressBar> = HashMap::new();
        let total_uploads = uploads.len();
        let mode = RenderMode::get_mode(total_uploads);

        let upload_started_at: time::Timespec = uploads
            .iter()
            .fold(time::now().to_timespec(), |acc, upload| {
                std::cmp::min(upload.created_at, acc)
            });

        match mode {
            RenderMode::FewFiles => {
                MULTI_PROGRESS_BAR.with(|multi| {
                    if let Some(ref mut mpb) = *multi.borrow_mut() {
                        for (i, u) in uploads.into_iter().enumerate() {
                            if output.is_rich() {
                                let pb = mpb.add(ProgressBar::new(100));
                                pb.set_style(PROGRESS_BAR_STYLE.clone());
                                pb.set_prefix(&format!("[{}/{}]", i + 1, uploads.len()));
                                pb.set_message(u.file_path.as_str());
                                pb.set_position(u.progress as u64);
                                bars.insert(u.file_path.clone(), pb);
                            } else {
                                println!("- {}", u.summary());
                            }
                        }
                    } else {
                        // INVARIANT: the `MULTI_PROGRESS_BAR` `RefCell` contains `Some(_)`:
                        unreachable!();
                    }
                });

                if !output.is_rich() {
                    println!();
                }
            }
            RenderMode::ManyFiles => MULTI_PROGRESS_BAR.with(|multi| {
                if let Some(ref mut mpb) = *multi.borrow_mut() {
                    if output.is_rich() {
                        let pb = mpb.add(ProgressBar::new(100));
                        pb.set_style(PROGRESS_BAR_STYLE.clone());
                        pb.set_prefix(&format!("[{}/{}]", 0, total_uploads));
                        pb.set_position(0 as u64);
                        bars.insert(TOTAL_BAR_KEY.to_string(), pb);
                    } else {
                        println!("[0/{} files uploaded]", total_uploads);
                        println!();
                    }
                }
            }),
        }

        UpdateState {
            bars,
            upload_started_at,
            mode,
        }
    }

    /// A function that updates the state of all progress bars on each tick
    fn update_progress_bars(
        db: &Database,
        output: OutputFormat,
        stop_mode: StopMode,
        state: UpdateState,
    ) -> Result<UpdateState> {
        let uploads: UploadRecords =
            db.get_active_uploads_started_since(state.upload_started_at)?;

        if uploads.is_package_completed() && stop_mode.on_finish() {
            info!("cli:upload-watcher: terminate mode = {:?}", stop_mode);
            // If terminate watching upon completion, send a signal
            // to kill the watcher future:
            // Send the shutdown signal to the agent once
            // uploading is complete:
            info!("Sending shutdown...");
            a::send_unconditionally::<server::StatusServer, _>(SystemShutdown);

            let failed_uploads = uploads
                .records
                .into_iter()
                .filter(|u| u.is_failed())
                .count();

            if failed_uploads == 0 {
                return Ok(state);
            } else {
                let units = if failed_uploads == 1 {
                    "upload"
                } else {
                    "uploads"
                };
                return Err(Error::upload_error(format!(
                    "{} {} failed.",
                    failed_uploads, units
                )));
            }
        }

        match state.mode {
            RenderMode::FewFiles => {
                for u in &uploads {
                    if output.is_rich() {
                        if let Some(progress_bar) = state.bars.get(&u.file_path) {
                            progress_bar.set_position(u.progress as u64);

                            if u.is_failed() {
                                progress_bar.set_style(ERROR_PROGRESS_BAR_STYLE.clone());
                                progress_bar
                                    .set_message(&format!("{} (FAILED)", u.file_path.as_str()));
                            }
                        }
                    } else if u.is_failed() {
                        println!("- {} (FAILED)", u.summary());
                    } else if u.is_file_upload_completed() {
                        println!("- {} (done)", u.summary());
                    } else {
                        println!("- {}", u.summary());
                    }
                }

                if !output.is_rich() {
                    println!();
                }
            }
            RenderMode::ManyFiles => {
                let completed_uploads = uploads
                    .iter()
                    .filter(|upload| upload.is_file_upload_completed() && !upload.is_failed())
                    .count() as u64;
                let failed_uploads =
                    uploads.iter().filter(|upload| upload.is_failed()).count() as u64;
                let total_uploads = uploads.len();

                if output.is_rich() {
                    let prefix = if failed_uploads == 0 {
                        format!("[{}/{}]", completed_uploads, total_uploads)
                    } else {
                        format!(
                            "[{}/{} ({} failed)]",
                            completed_uploads, total_uploads, failed_uploads
                        )
                    };

                    let percent_done = (completed_uploads as f64 / total_uploads as f64) * 100.0;
                    let in_progress_upload = UploadWatcher::get_in_progress_upload(uploads);
                    let progress_bar = &state.bars[TOTAL_BAR_KEY];
                    progress_bar.set_position(percent_done as u64);
                    progress_bar.set_prefix(&prefix);
                    progress_bar.set_message(&in_progress_upload.unwrap_or_else(|| "".to_string()));
                } else if failed_uploads == 0 {
                    println!("[{}/{} files uploaded]\n", completed_uploads, total_uploads);
                } else {
                    println!(
                        "[{}/{} files uploaded ({} failed)]\n",
                        completed_uploads, total_uploads, failed_uploads
                    );
                }
            }
        }

        Ok(state)
    }

    fn run(self) -> Future<()> {
        self.watch().into_trait()
    }

    /// Watch the progress of all active uploads using this upload watcher.
    pub fn watch(self) -> Future<()> {
        let id = self.id();
        let props: Props = self
            .get_props()
            .unwrap_or_else(|| panic!("{:?}: missing props", id));

        let db = props.db;
        let output = props.output;
        let interval_ms = props.interval_ms;
        let _start_mode = props.start_mode;
        let stop_mode = props.stop_mode;

        if stop_mode.never() {
            info!("Upload watcher in listening mode");
        }

        let uploads = try_future!(db.get_active_uploads());
        let initial_state = Self::initialize_progress_bars(output, uploads);

        // Initiate a Future to update the state on every watch tick:
        let k = Interval::new(Instant::now(), Duration::from_millis(interval_ms))
            .map_err(Into::<Error>::into)
            .fold(initial_state, move |state, _tick| {
                Self::update_progress_bars(&db, output, stop_mode, state)
            });

        // Take ownership of the multiprogress bar exclusively.
        //
        // Note: if this ever needs to be done in a multi-threaded context, we
        // can wrap the `MultiProgress` instance in `Arc<RwLock<_>>` and `.clone()` +
        // `.read()` as needed.
        let multi: MultiProgress = MULTI_PROGRESS_BAR
            .with(|multi| multi.borrow_mut().take())
            .unwrap_or_else(|| panic!("{:?}: multi-progress bar already taken", id));

        // Initiate a Future to wait for all progress bars to
        // finish. Since `multi.join()` will block the thread it is
        // running on, we run it on another thread managed by the
        // `futures_cpupool`'s `CpuPool` type.
        let f = future::poll_fn(move || -> result::Result<Async<()>, Error> {
            if output.is_rich() {
                multi.join()?;
            }
            Ok(Async::NotReady)
        });

        CpuPool::new(1)
            .spawn(f)
            .join(k)
            .map(|_| ())
            .map_err(Into::<agent::Error>::into)
            .into_trait()
    }
}

#[cfg(test)]
mod test {
    use crate::ps::agent::database::UploadRecord;

    use super::*;

    fn get_upload_record(id: usize, status: UploadStatus) -> UploadRecord {
        let now = time::now().to_timespec();
        UploadRecord {
            id: None,
            file_path: String::from(id.to_string()),
            dataset_id: String::from("dataset_id"),
            import_id: String::from("import_id"),
            package_id: None,
            progress: 0,
            status,
            created_at: now,
            updated_at: now,
            append: false,
            upload_service: true,
            organization_id: String::from("organization_id"),
            chunk_size: Some(100),
            multipart_upload_id: Some(String::from("multipart_upload_id")),
        }
    }

    #[test]
    fn few_files_initial_state_contains_bar_per_file() {
        let ids = 0..5;
        let records: Vec<UploadRecord> = ids
            .clone()
            .map(|id| get_upload_record(id, UploadStatus::Queued))
            .collect();
        let uploads = UploadRecords {
            records: records.clone(),
        };

        let initial_state = UploadWatcher::initialize_progress_bars(OutputFormat::Rich, uploads);

        let mut expected_bars: Vec<String> = ids.map(|id| id.to_string()).collect();
        let mut actual_bars: Vec<String> = initial_state.bars.keys().map(|k| k.clone()).collect();

        expected_bars.sort();
        actual_bars.sort();

        assert_eq!(expected_bars, actual_bars);
    }

    #[test]
    fn many_files_initial_state_contains_a_single_bar() {
        let ids = 0..UPLOAD_PROGRESS_MAX_BARS as usize + 1;
        let records: Vec<UploadRecord> = ids
            .clone()
            .map(|id| get_upload_record(id, UploadStatus::Queued))
            .collect();
        let uploads = UploadRecords {
            records: records.clone(),
        };

        let initial_state = UploadWatcher::initialize_progress_bars(OutputFormat::Rich, uploads);

        let actual_bars: Vec<String> = initial_state.bars.keys().map(|k| k.clone()).collect();

        assert_eq!(actual_bars, vec![TOTAL_BAR_KEY]);
    }
}
