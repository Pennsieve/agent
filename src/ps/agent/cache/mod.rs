//! Interface for reading and writing cache pages on the local filesystem.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64;
use std::io::prelude::*;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::vec::IntoIter;
use std::{cmp, fs, io};

use byteorder::{ByteOrder, NativeEndian};
use log::*;
use protobuf::repeated::RepeatedField;
use protobuf::Message;

use crate::ps::agent::database;
use crate::ps::proto;
use crate::ps::proto::timeseries::{AgentTimeSeriesResponse, ChunkResponse, Segment};

mod collector;
mod error;

pub use self::collector::{CachePageCollector, Props};
pub use self::error::{Error, ErrorKind, Result};
pub use crate::ps::agent::config::CacheConfig as Config;

/// Number of bits in a byte.
const BYTE_WIDTH: usize = 8;

/// Converts hz to microseconds.
fn hz_to_us(hz: f64) -> f64 {
    1e6 / hz
}

/// Normalizes the given string to make it safe to use as a directory
/// for the underlying operating system. The `:` character is not allowed on
/// Windows, but appears on Pennsieve node ids, e.g. "N:user:..."
fn normalize_path(p: &str) -> String {
    if cfg!(windows) {
        p.replace(":", "_")
    } else {
        p.to_owned()
    }
}

// Given two identifiers, checks for post-normalization equality.
fn normalize_equals(p: &str, q: &str) -> bool {
    normalize_path(p) == normalize_path(q)
}

/// Given a period, in microseconds, and a page size, returns the length of
/// one page, in microseconds.
fn page_window(period: f64, page_size: u32) -> u64 {
    (f64::from(page_size) * period).floor() as u64
}

/// Composes a unique page key.
fn page_key(package_id: &str, channel_id: &str, page_size: u32, index: u64) -> String {
    format!(
        "{}.{}.{}.{}",
        normalize_path(package_id),
        normalize_path(channel_id),
        page_size,
        index
    )
}

/// Takes a page key and returns the parts that were originally used
/// to build it.
fn from_page_key(key: &str) -> (String, String, u32, u64) {
    let parts: Vec<&str> = key.split('.').collect();
    let package_id = parts[0].to_string();
    let channel_id = parts[1].to_string();
    let page_size = parts[2].parse::<u32>().unwrap();
    let index = parts[3].parse::<u64>().unwrap();

    (package_id, channel_id, page_size, index)
}

/// Finds the start time, in microseconds, of the page that time `t`
/// falls on.
fn get_start(t: u64, period: f64, page_size: u32) -> u64 {
    let window = page_window(period, page_size) as f64;
    let start = t as f64 / window;
    start.floor() as u64
}

/// Finds the end time, in microseconds, of the page that time `t`
/// falls on.
fn get_end(t: u64, period: f64, page_size: u32) -> u64 {
    let window = page_window(period, page_size) as f64;
    let end = t as f64 / window;
    end.ceil() as u64
}

/// Given a desired start and page start, returns an offset that represents
/// the position of the underlying page you should seek to. This offset represents
/// the number of data points, not the number of bytes.
fn get_offset(start: u64, page_start: u64, period: f64) -> usize {
    let offset = (start - page_start) as f64 / period;
    offset.floor() as usize
}

/// Creates a template file for the given page size. The file will be
/// NaN filled.
pub fn create_page_template(config: &Config) -> io::Result<()> {
    let path = config.get_template_path();

    if !path.exists() {
        info!("Creating page template at path {:?}", path);

        path.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not get parent from path"))
            .and_then(fs::create_dir_all)?;

        let file = fs::File::create(&path)?;
        let mut writer = io::BufWriter::new(&file);

        let mut buf: [u8; BYTE_WIDTH] = [0; BYTE_WIDTH];
        NativeEndian::write_f64(&mut buf, f64::NAN);

        for _ in 0..config.page_size() {
            writer.write_all(&buf)?;
        }

        writer.flush()?;
    }

    Ok(())
}

/// Represents a timeseries channel. Rate is in hz.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    _id: String,
    normalized_id: String,
    rate: f64,
}

impl Channel {
    pub fn new<P>(id: P, rate: f64) -> Self
    where
        P: Into<String>,
    {
        let id = id.into();
        Self {
            normalized_id: normalize_path(&id),
            _id: id,
            rate,
        }
    }

    pub fn id(&self) -> &String {
        &self.normalized_id
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }

    pub fn period(&self) -> f64 {
        hz_to_us(self.rate)
    }
}

/// Represents a timeseries request.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    _package_id: String,
    normalized_package_id: String,
    channels: Vec<Channel>,
    start: u64,
    end: u64,
    chunk_size: u32,
    use_cache: bool,
}

impl Request {
    pub fn new<P>(
        package_id: P,
        channels: Vec<Channel>,
        start: u64,
        end: u64,
        chunk_size: u32,
        use_cache: bool,
    ) -> Self
    where
        P: Into<String>,
    {
        let package_id = package_id.into();
        Self {
            normalized_package_id: normalize_path(&package_id),
            _package_id: package_id,
            channels,
            start,
            end,
            chunk_size,
            use_cache,
        }
    }

    pub fn package_id(&self) -> &String {
        &self.normalized_package_id
    }

    pub fn channels(&self) -> &Vec<Channel> {
        &self.channels
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }

    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    pub fn use_cache(&self) -> bool {
        self.use_cache
    }

    /// Returns a range that encompasses all of the pages that are within
    /// the bounds the the request.
    fn get_page_range(&self, period: f64, page_size: u32) -> Range<u64> {
        let page_start = get_start(self.start, period, page_size);
        let page_end = get_end(self.end, period, page_size);

        (page_start..page_end)
    }

    /// Returns a timeseries response.
    pub fn get_response(&self, config: &Config) -> Response {
        let mut pages = BTreeMap::new();
        let mut page_range = BTreeMap::new();

        // every channel can have a different period
        for channel in &self.channels {
            let period = channel.period();
            let range = self.get_page_range(period, config.page_size());
            let page_window = page_window(period, config.page_size());

            info!("Request for {} over page range {:?}", channel.id(), range);
            page_range.insert(channel.id().clone(), range.clone());

            for id in range {
                let key = page_key(self.package_id(), channel.id(), config.page_size(), id);
                let page_start = id as u64 * page_window;
                let page_end =
                    page_start + (period * f64::from(config.page_size() - 1) as f64).floor() as u64;
                pages.insert(
                    key,
                    Page::new(
                        &config,
                        self.package_id(),
                        channel.id(),
                        page_start,
                        page_end,
                        id,
                    ),
                );
            }
        }

        Response::new(self, config, pages, page_range)
    }
}

