#[macro_use]
extern crate lazy_static;
use protobuf;

use std::collections::HashMap;
use std::path::PathBuf;

use tempfile::tempdir;

use pennsieve::cache::*;
use pennsieve::proto::timeseries::{ChunkResponse, Segment};
use pennsieve::util;

fn helper_convert_chunk(bytes: &Vec<u8>) -> ChunkResponse {
    protobuf::parse_from_bytes(bytes).unwrap()
}

fn segment<'a>(
    start: u64,
    period: f64,
    channel: &'a str,
    range: &mut std::ops::Range<u32>,
) -> Segment {
    let mut data = Vec::new();

    for d in range.into_iter() {
        data.push(d as f64);
    }

    let mut segment = Segment::new();
    segment.set_startTs(start);
    segment.set_source(String::from(channel));
    segment.set_samplePeriod(period);
    segment.set_data(data);
    segment
}

lazy_static! {
    static ref TEMP_DIR: PathBuf = tempdir().unwrap().into_path();
}

#[test]
pub fn test_complex_ts_50() {
    let page_creator = PageCreator::new();
    let config = Config::new(
        &*TEMP_DIR, // base_path
        1000,       // page_size
        0,          // soft_cache_size
        0,          // hard_cache_size
    );
    let db = util::database::temp().unwrap();
    assert!(create_page_template(&config).is_ok());

    let request = Request::new(
        "p:integration:1",                // package_id
        vec![Channel::new("c:2", 50f64)], // channels
        1516550500000000,                 // start
        1516550547000000,                 // end
        1000 * 20000,                     // chunk_size
        false,                            // use_cache
    );

    let mut response = request.get_response(&config);
    response.uncached_page_requests(&db).unwrap();

    for i in 0..3 {
        let pos = i * 20000000;
        let start = 1516550500000000u64 + pos;

        let mut segments = Vec::new();

        // channel 2 - full
        segments.push(segment(start + 0 * 20000, 20000f64, "c:2", &mut (0..500)));
        segments.push(segment(
            start + 500 * 20000,
            20000f64,
            "c:2",
            &mut (500..1000),
        ));

        for segment in segments.iter() {
            response.cache_response(&page_creator, segment).unwrap();
        }
    }

    response.record_page_requests(&db).unwrap();
    let mut counts = HashMap::with_capacity(3);
    counts.insert("c:2", 1000);

    for chunk in response.owned_chunk_response_iter(db) {
        let chunk = chunk.unwrap();
        let chunk = helper_convert_chunk(&chunk);

        for chunk in chunk.channels.into_iter() {
            let count: Option<&usize> = counts.get(&chunk.id.as_ref());
            assert_eq!(chunk.data.len(), *count.unwrap());
        }
    }
}

#[test]
pub fn test_complex_ts_100() {
    let page_creator = PageCreator::new();
    let config = Config::new(
        &*TEMP_DIR, // base_path
        11000,      // page_size
        0,          // soft_cache_size
        0,          // hard_cache_size
    );
    let db = util::database::temp().unwrap();
    assert!(create_page_template(&config).is_ok());

    let request = Request::new(
        "p_integration_1",                // package_id
        vec![Channel::new("c3", 100f64)], // channels
        1516550500000000,                 // start
        1516550547000000,                 // end
        1000 * 10000,                     // chunk_size
        false,                            // use_cache
    );

    let mut response = request.get_response(&config);
    response.uncached_page_requests(&db).unwrap();

    for i in 0..5 {
        let pos = i * 10000000;
        let start = 1516550500000000u64 + pos;

        let mut segments = Vec::new();

        // channel c3 - dense
        segments.push(segment(start + 0 * 10000, 10000f64, "c3", &mut (0..100)));
        segments.push(segment(
            start + 100 * 10000,
            10000f64,
            "c3",
            &mut (100..300),
        ));
        segments.push(segment(
            start + 400 * 10000,
            10000f64,
            "c3",
            &mut (400..410),
        ));
        segments.push(segment(
            start + 410 * 10000,
            10000f64,
            "c3",
            &mut (410..500),
        ));
        segments.push(segment(
            start + 950 * 10000,
            10000f64,
            "c3",
            &mut (950..1000),
        ));

        for segment in segments.iter() {
            response.cache_response(&page_creator, segment).unwrap();
        }
    }

    response.record_page_requests(&db).unwrap();
    let mut counts = HashMap::with_capacity(3);
    counts.insert("c3", 450);

    for chunk in response.owned_chunk_response_iter(db) {
        let chunk = chunk.unwrap();
        let chunk = helper_convert_chunk(&chunk);

        for chunk in chunk.channels.into_iter() {
            let count: Option<&usize> = counts.get(&chunk.id.as_ref());
            assert_eq!(chunk.data.len(), *count.unwrap());
        }
    }
}

#[test]
pub fn test_complex_ts_200() {
    let page_creator = PageCreator::new();
    let config = Config::new(
        &*TEMP_DIR, // base_path
        1000,       // page_size
        0,          // soft_cache_size
        0,          // hard_cache_size
    );
    let db = util::database::temp().unwrap();
    assert!(create_page_template(&config).is_ok());

    let request = Request::new(
        "p_integration_1",                // package_id
        vec![Channel::new("c1", 200f64)], // channels
        1516550500000000,                 // start
        1516550547000000,                 // end
        1000 * 5000,                      // chunk_size
        false,                            // use_cache
    );

    let mut response = request.get_response(&config);
    response.uncached_page_requests(&db).unwrap();

    for i in 0..10 {
        let pos = i * 5000000;
        let start = 1516550500000000u64 + pos;

        let mut segments = Vec::new();

        // channel c1 - sparse
        segments.push(segment(start + 0 * 5000, 5000f64, "c1", &mut (0..5)));
        segments.push(segment(start + 20 * 5000, 5000f64, "c1", &mut (20..30)));
        segments.push(segment(start + 50 * 5000, 5000f64, "c1", &mut (50..51)));
        segments.push(segment(start + 51 * 5000, 5000f64, "c1", &mut (51..52)));
        segments.push(segment(start + 52 * 5000, 5000f64, "c1", &mut (52..53)));
        segments.push(segment(start + 54 * 5000, 5000f64, "c1", &mut (54..55)));
        segments.push(segment(start + 995 * 5000, 5000f64, "c1", &mut (995..998)));

        for segment in segments.iter() {
            response.cache_response(&page_creator, segment).unwrap();
        }
    }

    response.record_page_requests(&db).unwrap();
    let mut counts = HashMap::with_capacity(3);
    counts.insert("c1", 22);

    for chunk in response.owned_chunk_response_iter(db) {
        let chunk = chunk.unwrap();
        let chunk = helper_convert_chunk(&chunk);

        for chunk in chunk.channels.into_iter() {
            let count: Option<&usize> = counts.get(&chunk.id.as_ref());
            assert_eq!(chunk.data.len(), *count.unwrap());
        }
    }
}
