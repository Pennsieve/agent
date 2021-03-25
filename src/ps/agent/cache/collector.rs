//! Cleans up cached pages according to when they were last accessed
//! and how much space the pages are currently using on the underlying
//! filesystem.

use std::time;

use actix::prelude::*;
use futures::prelude::*;
use futures::Future as _Future;
use log::*;
use tokio::timer::Interval;

use crate::ps::agent::cache::{self, Error, Page, Result};
use crate::ps::agent::config::CacheConfig as Config;
use crate::ps::agent::database::{Database, PageRecord};
use crate::ps::agent::messages::Response;
use crate::ps::agent::types::{ServiceFuture, ServiceId, WithProps, Worker};
use crate::ps::agent::{self, config, messages, server, Future};
use crate::ps::util::actor as a;
use crate::ps::util::futures::*;

/// A collector that cleans up cache pages on the underlying filesystem.
#[derive(Clone, Default, Debug)]
pub struct CachePageCollector;

#[derive(Clone, Debug)]
pub struct Props {
    pub db: Database,
    pub config: Config,
}

impl Actor for CachePageCollector {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} actor", self.id());
    }
}

impl WithProps for CachePageCollector {
    type Props = Props;
}

impl Supervised for CachePageCollector {}

impl SystemService for CachePageCollector {
    fn service_started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} system service", self.id());
    }
}

impl CachePageCollector {
    /// Removes the page from the underlying filesystem.
    pub fn remove_page(&self, record: &PageRecord) -> Result<()> {
        let id = self.id();

        self.borrow_props(|props: Option<&Props>| {
            debug!(
                "Removing page {} - last used on {}",
                record.id,
                record.str_time()
            );

            let props: &Props = props.unwrap_or_else(|| panic!("{:?}: missing props", id));
            let db = &props.db;
            let config = &props.config;

            // Removes the page in the database first, then the file system. This ordering
            // is important, because if it was reversed, the file system page can be removed,
            // then the db delete can fail. This would produce bad data responses. Doing it
            // in this order, the worst case scenario is that the underlying page will still
            // take up space on the file system, but wouldn't be accounted for in the collector.
            // This case is fixed once that page is cached again.
            db.delete_page(&record)?;

            let (package, channel, _, index) = cache::from_page_key(&record.id);
            let page = Page::new(config, &package, &channel, 0, 0, index);

            page.delete()
        })
    }

    /// Removes cache pages according to the soft aged records
    /// implementation.
    pub fn soft_recycle(&self) -> Result<i64> {
        let id = self.id();

        self.borrow_props(|props: Option<&Props>| {
            let props: &Props = props.unwrap_or_else(|| panic!("{:?}: missing props", id));
            let db = &props.db;
            let config = &props.config;
            let mut current_size = db.get_total_size()?;

            info!(
                "Running soft recycle - current_size: {} soft_cache_size: {}",
                current_size,
                config.soft_cache_size()
            );

            let recycled =
                cache::soft_cleanup(self, db, config.soft_cache_size(), &mut current_size)?;

            if recycled > 0 {
                info!("Soft recycling recaptured {} page(s)", recycled);
            }

            Ok(current_size)
        })
    }

    /// Removes cache pages according to the hard aged records
    /// implementation.
    pub fn hard_recycle(&self) -> Result<i64> {
        let id = self.id();

        self.borrow_props(|props: Option<&Props>| {
            let props: &Props = props.unwrap_or_else(|| panic!("{:?}: missing props", id));
            let db = &props.db;
            let config = &props.config;
            let mut current_size = db.get_total_size()?;

            info!(
                "Running hard recycle - current_size: {} hard_cache_size: {}",
                current_size,
                config.soft_cache_size()
            );

            let recycled =
                cache::hard_cleanup(self, db, config.hard_cache_size(), &mut current_size)?;

            if current_size as u64 > config.hard_cache_size() {
                let msg = format!(
                    "current_size: {} hard_cache_size: {}",
                    current_size,
                    config.hard_cache_size()
                );
                Err(Error::no_space(msg))
            } else {
                if recycled > 0 {
                    info!("Hard recycling recaptured {} page(s)", recycled);
                }

                Ok(current_size)
            }
        })
    }
}