/// Encapsulates the critical section, for soft cleanups, that cannot be
/// interleaved with other cache logic.
pub fn soft_cleanup(
    collector: &CachePageCollector,
    db: &database::Database,
    soft_cache_size: u64,
    current_size: &mut i64,
) -> Result<i64> {
    let mut recycled = 0;

    if *current_size as u64 > soft_cache_size {
        for page in db.get_soft_aged_pages()? {
            if *current_size as u64 > soft_cache_size {
                recycled += 1;
                *current_size -= page.size;
                collector.remove_page(&page)?;
            } else {
                break;
            }
        }
    }

    Ok(recycled)
}

/// Encapsulates the critical section, for hard cleanups, that cannot be
/// interleaved with other cache logic.
pub fn hard_cleanup(
    collector: &CachePageCollector,
    db: &database::Database,
    hard_cache_size: u64,
    current_size: &mut i64,
) -> Result<i64> {
    let mut recycled = 0;

    if *current_size as u64 > hard_cache_size {
        for page in db.get_hard_aged_pages()? {
            if *current_size as u64 > hard_cache_size {
                recycled += 1;
                *current_size -= page.size;
                collector.remove_page(&page)?;
            } else {
                break;
            }
        }
    }

    Ok(recycled)
}

/// Encapsulates the critical section, for fetching requests, that cannot
/// be interleaved with other cache logic.
fn get_uncached_pages(
    response: &mut Response,
    db: &database::Database,
) -> Result<Vec<PageRequest>> {
    let mut requests = Vec::new();

    for channel in &response.channels {
        let window = page_window(channel.period(), response.config.page_size());
        let range = response
            .page_range
            .get_mut(channel.id())
            .expect("channel id was not in page range map");

        // seed max completed for each channel to 0
        response.max_completed.insert(channel.id().clone(), 0);

        for page_id in range {
            let key = page_key(
                &response.package_id,
                &channel.id(),
                response.config.page_size(),
                page_id,
            );
            db.touch_last_used(&key)?;

            let page_start = page_id as u64 * window;
            let page_end =
                page_start as f64 + channel.period() * f64::from(response.config.page_size());

            if !response.use_cache || !db.is_page_cached(&key)? {
                response.page_requests.push(key);
                requests.push(PageRequest {
                    channel_id: channel.id().clone(),
                    start: page_start,
                    end: page_end as u64,
                });
            }
        }
    }

    info!(
        "Uncached requests required: {}",
        response.page_requests.len()
    );
    debug!(
        "Requests required for the following keys: {:#?}",
        response.page_requests
    );

    Ok(requests)
}

/// Utility functions for seeding cache pages with the template files.
struct PageCreatorInner;

impl PageCreatorInner {
    /// Copies a blank page into the location on the local filesystem that
    /// backs this cache page.
    pub fn copy_page_template(&self, path: &PathBuf, config: &Config) -> Result<u64> {
        // double check existence!
        if path.exists() {
            return Ok(0);
        }

        let template_path = config.get_template_path();

        if template_path.exists() {
            path.parent()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("cache:PageCreator:copy_page_template :: couldn't get template parent: {:?}", template_path),
                    )
                })
                .and_then(fs::create_dir_all)
                .and_then(|_| fs::copy(&template_path, &path))
                .map_err(Into::into)
        } else {
            Err(Error::invalid_page(template_path))
        }
    }
}

/// Wrapper around the private `PageCreatorInner` implementation. This wrapper
/// adds a reference counted mutex, this allows the underlying template creator
/// to copy templates in a safe way.
#[derive(Clone)]
pub struct PageCreator {
    inner: Arc<Mutex<PageCreatorInner>>,
}

impl Default for PageCreator {
    fn default() -> Self {
        Self::new()
    }
}

impl PageCreator {
    /// Creates a new page creator.
    pub fn new() -> Self {
        PageCreator {
            inner: Arc::new(Mutex::new(PageCreatorInner)),
        }
    }

    /// Unlocks the mutex before seeding the cache page from the template.
    fn copy_page_template(&self, path: &PathBuf, config: &Config) -> Result<u64> {
        let inner = self.inner.lock().unwrap();

        inner.copy_page_template(path, config)
    }
}

/// Represents a page request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageRequest {
    channel_id: String,
    start: u64,
    end: u64,
}

impl PageRequest {
    pub fn new<P>(channel_id: P, start: u64, end: u64) -> Self
    where
        P: Into<String>,
    {
        Self {
            channel_id: channel_id.into(),
            start,
            end,
        }
    }

    pub fn channel_id(&self) -> &String {
        &self.channel_id
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }
}

/// Represents a cache page that is backed by the local filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Page {
    start: u64,
    end: u64,
    path: PathBuf,
    size: u32,
    id: u64,
}

impl Page {
    /// Creates a new cache page.
    fn new(
        config: &Config,
        package_id: &str,
        channel_id: &str,
        start: u64,
        end: u64,
        id: u64,
    ) -> Page {
        let mut path = config.base_path().to_path_buf();
        let package_id = normalize_path(package_id);
        let channel_id = normalize_path(channel_id);
        let size = config.page_size();

        path.push(package_id);
        path.push(channel_id);
        path.push(size.to_string());
        path.push(id.to_string());
        path.set_extension("bin");

        Page {
            path,
            start,
            end,
            size,
            id,
        }
    }

    /// Returns the offset from the start of this page to the requested start.
    /// An error is returned if the requested start falls after the page end.
    fn get_offset(&self, window_start: u64, period: f64) -> Result<usize> {
        if window_start < self.start {
            Ok(0)
        } else if window_start > self.end {
            Err(io::Error::new(io::ErrorKind::Other, "would seek outside of file range").into())
        } else {
            Ok(get_offset(window_start, self.start, period))
        }
    }

    /// Writes the data to the cached page with the requested offset.
    fn write(
        &self,
        page_creator: &PageCreator,
        config: &Config,
        offset: usize,
        data: &[f64],
    ) -> Result<()> {
        if !self.path.exists() {
            page_creator.copy_page_template(&self.path, config)?;
        }

        let file = fs::OpenOptions::new().write(true).open(&self.path)?;
        let mut writer = io::BufWriter::new(&file);

        if offset + data.len() > self.size as usize {
            return Err(
                io::Error::new(io::ErrorKind::Other, "would write outside of file range").into(),
            );
        }

        if offset > 0 {
            writer.seek(io::SeekFrom::Start(offset as u64 * BYTE_WIDTH as u64))?;
        }

        for &d in data {
            let mut buf: [u8; BYTE_WIDTH] = [0; BYTE_WIDTH];
            NativeEndian::write_f64(&mut buf, d);
            writer.write_all(&buf)?;
        }

        writer.flush().map_err(Into::into)
    }

