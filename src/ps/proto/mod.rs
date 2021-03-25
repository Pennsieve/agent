//! General utility functions go here

use std::vec::Vec;

use protobuf;

// Expose module `ps::proto::timeseries`:
pub mod timeseries;

pub fn create_datum(time: u64, value: f64) -> timeseries::Datum {
    let mut datum = timeseries::Datum::new();
    datum.set_time(time);
    datum.set_value(value);
    datum
}

pub fn create_channel_chunk(id: String, data: Vec<timeseries::Datum>) -> timeseries::ChannelChunk {
    let mut chunk = timeseries::ChannelChunk::new();
    chunk.set_id(id);
    chunk.set_data(protobuf::RepeatedField::from_vec(data));
    chunk
}