// It is also possible to return a Future here as well (see `ServiceFuture`):
impl Handler<messages::WorkerStartup> for CachePageCollector {
    type Result = ();

    fn handle(&mut self, _msg: messages::WorkerStartup, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id();
        Arbiter::spawn(ServiceFuture::wrap(self.run()).map_err(move |e| {
            e.render_with_context(id);
            a::send_unconditionally::<server::StatusServer, _>(Response::error(e));
        }))
    }
}

impl Worker for CachePageCollector {
    fn id(&self) -> ServiceId {
        ServiceId("CacheCollector")
    }
}

impl CachePageCollector {
    fn run(&self) -> Future<()> {
        // This is needed due to the 'static constraint placed on the returned Future.
        // Cloning `Collector` is cheap because copies are just refcounted.
        let this = self.clone();

        // run one collector step every N minutes
        let interval =
            time::Duration::from_secs(config::constants::CACHE_COLLECTOR_RUN_INTERVAL_SECS);
        let first_run = time::Instant::now() + time::Duration::from_secs(30);
        let timer = Interval::new(first_run, interval);

        info!(
            "Configuring CacheCollector on a {} minute timer",
            config::constants::CACHE_COLLECTOR_RUN_INTERVAL_SECS / 60
        );

        // runs five soft recycles, followed by one hard recycle. This pattern
        // is followed indefinitely.
        let f = timer
            .map_err(Into::<agent::Error>::into)
            .fold(0, move |step, _| -> agent::Future<i32> {
                if step < 5 {
                    this.soft_recycle().map(|_| step + 1).or_else(|e| {
                        warn!("Soft recycle failure {:?}", e);
                        Ok(step + 1)
                    })
                } else {
                    this.hard_recycle().map(|_| 0).or_else(|e| {
                        error!("Hard recycle failure {:?}", e);
                        Ok(0)
                    })
                }
                .into_future()
                .into_trait()
            })
            .map(|_| ())
            .into_trait();

        to_future_trait(f)
    }
}

#[cfg(test)]
#[macro_use]
mod test {
    use std::path;

    use ::time::{now_utc, Duration};
    use lazy_static::lazy_static;
    use tempfile::tempdir;

    use pennsieve_macros::path;

    use super::*;
    use crate::ps::agent::cache::PageCreator;
    use crate::ps::util;

    lazy_static! {
        static ref TEMP_DIR: path::PathBuf = tempdir().unwrap().into_path();
    }

    #[test]
    fn soft_recycle_with_deletes() {
        let config = Config::new(
            &*TEMP_DIR, // base_path
            150,        // page_size
            100,        // soft_cache_size
            0,          // hard_cache_size
        );
        assert!(cache::create_page_template(&config).is_ok());

        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "c_collector_1", "150", "2"; extension => "bin"), // "${TEMPDIR}/p1/c_collector_1/150/2.bin"
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        page_creator
            .copy_page_template(&page.path, &config)
            .unwrap();
        let record1 = PageRecord {
            id: String::from("p1.c_collector_1.150.2"),
            nan_filled: false,
            complete: true,
            size: 150,
            last_used: now_utc().to_timespec() - Duration::weeks(20),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("record"),
            nan_filled: false,
            complete: true,
            size: 50,
            last_used: now_utc().to_timespec() - Duration::weeks(10),
        };
        db.upsert_page(&record2).unwrap();

        CachePageCollector::with_props(Props { config, db });