    /// Reads from the cached page. The length of the data array determines
    /// the amount of data points read. The position of the start of the reaad
    /// is determined by the offset.
    fn read(&self, offset: usize, data: &mut [f64]) -> Result<()> {
        let file = fs::File::open(&self.path)?;
        let mut reader = io::BufReader::new(&file);

        if offset + data.len() > self.size as usize {
            return Err(
                io::Error::new(io::ErrorKind::Other, "would write outside of file range").into(),
            );
        }

        if offset > 0 {
            reader.seek(io::SeekFrom::Start(offset as u64 * BYTE_WIDTH as u64))?;
        }

        for d in data {
            let mut buf: [u8; BYTE_WIDTH] = [0; BYTE_WIDTH];
            reader.read_exact(&mut buf)?;
            *d = NativeEndian::read_f64(&buf);
        }

        Ok(())
    }

    /// Deletes the cached page on the local filesystem.
    fn delete(&self) -> Result<()> {
        fs::remove_file(&self.path).map_err(Into::into)
    }
}

/// Creates a new timeseries response. This response is capable of caching segments
/// returned from Pennsieve and holds the contents from the original request in
/// order to be able to return chunks through an iterator.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    config: Config,
    pub pages: BTreeMap<String, Page>,
    page_range: BTreeMap<String, Range<u64>>,
    package_id: String,
    channels: Vec<Channel>,
    start: u64,
    end: u64,
    chunk_size: u32,
    use_cache: bool,
    page_requests: Vec<String>,
    nan_pages: HashSet<String>,
    max_completed: HashMap<String, u64>,
}

impl Response {
    /// Creates a new timeseries response.
    fn new(
        request: &Request,
        config: &Config,
        pages: BTreeMap<String, Page>,
        page_range: BTreeMap<String, Range<u64>>,
    ) -> Response {
        Response {
            config: config.clone(),
            pages,
            page_range,
            package_id: request.package_id().clone(),
            channels: request.channels.clone(),
            start: request.start,
            end: request.end,
            chunk_size: request.chunk_size,
            use_cache: request.use_cache,
            page_requests: Vec::new(),
            nan_pages: HashSet::new(),
            max_completed: HashMap::new(),
        }
    }

    /// Returns a reference to a cached page associated with the provided
    /// page key.
    fn get_page(&self, key: &str) -> Result<&Page> {
        self.pages
            .get(key)
            .ok_or_else(|| Error::invalid_page(key.to_string()))
    }

    /// Returns an iterator of page requests. These page requests represent
    /// pages that are not already fully cached on the local filesystem.
    pub fn uncached_page_requests(
        &mut self,
        db: &database::Database,
    ) -> Result<IntoIter<PageRequest>> {
        self.page_requests.clear();
        self.nan_pages.clear();
        self.max_completed.clear();

        let requests = get_uncached_pages(self, &db)?;

        Ok(requests.into_iter())
    }

    /// Writes a page record to the db for every page request. This method is meant
    /// to be called only after all segments are cached, as it is writing from
    /// lookup maps that it accumulated during the segment caching process.
    pub fn record_page_requests(&self, db: &database::Database) -> Result<()> {
        for req in &self.page_requests {
            let key = req.to_string();
            let (_, channel_id, _, page_id) = from_page_key(&key);
            let completed: Result<&u64> = self
                .max_completed
                .get(&channel_id)
                .ok_or_else(|| Error::invalid_channel(channel_id));
            let completed: &u64 = completed?;
            let completed = *completed > page_id;

            if self.nan_pages.contains(&key) {
                db.write_nan_filled(&key, completed)?;
            } else {
                let page = database::PageRecord::new(
                    key,
                    false,
                    completed,
                    i64::from(self.config.page_size()),
                );
                db.upsert_page(&page)?;
            }
        }

        Ok(())
    }

    /// Caches the provided segment of data to the cache. If the segment is empty,
    /// a NaN page is written.
    pub fn cache_response(&mut self, page_creator: &PageCreator, segment: &Segment) -> Result<()> {
        if segment.data.is_empty() {
            for c in &self.channels {
                let channel_id = c.id().clone();
                if normalize_equals(&channel_id, &segment.source) {
                    let index = get_start(segment.startTs, c.period(), self.config.page_size());
                    let key = page_key(
                        &self.package_id,
                        &segment.source,
                        self.config.page_size(),
                        index,
                    );
                    self.nan_pages.insert(key);
                }
            }

            Ok(())
        } else {
            let mut data_pos = 0;
            let mut index = get_start(
                segment.startTs,
                segment.samplePeriod,
                self.config.page_size(),
            );

            // Normalize the segment's source ID before comparison
            // and indexing operations:
            let segment_source_id = normalize_path(&segment.source);

            while data_pos < segment.data.len() {
                let page_id;

                {
                    let key = page_key(
                        &self.package_id,
                        &segment_source_id,
                        self.config.page_size(),
                        index,
                    );
                    let page = self.get_page(&key)?;
                    page_id = page.id;
                    let offset = page.get_offset(segment.startTs, segment.samplePeriod)?;
                    let len = cmp::min(segment.data.len() - data_pos, page.size as usize - offset);

                    page.write(
                        &page_creator,
                        &self.config,
                        offset,
                        &segment.data[data_pos..(data_pos + len)],
                    )?;

                    data_pos += len;
                    index += 1;
                }

                // when we are in this arm, the segment has datapoints in it. Fetch the
                // current max completed page for this channel and increment it if the current
                // page is greater than the value that already exists.
                {
                    let max_completed: Result<&mut u64> = self
                        .max_completed
                        .get_mut(&segment_source_id)
                        .ok_or_else(|| Error::invalid_channel(segment_source_id.clone()));
                    let max_completed: &mut u64 = max_completed?;
                    *max_completed = cmp::max(*max_completed, page_id);
                }
            }

            Ok(())
        }
    }

    /// Returns an iterator that represents each chunk defined in the original
    /// request.
    pub fn owned_chunk_response_iter(self, db: database::Database) -> ChunkResponseIterator {
        let mut pos = HashMap::new();

        for channel in &self.channels {
            pos.insert(channel.id().clone(), self.start);
        }

        ChunkResponseIterator {
            response: self,
            db,
            pos,
        }
    }
}

/// Iterator that represents each chunk defined in the original request.
#[derive(Debug)]
pub struct ChunkResponseIterator {
    response: Response,
    db: database::Database,
    pos: HashMap<String, u64>,
}

impl ChunkResponseIterator {
    /// Returns one `ChunkResponse`. This is defined by the protobuf interface.
    fn get_chunk(&mut self) -> Result<ChunkResponse> {
        let mut chunk = ChunkResponse::new();
        let channels = Vec::with_capacity(self.response.channels.len());
        chunk.set_channels(RepeatedField::from_vec(channels));

        for channel in &self.response.channels {
            let channel_pos: Result<&mut u64> = self
                .pos
                .get_mut(channel.id())
                .ok_or_else(|| Error::invalid_channel(channel.id().clone()));
            let channel_pos = channel_pos?;
            let mut start_pos = *channel_pos;
            let mut chunk_pos_index = 0;
            let mut chunk_pos = 0;
            let chunk_size = self.response.chunk_size / channel.period() as u32;
            let mut data = vec![0f64; chunk_size as usize];
            let mut index = get_start(
                *channel_pos,
                channel.period(),
                self.response.config.page_size(),
            );

            while chunk_pos < self.response.chunk_size.into() && *channel_pos < self.response.end {
                let key = page_key(
                    &self.response.package_id,
                    &channel.id(),
                    self.response.config.page_size(),
                    index,
                );
                let page = self.response.get_page(&key)?;
                let offset = page.get_offset(*channel_pos, channel.period())?;
                let len = cmp::min(chunk_size - chunk_pos_index, page.size - offset as u32);

                let data_slice = data.as_mut_slice();

                if self.db.is_page_nan(&key)? {
                    for d in &mut data_slice
                        [chunk_pos_index as usize..(chunk_pos_index as usize + len as usize)]
                    {
                        *d = f64::NAN;
                    }
                } else {
                    page.read(
                        offset,
                        &mut data_slice
                            [chunk_pos_index as usize..(chunk_pos_index as usize + len as usize)],
                    )?;
                }

                chunk_pos_index += len;
                chunk_pos += u64::from(len) * channel.period() as u64;
                *channel_pos += u64::from(len) * channel.period() as u64;
                index += 1;
            }

            data.truncate(chunk_pos_index as usize);

            let mut points = Vec::with_capacity(data.len());
            for &d in &data {
                if !d.is_nan() {
                    points.push(proto::create_datum(start_pos, d));
                }

                start_pos += channel.period() as u64;
            }

            if !data.is_empty() {
                chunk
                    .channels
                    .push(proto::create_channel_chunk(channel.id().clone(), points));
            }
        }

        Ok(chunk)
    }
}