        assert_eq!(CachePageCollector.soft_recycle().unwrap(), 50);
    }

    #[test]
    fn soft_recycle_no_deletes() {
        let config = Config::new(
            &*TEMP_DIR, // base_path
            0,          // page_size
            500,        // soft_cache_size
            0,          // hard_cache_size
        );
        assert!(cache::create_page_template(&config).is_ok());

        let db = util::database::temp().unwrap();
        let record1 = PageRecord {
            id: String::from("record-1"),
            nan_filled: false,
            complete: true,
            size: 150,
            last_used: now_utc().to_timespec() - Duration::weeks(20),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("record-2"),
            nan_filled: false,
            complete: true,
            size: 50,
            last_used: now_utc().to_timespec() - Duration::weeks(10),
        };
        db.upsert_page(&record2).unwrap();

        CachePageCollector::with_props(Props { config, db });

        assert_eq!(CachePageCollector.soft_recycle().unwrap(), 200);
    }

    #[test]
    fn hard_recycle_with_deletes() {
        let config = Config::new(
            &*TEMP_DIR, // base_path
            150,        // page_size
            0,          // soft_cache_size
            100,        // hard_cache_size
        );
        assert!(cache::create_page_template(&config).is_ok());

        let page_creator = PageCreator::new();
        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "c_collector_2", "150", "2"; extension => "bin"), // "${TEMPDIR}/p1/c_collector_2/150/2.bin"
            start: 0,
            end: 0,
            size: 150,
            id: 2,
        };
        page_creator
            .copy_page_template(&page.path, &config)
            .unwrap();
        let db = util::database::temp().unwrap();
        let record1 = PageRecord {
            id: String::from("p1.c_collector_2.150.2"),
            nan_filled: false,
            complete: true,
            size: 150,
            last_used: now_utc().to_timespec() - Duration::days(20),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("record"),
            nan_filled: false,
            complete: true,
            size: 50,
            last_used: now_utc().to_timespec() - Duration::hours(18),
        };
        db.upsert_page(&record2).unwrap();

        CachePageCollector::with_props(Props { config, db });

        assert_eq!(CachePageCollector.hard_recycle().unwrap(), 50);
    }

    #[test]
    fn hard_recycle_no_deletes() {
        let config = Config::new(
            &*TEMP_DIR, // base_path
            0,          // page_size
            0,          // soft_cache_size
            500,        // hard_cache_size
        );
        assert!(cache::create_page_template(&config).is_ok());

        let db = util::database::temp().unwrap();
        let record1 = PageRecord {
            id: String::from("record-1"),
            nan_filled: false,
            complete: true,
            size: 150,
            last_used: now_utc().to_timespec() - Duration::weeks(20),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("record-2"),
            nan_filled: false,
            complete: true,
            size: 50,
            last_used: now_utc().to_timespec() - Duration::weeks(10),
        };
        db.upsert_page(&record2).unwrap();

        CachePageCollector::with_props(Props { config, db });

        assert_eq!(CachePageCollector.hard_recycle().unwrap(), 200);
    }

    #[test]
    fn hard_recycle_space_err() {
        let config = Config::new(
            &*TEMP_DIR, // base_path
            0,          // page_size
            0,          // soft_cache_size
            10,         // hard_cache_size
        );
        assert!(cache::create_page_template(&config).is_ok());

        let db = util::database::temp().unwrap();
        let record1 = PageRecord {
            id: String::from("record-remove"),
            nan_filled: false,
            complete: true,
            size: 150,
            last_used: now_utc().to_timespec() - Duration::hours(10),
        };
        db.upsert_page(&record1).unwrap();
        let record2 = PageRecord {
            id: String::from("record"),
            nan_filled: false,
            complete: true,
            size: 50,
            last_used: now_utc().to_timespec() - Duration::hours(6),
        };
        db.upsert_page(&record2).unwrap();

        CachePageCollector::with_props(Props { config, db });

        assert!(CachePageCollector.hard_recycle().is_err());
    }
}