impl<'a> Iterator for ChunkResponseIterator {
    type Item = Result<Vec<u8>>;

    /// Gets each chunk and converts the protobuf representation to a
    /// `Vec` of bytes.
    fn next(&mut self) -> Option<Self::Item> {
        match self.get_chunk() {
            Ok(chunk) => {
                if chunk.channels.is_empty() {
                    None
                } else {
                    let mut response = AgentTimeSeriesResponse::new();
                    response.set_chunk(chunk);

                    Some(response.write_to_bytes().map_err(Into::into))
                }
            }
            Err(e) => {
                error!("{:?}", e);
                Some(Err(e))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use lazy_static::lazy_static;
    use protobuf;
    use tempfile::tempdir;

    use std::path;

    use pennsieve_macros::path;

    use super::*;
    use crate::ps::util;

    lazy_static! {
        static ref TEMP_DIR: path::PathBuf = tempdir().unwrap().into_path();
    }

    fn eq_with_nan_eq(a: f64, b: f64) -> bool {
        (a.is_nan() && b.is_nan()) || (a == b)
    }

    fn vec_compare(va: &[f64], vb: &[f64]) -> bool {
        (va.len() == vb.len()) && va.iter().zip(vb).all(|(a, b)| eq_with_nan_eq(*a, *b))
    }

    fn helper_convert_chunk(bytes: &Vec<u8>) -> ChunkResponse {
        let mut response: AgentTimeSeriesResponse = protobuf::parse_from_bytes(bytes).unwrap();
        response.take_chunk()
    }

    fn helper_create_config(page_size: u32) -> Config {
        Config::new(
            &*TEMP_DIR, // base_path
            page_size,  // page_size
            0,          // soft_cache_size
            0,          // hard_cache_size
        )
    }

    #[test]
    fn test_create_page_template() {
        let config = helper_create_config(300);
        assert!(create_page_template(&config).is_ok());

        let path = config.get_template_path();
        let metadata = fs::metadata(&path).unwrap();

        assert_eq!(metadata.len(), 300 * BYTE_WIDTH as u64);
    }

    #[test]
    fn window_page_range_global_start() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",
            vec![c.clone()],
            0,     // start
            10,    // end
            0,     // chunk_size
            false, // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 100), (0..1));
    }

    #[test]
    fn window_page_range_span_simple() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",            // package_id
            vec![c.clone()], // channels
            24,              // start
            55,              // end
            0,               // chunk_size
            false,           // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 10), (2..6));
    }

    #[test]
    fn window_page_range_span_edge_right_1() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",            // package_id
            vec![c.clone()], // channels
            5,               // start
            20,              // end
            0,               // chunk_size
            false,           // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 10), (0..2));
    }

    #[test]
    fn window_page_range_span_edge_right_2() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",            // package_id
            vec![c.clone()], // channels
            5,               // start
            21,              // end
            0,               // chunk_size
            false,           // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 10), (0..3));
    }

    #[test]
    fn window_page_range_span_edge_left_1() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",            // package_id
            vec![c.clone()], // channels
            9,               // start
            25,              // end
            0,               // chunk_size
            false,           // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 10), (0..3));
    }

    #[test]
    fn window_page_range_span_edge_left_2() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",            // package_id
            vec![c.clone()], // channels
            10,              // start
            25,              // end
            0,               // chunk_size
            false,           // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 10), (1..3));
    }

    #[test]
    fn window_page_range_span_long() {
        let c = Channel::new("c1", 1e6);
        let r = Request::new(
            "p1",            // package_id
            vec![c.clone()], // channels
            500,             // start
            21001,           // end
            0,               // chunk_size
            false,           // use_cache
        );
        assert_eq!(r.get_page_range(c.period(), 100), (5..211));
    }

    #[test]
    fn test_page_key_parsing() {
        let key = page_key(&String::from("p1"), &String::from("c1"), 100, 200);
        let (package, channel, size, index) = from_page_key(&key);

        assert_eq!(package, String::from("p1"));
        assert_eq!(channel, String::from("c1"));
        assert_eq!(size, 100);
        assert_eq!(index, 200);
    }

    #[test]
    fn window_pages() {
        let config = helper_create_config(10);
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p:1", // package_id
            vec![
                // channels
                Channel::new("c:1", 1e6),
                Channel::new("c:2", 1e6),
            ],
            10,    // start
            29,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let pages = vec![
            Page {
                path: path!(&*TEMP_DIR, "p:1", "c:1", "10", "1"; extension => "bin"),
                start: 10,
                end: 19,
                size: 10,
                id: 1,
            },
            Page {
                path: path!(&*TEMP_DIR, "p:1", "c:1", "10", "2"; extension => "bin"),
                start: 20,
                end: 29,
                size: 10,
                id: 2,
            },
            Page {
                path: path!(&*TEMP_DIR, "p:1", "c:2", "10", "1"; extension => "bin"),
                start: 10,
                end: 19,
                size: 10,
                id: 1,
            },
            Page {
                path: path!(&*TEMP_DIR, "p:1", "c:2", "10", "2"; extension => "bin"),
                start: 20,
                end: 29,
                size: 10,
                id: 2,
            },
        ];

        let response_pages = request.get_response(&config).pages;

        let key = page_key(&String::from("p:1"), &String::from("c:1"), 10, 1);
        assert_eq!(response_pages.get(&key), Some(&pages[0]));
        let key = page_key(&String::from("p:1"), &String::from("c:1"), 10, 2);
        assert_eq!(response_pages.get(&key), Some(&pages[1]));
        let key = page_key(&String::from("p:1"), &String::from("c:2"), 10, 1);
        assert_eq!(response_pages.get(&key), Some(&pages[2]));
        let key = page_key(&String::from("p:1"), &String::from("c:2"), 10, 2);
        assert_eq!(response_pages.get(&key), Some(&pages[3]));
    }

    #[test]
    fn page_new() {
        let config = helper_create_config(10);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 0, 101);
        let path = path!(&*TEMP_DIR, "p1", "c1", "10", "101"; extension => "bin");
        assert_eq!(page.path, path);
        assert_eq!(page.start, 0);
        assert_eq!(page.end, 0);
    }

    #[test]
    fn page_create() {
        let config = helper_create_config(100);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 0, 10);
        let page_creator = PageCreator::new();

        assert!(page.write(&page_creator, &config, 0, &[0f64]).is_ok());
        let metadata = fs::metadata(&page.path);
        assert_eq!(metadata.unwrap().len(), 800);
    }

    #[test]
    fn page_delete() {
        let config = helper_create_config(100);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c12345");
        let page = Page::new(&config, &package, &channel, 0, 0, 10);
        let page_creator = PageCreator::new();

        assert!(page.write(&page_creator, &config, 0, &[0f64]).is_ok());
        let metadata = fs::metadata(&page.path);
        assert_eq!(metadata.unwrap().len(), 800);

        assert!(page.path.exists());
        assert!(page.delete().is_ok());
        assert!(!page.path.exists());
    }

    #[test]
    fn page_offset_simple() {
        let config = helper_create_config(10);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 9, 1);

        assert_eq!(page.get_offset(5, 1f64).unwrap(), 5);
    }

    #[test]
    fn page_offset_simple_edge_cases() {
        let config = helper_create_config(10);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 10, 19, 1);

        assert_eq!(page.get_offset(9, 1f64).unwrap(), 0);
        assert_eq!(page.get_offset(10, 1f64).unwrap(), 0);
        assert_eq!(page.get_offset(19, 1f64).unwrap(), 9);
        assert!(page.get_offset(20, 1f64).is_err());
    }

    #[test]
    fn page_read_write_simple() {
        let config = helper_create_config(5);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 4, 1);
        let page_creator = PageCreator::new();

        let output = [0.1, 1.0, 0.9, 9.0, 0.5];
        page.write(&page_creator, &config, 0, &output).unwrap();
        let mut input: [f64; 5] = [0f64; 5];
        page.read(0, &mut input).unwrap();

        assert_eq!(input, output);
    }

    #[test]
    fn page_read_write_offset_simple() {
        let config = helper_create_config(5);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 4, 1);
        let page_creator = PageCreator::new();

        let output = [1.0, 0.9, 9.0];
        page.write(&page_creator, &config, 1, &output).unwrap();
        let mut input: [f64; 2] = [0f64; 2];
        page.read(2, &mut input).unwrap();

        assert_eq!(input, [0.9, 9.0]);
    }

    #[test]
    fn page_write_file_range() {
        let config = helper_create_config(5);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 4, 1);
        let page_creator = PageCreator::new();

        let output = [0.9, 9.0, 0.5];

        assert!(page.write(&page_creator, &config, 2, &output).is_ok());
        assert!(page.write(&page_creator, &config, 3, &output).is_err());
    }

    #[test]
    fn page_read_file_range() {
        let config = helper_create_config(5);
        assert!(create_page_template(&config).is_ok());

        let package = String::from("p1");
        let channel = String::from("c1");
        let page = Page::new(&config, &package, &channel, 0, 4, 1);

        let page_creator = PageCreator::new();
        page_creator
            .copy_page_template(&page.path, &config)
            .unwrap();

        let mut input: [f64; 3] = [0f64; 3];

        assert!(page.read(2, &mut input).is_ok());
        assert!(page.read(3, &mut input).is_err());
    }

    #[test]
    fn record_page_requests_cache_mix() {
        let config = helper_create_config(10);
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());
        let request = Request::new(
            "p1", // package_id
            vec![
                // channels
                Channel::new("c11", 1e6),
                Channel::new("c12", 1e6),
            ],
            10,   // start
            29,   // end
            0,    // chunk_size
            true, // use_cache
        );
        let mut pages = Vec::new();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c11", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/c11/10/1.bin"
            start: 10,
            end: 19,
            size: 10,
            id: 1,
        });
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            1,
        );
        db.write_nan_filled(&key, true).unwrap();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c11", "10", "2"; extension => "bin"), // "${TEMPDIR}/p1/c11/10/2.bin"
            start: 20,
            end: 29,
            size: 10,
            id: 2,
        });
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c12", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/c12/10/1.bin"
            start: 10,
            end: 19,
            size: 10,
            id: 1,
        });
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c12", "10", "2"; extension => "bin"), // "${TEMPDIR}/p1/c12/10/2.bin"
            start: 20,
            end: 29,
            size: 10,
            id: 2,
        });

        let mut response = request.get_response(&config);
        let pages: Vec<PageRequest> = response.uncached_page_requests(&db).unwrap().collect();

        assert_eq!(pages.len(), 3);
    }

    #[test]
    fn response_uncached_iter_use_cache() {
        let config = helper_create_config(10);
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());
        let request = Request::new(
            String::from("p1"), // package_id
            vec![
                // channels
                Channel::new("c1", 1e6),
                Channel::new("c2", 1e6),
            ],
            10,   // start
            29,   // end
            0,    // chunk_size
            true, // use_cache
        );
        let mut pages = Vec::new();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c1", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/c1/10/1.bin"
            start: 10,
            end: 19,
            size: 10,
            id: 1,
        });
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c1", "10", "2"; extension => "bin"), // "${TEMPDIR}/p1/c1/10/2.bin"
            start: 20,
            end: 29,
            size: 10,
            id: 2,
        });
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            2,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c2", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/c2/10/1.bin"
            start: 10,
            end: 19,
            size: 10,
            id: 1,
        });
        let key = page_key(
            request.package_id(),
            request.channels[1].id(),
            config.page_size(),
            1,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c2", "10", "2"; extension => "bin"), // "${TEMPDIR}/p1/c2/10/2.bin"
            start: 20,
            end: 29,
            size: 10,
            id: 2,
        });

        let mut response = request.get_response(&config);
        let pages: Vec<PageRequest> = response.uncached_page_requests(&db).unwrap().collect();

        assert_eq!(
            pages,
            vec![
                PageRequest::new("c1", 10, 20),
                PageRequest::new("c2", 20, 30),
            ],
        );
    }

    #[test]
    fn response_uncached_iter_use_cache_false() {
        let config = helper_create_config(10);
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());
        let request = Request::new(
            String::from("p1"), // package_id
            vec![
                // channels
                Channel::new("c111", 1e6),
                Channel::new("c112", 1e6),
            ],
            10,    // start
            29,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let mut pages = Vec::new();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c111", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/c111/10/1.bin"
            start: 10,
            end: 19,
            size: 10,
            id: 1,
        });
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c111", "10", "2"; extension => "bin"), // "${TEMPDIR}/p1/c111/10/2.bin"
            start: 20,
            end: 29,
            size: 10,
            id: 2,
        });
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            2,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c112", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/c112/10/1.bin"
            start: 10,
            end: 19,
            size: 10,
            id: 1,
        });
        let key = page_key(
            request.package_id(),
            request.channels[1].id(),
            config.page_size(),
            1,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        pages.push(Page {
            path: path!(&*TEMP_DIR, "p1", "c112", "10", "2"; extension => "bin"), // "${TEMPDIR}/p1/c112/10/2.bin"
            start: 20,
            end: 29,
            size: 10,
            id: 2,
        });

        let mut response = request.get_response(&config);
        let pages: Vec<PageRequest> = response.uncached_page_requests(&db).unwrap().collect();

        assert_eq!(
            pages,
            vec![
                PageRequest::new("c111", 10, 20),
                PageRequest::new("c111", 20, 30),
                PageRequest::new("c112", 10, 20),
                PageRequest::new("c112", 20, 30),
            ],
        );
    }

    #[test]
    fn response_cache_response_empty() {
        let config = helper_create_config(10);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c1", 1e6,
            )],
            10,    // start
            19,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let mut segment = Segment::new();
        segment.set_startTs(10);
        segment.set_source(String::from("cache_c1"));
        segment.set_data(vec![]);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/cache_c1/10/1.bin"
            start: 0,
            end: 0,
            size: 10,
            id: 1,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );

        let mut response = request.get_response(&config);
        response.uncached_page_requests(&db).unwrap();
        response.cache_response(&page_creator, &segment).unwrap();
        response.record_page_requests(&db).unwrap();

        assert!(db.is_page_nan(&key).unwrap());
    }

    #[test]
    fn response_cache_response_exact_page() {
        let config = helper_create_config(10);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c1", 1e6,
            )],
            10,    // start
            19,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let mut segment = Segment::new();
        segment.set_startTs(10);
        segment.set_source(String::from("cache_c1"));
        segment.set_samplePeriod(1f64);
        segment.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/cache_c1/10/1.bin"
            start: 0,
            end: 0,
            size: 10,
            id: 1,
        };

        let mut response = request.get_response(&config);
        response.uncached_page_requests(&db).unwrap();
        response.cache_response(&page_creator, &segment).unwrap();

        let mut input: [f64; 10] = [0f64; 10];

        assert!(page.read(0, &mut input).is_ok());
        assert_eq!(input, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    }

    #[test]
    fn response_cache_response_completed_flag_multiple_channel() {
        let config = helper_create_config(10);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![
                // channels
                Channel::new("cache_c1", 1e6),
                Channel::new("cache_c2", 1e6),
            ],
            10,    // start
            39,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let mut segment = Segment::new();
        segment.set_startTs(10);
        segment.set_source(String::from("cache_c1"));
        segment.set_samplePeriod(1f64);
        segment.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let mut segment2 = Segment::new();
        segment2.set_startTs(20);
        segment2.set_source(String::from("cache_c1"));
        segment2.set_samplePeriod(1f64);
        segment2.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let mut segment3 = Segment::new();
        segment3.set_startTs(30);
        segment3.set_source(String::from("cache_c1"));
        segment3.set_samplePeriod(1f64);
        segment3.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        let mut segment4 = Segment::new();
        segment4.set_startTs(10);
        segment4.set_source(String::from("cache_c2"));
        segment4.set_samplePeriod(1f64);
        segment4.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let mut segment5 = Segment::new();
        segment5.set_startTs(20);
        segment5.set_source(String::from("cache_c2"));
        segment5.set_samplePeriod(1f64);
        segment5.set_data(vec![]);
        let mut segment6 = Segment::new();
        segment6.set_startTs(30);
        segment6.set_source(String::from("cache_c2"));
        segment6.set_samplePeriod(1f64);
        segment6.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        let mut response = request.get_response(&config);
        response.uncached_page_requests(&db).unwrap();
        response.cache_response(&page_creator, &segment).unwrap();
        response.cache_response(&page_creator, &segment2).unwrap();
        response.cache_response(&page_creator, &segment3).unwrap();
        response.cache_response(&page_creator, &segment4).unwrap();
        response.cache_response(&page_creator, &segment5).unwrap();
        response.cache_response(&page_creator, &segment6).unwrap();
        response.record_page_requests(&db).unwrap();

        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            1,
        );
        assert!(db.is_page_cached(&key).unwrap());

        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            2,
        );
        assert!(db.is_page_cached(&key).unwrap());

        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            3,
        );
        assert!(!db.is_page_cached(&key).unwrap());

        let key = page_key(
            request.package_id(),
            request.channels[1].id(),
            config.page_size(),
            1,
        );
        assert!(db.is_page_cached(&key).unwrap());

        let key = page_key(
            request.package_id(),
            request.channels[1].id(),
            config.page_size(),
            2,
        );
        assert!(db.is_page_cached(&key).unwrap());
        assert!(db.is_page_nan(&key).unwrap());

        let key = page_key(
            request.package_id(),
            &request.channels[1].id(),
            config.page_size(),
            3,
        );
        assert!(!db.is_page_cached(&key).unwrap());
    }

    #[test]
    fn response_cache_response_across_pages() {
        let config = helper_create_config(10);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c2", 20f64,
            )],
            1516560423000000, // start
            1516560424000000, // end
            0,                // chunk_size
            false,            // use_cache
        );
        let mut segment = Segment::new();
        segment.set_startTs(1516560423250000);
        segment.set_source(String::from("cache_c2"));
        segment.set_samplePeriod(50000f64);
        segment.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        let mut segment2 = Segment::new();
        segment2.set_startTs(1516560423750000);
        segment2.set_source(String::from("cache_c2"));
        segment2.set_samplePeriod(50000f64);
        segment2.set_data(vec![10.0, 11.0]);

        let mut segment3 = Segment::new();
        segment3.set_startTs(1516560423850000);
        segment3.set_source(String::from("cache_c2"));
        segment3.set_samplePeriod(50000f64);
        segment3.set_data(vec![12.0]);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c2", "10", "3033120846"; extension => "bin"), // "${TEMPDIR}/p1/cache_c2/10/3033120846.bin"
            start: 0,
            end: 0,
            size: 10,
            id: 3033120846,
        };
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c2", "10", "3033120847"; extension => "bin"), // "${TEMPDIR}/p1/cache_c2/10/3033120847.bin"
            start: 0,
            end: 0,
            size: 10,
            id: 3033120847,
        };

        let mut response = request.get_response(&config);
        response.uncached_page_requests(&db).unwrap();
        response.cache_response(&page_creator, &segment).unwrap();
        response.cache_response(&page_creator, &segment2).unwrap();
        response.cache_response(&page_creator, &segment3).unwrap();

        let mut input: [f64; 10] = [0f64; 10];

        assert!(page.read(0, &mut input).is_ok());
        assert!(vec_compare(
            &input,
            &[
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                0.0,
                1.0,
                2.0,
                3.0,
                4.0,
            ]
        ));
        assert!(page2.read(0, &mut input).is_ok());
        assert!(vec_compare(
            &input,
            &[
                5.0,
                6.0,
                7.0,
                8.0,
                9.0,
                10.0,
                11.0,
                12.0,
                f64::NAN,
                f64::NAN,
            ]
        ));
    }

    #[test]
    fn response_cache_response_across_full_pages() {
        let config = helper_create_config(5);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c3", 1e6,
            )],
            10,    // start
            29,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let mut segment = Segment::new();
        segment.set_startTs(10);
        segment.set_source(String::from("cache_c3"));
        segment.set_samplePeriod(1f64);
        segment.set_data(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c3", "5", "2"; extension => "bin"), // "${TEMPDIR}/p1/cache_c3/5/2.bin"
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c3", "5", "3"; extension => "bin"), // "${TEMPDIR}/p1/cache_c3/5/3.bin",
            start: 0,
            end: 0,
            size: 5,
            id: 3,
        };

        let mut response = request.get_response(&config);
        response.uncached_page_requests(&db).unwrap();
        response.cache_response(&page_creator, &segment).unwrap();

        let mut input: [f64; 5] = [0f64; 5];

        assert!(page.read(0, &mut input).is_ok());
        assert_eq!(input, [0.0, 1.0, 2.0, 3.0, 4.0]);
        assert!(page2.read(0, &mut input).is_ok());
        assert_eq!(input, [5.0, 6.0, 7.0, 8.0, 9.0]);
    }

    #[test]
    fn response_cache_response_one_point() {
        let config = helper_create_config(5);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c4", 1e6,
            )],
            10,    // start
            29,    // end
            0,     // chunk_size
            false, // use_cache
        );
        let mut segment = Segment::new();
        segment.set_startTs(11);
        segment.set_source(String::from("cache_c4"));
        segment.set_samplePeriod(1f64);
        segment.set_data(vec![1.0]);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c4", "5", "2"; extension => "bin"), // "${TEMPDIR}/p1/cache_c4/5/2.bin"
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };

        let mut response = request.get_response(&config);
        response.uncached_page_requests(&db).unwrap();
        response.cache_response(&page_creator, &segment).unwrap();

        let mut input: [f64; 5] = [0f64; 5];

        assert!(page.read(0, &mut input).is_ok());
        assert!(vec_compare(
            &input,
            &[f64::NAN, 1.0, f64::NAN, f64::NAN, f64::NAN]
        ));
    }

    #[test]
    fn chunk_response_iterator_exact_page_simple() {
        let config = helper_create_config(10);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            String::from("p1"), // package_id
            vec![Channel::new(
                // channels
                "cache_c1_r1",
                1e6,
            )],
            10,    // start
            19,    // end
            10,    // chunk_size
            false, // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r1", "10", "1"; extension => "bin"), // "${TEMPDIR}/p1/cache_c1_r1/10/1.bin"
            start: 0,
            end: 0,
            size: 10,
            id: 1,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        assert!(page.write(&page_creator, &config, 0, &data).is_ok());

        let mut iter = response.owned_chunk_response_iter(db);

        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c1_r1"),
            vec![
                proto::create_datum(10, 0.0),
                proto::create_datum(11, 1.0),
                proto::create_datum(12, 2.0),
                proto::create_datum(13, 3.0),
                proto::create_datum(14, 4.0),
                proto::create_datum(15, 5.0),
                proto::create_datum(16, 6.0),
                proto::create_datum(17, 7.0),
                proto::create_datum(18, 8.0),
                proto::create_datum(19, 9.0),
            ],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);
        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_response_iterator_across_pages() {
        let config = helper_create_config(5);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c1_r2",
                1e6,
            )],
            10,    // start
            19,    // end
            10,    // chunk_size
            false, // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r2", "5", "2"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r2", "5", "3"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 3,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page2.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        assert!(page.write(&page_creator, &config, 0, &data[0..5]).is_ok());
        assert!(page2.write(&page_creator, &config, 0, &data[5..10]).is_ok());

        let mut iter = response.owned_chunk_response_iter(db);

        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c1_r2"),
            vec![
                proto::create_datum(10, 0.0),
                proto::create_datum(11, 1.0),
                proto::create_datum(12, 2.0),
                proto::create_datum(13, 3.0),
                proto::create_datum(14, 4.0),
                proto::create_datum(15, 5.0),
                proto::create_datum(16, 6.0),
                proto::create_datum(17, 7.0),
                proto::create_datum(18, 8.0),
                proto::create_datum(19, 9.0),
            ],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);
        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_response_iterator_across_pages_nan() {
        let config = helper_create_config(5);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c10_r2",
                1e6,
            )],
            10,    // start
            19,    // end
            10,    // chunk_size
            false, // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c10_r2", "5", "2"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.write_nan_filled(&key, true).unwrap();
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c10_r2", "5", "3"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 3,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page2.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        assert!(page2.write(&page_creator, &config, 0, &data[5..10]).is_ok());

        let mut iter = response.owned_chunk_response_iter(db);

        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c10_r2"),
            vec![
                proto::create_datum(15, 5.0),
                proto::create_datum(16, 6.0),
                proto::create_datum(17, 7.0),
                proto::create_datum(18, 8.0),
                proto::create_datum(19, 9.0),
            ],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);
        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_response_iterator_exact_page_half_chunk() {
        let config = helper_create_config(10);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c1_r3",
                1e6,
            )],
            10,    // start
            14,    // end
            5,     // chunk_size
            false, // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r3", "10", "1"; extension => "bin"),
            start: 0,
            end: 0,
            size: 10,
            id: 1,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        assert!(page.write(&page_creator, &config, 0, &data).is_ok());

        let mut iter = response.owned_chunk_response_iter(db);

        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c1_r3"),
            vec![
                proto::create_datum(10, 0.0),
                proto::create_datum(11, 1.0),
                proto::create_datum(12, 2.0),
                proto::create_datum(13, 3.0),
                proto::create_datum(14, 4.0),
            ],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);
        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_response_iterator_across_pages_and_multiple_chunks() {
        let config = helper_create_config(5);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            "p1", // package_id
            vec![Channel::new(
                // channels
                "cache_c1_r4",
                1e6,
            )],
            10,    // start
            17,    // end
            2,     // chunk_size
            false, // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r4", "5", "2"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r4", "5", "3"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 3,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page2.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        assert!(page.write(&page_creator, &config, 0, &data[0..5]).is_ok());
        assert!(page2.write(&page_creator, &config, 0, &data[5..10]).is_ok());

        let mut iter = response.owned_chunk_response_iter(db);
        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c1_r4"),
            vec![proto::create_datum(10, 0.0), proto::create_datum(11, 1.0)],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(12, 2.0),
            proto::create_datum(13, 3.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(14, 4.0),
            proto::create_datum(15, 5.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(16, 6.0),
            proto::create_datum(17, 7.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_response_iterator_across_pages_and_multiple_chunks_multiple_channels() {
        let config = helper_create_config(5);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            String::from("p1"), // package_id
            vec![
                // channels
                Channel::new("cache_c1_r5", 1e6),
                Channel::new("cache_c2_r5", 1e6),
            ],
            11,    // start
            17,    // end
            2,     // chunk_size
            false, // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r5", "5", "2"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c1_r5", "5", "3"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 3,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page2.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        let page3 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c2_r5", "5", "2"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 2,
        };
        let key = page_key(
            request.package_id(),
            request.channels[1].id(),
            config.page_size(),
            page3.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        let page4 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_c2_r5", "5", "3"; extension => "bin"),
            start: 0,
            end: 0,
            size: 5,
            id: 3,
        };
        let key = page_key(
            request.package_id(),
            request.channels[1].id(),
            config.page_size(),
            page4.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let data2: [f64; 10] = [9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0];

        assert!(page.write(&page_creator, &config, 0, &data[0..5]).is_ok());
        assert!(page2.write(&page_creator, &config, 0, &data[5..10]).is_ok());
        assert!(page3.write(&page_creator, &config, 0, &data2[0..5]).is_ok());
        assert!(page4
            .write(&page_creator, &config, 0, &data2[5..10])
            .is_ok());

        let mut iter = response.owned_chunk_response_iter(db);
        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c1_r5"),
            vec![proto::create_datum(11, 1.0), proto::create_datum(12, 2.0)],
        ));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_c2_r5"),
            vec![proto::create_datum(11, 8.0), proto::create_datum(12, 7.0)],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(13, 3.0),
            proto::create_datum(14, 4.0),
        ]);
        chunk.channels[1].data = RepeatedField::from_vec(vec![
            proto::create_datum(13, 6.0),
            proto::create_datum(14, 5.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(15, 5.0),
            proto::create_datum(16, 6.0),
        ]);
        chunk.channels[1].data = RepeatedField::from_vec(vec![
            proto::create_datum(15, 4.0),
            proto::create_datum(16, 3.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_response_iterator_across_pages_and_multiple_chunks_complex_rate() {
        let config = helper_create_config(15);
        let page_creator = PageCreator::new();
        let db = util::database::temp().unwrap();
        assert!(create_page_template(&config).is_ok());

        let request = Request::new(
            String::from("p1"), // package_id
            vec![Channel::new(
                // channels
                "cache_rate_20",
                20f64,
            )],
            1516560423000000, // start
            1516560424500000, // end
            4 * 50000,        // chunk_size
            false,            // use_cache
        );

        let response = request.get_response(&config);

        let page = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_rate_20", "15", "2022080564"; extension => "bin"),
            start: 0,
            end: 0,
            size: 15,
            id: 2022080564,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();
        let page2 = Page {
            path: path!(&*TEMP_DIR, "p1", "cache_rate_20", "15", "2022080565"; extension => "bin"),
            start: 0,
            end: 0,
            size: 15,
            id: 2022080565,
        };
        let key = page_key(
            request.package_id(),
            request.channels[0].id(),
            config.page_size(),
            page2.id,
        );
        db.upsert_page(&database::PageRecord::new(
            key,
            false,
            true,
            config.page_size() as i64,
        ))
        .unwrap();

        let data: [f64; 30] = [
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0,
        ];

        assert!(page.write(&page_creator, &config, 0, &data[0..15]).is_ok());
        assert!(page2
            .write(&page_creator, &config, 0, &data[15..30])
            .is_ok());

        let mut iter = response.owned_chunk_response_iter(db);
        let mut chunk = ChunkResponse::new();
        chunk.set_channels(RepeatedField::from_vec(Vec::new()));
        chunk.channels.push(proto::create_channel_chunk(
            String::from("cache_rate_20"),
            vec![
                proto::create_datum(1516560423000000, 0.0),
                proto::create_datum(1516560423050000, 1.0),
                proto::create_datum(1516560423100000, 2.0),
                proto::create_datum(1516560423150000, 3.0),
            ],
        ));

        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560423200000, 4.0),
            proto::create_datum(1516560423250000, 5.0),
            proto::create_datum(1516560423300000, 6.0),
            proto::create_datum(1516560423350000, 7.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560423400000, 8.0),
            proto::create_datum(1516560423450000, 9.0),
            proto::create_datum(1516560423500000, 10.0),
            proto::create_datum(1516560423550000, 11.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560423600000, 12.0),
            proto::create_datum(1516560423650000, 13.0),
            proto::create_datum(1516560423700000, 14.0),
            proto::create_datum(1516560423750000, 15.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560423800000, 16.0),
            proto::create_datum(1516560423850000, 17.0),
            proto::create_datum(1516560423900000, 18.0),
            proto::create_datum(1516560423950000, 19.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560424000000, 20.0),
            proto::create_datum(1516560424050000, 21.0),
            proto::create_datum(1516560424100000, 22.0),
            proto::create_datum(1516560424150000, 23.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560424200000, 24.0),
            proto::create_datum(1516560424250000, 25.0),
            proto::create_datum(1516560424300000, 26.0),
            proto::create_datum(1516560424350000, 27.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        chunk.channels[0].data = RepeatedField::from_vec(vec![
            proto::create_datum(1516560424400000, 28.0),
            proto::create_datum(1516560424450000, 29.0),
        ]);
        assert_eq!(helper_convert_chunk(&iter.next().unwrap().unwrap()), chunk);

        assert!(iter.next().is_none());
    }
}
