// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

use protobuf::Message as Message_imported_for_functions;
use protobuf::ProtobufEnum as ProtobufEnum_imported_for_functions;

#[derive(PartialEq,Clone,Default)]
pub struct Segment {
    // message fields
    pub startTs: u64,
    pub source: ::std::string::String,
    pub lastUsed: u64,
    pub unit: ::std::string::String,
    pub samplePeriod: f64,
    pub requestedSamplePeriod: f64,
    pub pageStart: u64,
    pub isMinMax: bool,
    pub unitM: u64,
    pub segmentType: ::std::string::String,
    pub nrPoints: u64,
    pub data: ::std::vec::Vec<f64>,
    pub pageEnd: u64,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for Segment {}

impl Segment {
    pub fn new() -> Segment {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static Segment {
        static mut instance: ::protobuf::lazy::Lazy<Segment> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const Segment,
        };
        unsafe {
            instance.get(Segment::new)
        }
    }

    // uint64 startTs = 1;

    pub fn clear_startTs(&mut self) {
        self.startTs = 0;
    }

    // Param is passed by value, moved
    pub fn set_startTs(&mut self, v: u64) {
        self.startTs = v;
    }

    pub fn get_startTs(&self) -> u64 {
        self.startTs
    }

    fn get_startTs_for_reflect(&self) -> &u64 {
        &self.startTs
    }

    fn mut_startTs_for_reflect(&mut self) -> &mut u64 {
        &mut self.startTs
    }

    // string source = 2;

    pub fn clear_source(&mut self) {
        self.source.clear();
    }

    // Param is passed by value, moved
    pub fn set_source(&mut self, v: ::std::string::String) {
        self.source = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_source(&mut self) -> &mut ::std::string::String {
        &mut self.source
    }

    // Take field
    pub fn take_source(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.source, ::std::string::String::new())
    }

    pub fn get_source(&self) -> &str {
        &self.source
    }

    fn get_source_for_reflect(&self) -> &::std::string::String {
        &self.source
    }

    fn mut_source_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.source
    }

    // uint64 lastUsed = 3;

    pub fn clear_lastUsed(&mut self) {
        self.lastUsed = 0;
    }

    // Param is passed by value, moved
    pub fn set_lastUsed(&mut self, v: u64) {
        self.lastUsed = v;
    }

    pub fn get_lastUsed(&self) -> u64 {
        self.lastUsed
    }

    fn get_lastUsed_for_reflect(&self) -> &u64 {
        &self.lastUsed
    }

    fn mut_lastUsed_for_reflect(&mut self) -> &mut u64 {
        &mut self.lastUsed
    }

    // string unit = 4;

    pub fn clear_unit(&mut self) {
        self.unit.clear();
    }

    // Param is passed by value, moved
    pub fn set_unit(&mut self, v: ::std::string::String) {
        self.unit = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_unit(&mut self) -> &mut ::std::string::String {
        &mut self.unit
    }

    // Take field
    pub fn take_unit(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.unit, ::std::string::String::new())
    }

    pub fn get_unit(&self) -> &str {
        &self.unit
    }

    fn get_unit_for_reflect(&self) -> &::std::string::String {
        &self.unit
    }

    fn mut_unit_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.unit
    }

    // double samplePeriod = 5;

    pub fn clear_samplePeriod(&mut self) {
        self.samplePeriod = 0.;
    }

    // Param is passed by value, moved
    pub fn set_samplePeriod(&mut self, v: f64) {
        self.samplePeriod = v;
    }

    pub fn get_samplePeriod(&self) -> f64 {
        self.samplePeriod
    }

    fn get_samplePeriod_for_reflect(&self) -> &f64 {
        &self.samplePeriod
    }

    fn mut_samplePeriod_for_reflect(&mut self) -> &mut f64 {
        &mut self.samplePeriod
    }

    // double requestedSamplePeriod = 6;

    pub fn clear_requestedSamplePeriod(&mut self) {
        self.requestedSamplePeriod = 0.;
    }

    // Param is passed by value, moved
    pub fn set_requestedSamplePeriod(&mut self, v: f64) {
        self.requestedSamplePeriod = v;
    }

    pub fn get_requestedSamplePeriod(&self) -> f64 {
        self.requestedSamplePeriod
    }

    fn get_requestedSamplePeriod_for_reflect(&self) -> &f64 {
        &self.requestedSamplePeriod
    }

    fn mut_requestedSamplePeriod_for_reflect(&mut self) -> &mut f64 {
        &mut self.requestedSamplePeriod
    }

    // uint64 pageStart = 7;

    pub fn clear_pageStart(&mut self) {
        self.pageStart = 0;
    }

    // Param is passed by value, moved
    pub fn set_pageStart(&mut self, v: u64) {
        self.pageStart = v;
    }

    pub fn get_pageStart(&self) -> u64 {
        self.pageStart
    }

    fn get_pageStart_for_reflect(&self) -> &u64 {
        &self.pageStart
    }

    fn mut_pageStart_for_reflect(&mut self) -> &mut u64 {
        &mut self.pageStart
    }

    // bool isMinMax = 8;

    pub fn clear_isMinMax(&mut self) {
        self.isMinMax = false;
    }

    // Param is passed by value, moved
    pub fn set_isMinMax(&mut self, v: bool) {
        self.isMinMax = v;
    }

    pub fn get_isMinMax(&self) -> bool {
        self.isMinMax
    }

    fn get_isMinMax_for_reflect(&self) -> &bool {
        &self.isMinMax
    }

    fn mut_isMinMax_for_reflect(&mut self) -> &mut bool {
        &mut self.isMinMax
    }

    // uint64 unitM = 9;

    pub fn clear_unitM(&mut self) {
        self.unitM = 0;
    }

    // Param is passed by value, moved
    pub fn set_unitM(&mut self, v: u64) {
        self.unitM = v;
    }

    pub fn get_unitM(&self) -> u64 {
        self.unitM
    }

    fn get_unitM_for_reflect(&self) -> &u64 {
        &self.unitM
    }

    fn mut_unitM_for_reflect(&mut self) -> &mut u64 {
        &mut self.unitM
    }

    // string segmentType = 10;

    pub fn clear_segmentType(&mut self) {
        self.segmentType.clear();
    }

    // Param is passed by value, moved
    pub fn set_segmentType(&mut self, v: ::std::string::String) {
        self.segmentType = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_segmentType(&mut self) -> &mut ::std::string::String {
        &mut self.segmentType
    }

    // Take field
    pub fn take_segmentType(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.segmentType, ::std::string::String::new())
    }

    pub fn get_segmentType(&self) -> &str {
        &self.segmentType
    }

    fn get_segmentType_for_reflect(&self) -> &::std::string::String {
        &self.segmentType
    }

    fn mut_segmentType_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.segmentType
    }

    // uint64 nrPoints = 11;

    pub fn clear_nrPoints(&mut self) {
        self.nrPoints = 0;
    }

    // Param is passed by value, moved
    pub fn set_nrPoints(&mut self, v: u64) {
        self.nrPoints = v;
    }

    pub fn get_nrPoints(&self) -> u64 {
        self.nrPoints
    }

    fn get_nrPoints_for_reflect(&self) -> &u64 {
        &self.nrPoints
    }

    fn mut_nrPoints_for_reflect(&mut self) -> &mut u64 {
        &mut self.nrPoints
    }

    // repeated double data = 12;

    pub fn clear_data(&mut self) {
        self.data.clear();
    }

    // Param is passed by value, moved
    pub fn set_data(&mut self, v: ::std::vec::Vec<f64>) {
        self.data = v;
    }

    // Mutable pointer to the field.
    pub fn mut_data(&mut self) -> &mut ::std::vec::Vec<f64> {
        &mut self.data
    }

    // Take field
    pub fn take_data(&mut self) -> ::std::vec::Vec<f64> {
        ::std::mem::replace(&mut self.data, ::std::vec::Vec::new())
    }

    pub fn get_data(&self) -> &[f64] {
        &self.data
    }

    fn get_data_for_reflect(&self) -> &::std::vec::Vec<f64> {
        &self.data
    }

    fn mut_data_for_reflect(&mut self) -> &mut ::std::vec::Vec<f64> {
        &mut self.data
    }

    // uint64 pageEnd = 13;

    pub fn clear_pageEnd(&mut self) {
        self.pageEnd = 0;
    }

    // Param is passed by value, moved
    pub fn set_pageEnd(&mut self, v: u64) {
        self.pageEnd = v;
    }

    pub fn get_pageEnd(&self) -> u64 {
        self.pageEnd
    }

    fn get_pageEnd_for_reflect(&self) -> &u64 {
        &self.pageEnd
    }

    fn mut_pageEnd_for_reflect(&mut self) -> &mut u64 {
        &mut self.pageEnd
    }
}

impl ::protobuf::Message for Segment {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.startTs = tmp;
                },
                2 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.source)?;
                },
                3 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.lastUsed = tmp;
                },
                4 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.unit)?;
                },
                5 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.samplePeriod = tmp;
                },
                6 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.requestedSamplePeriod = tmp;
                },
                7 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.pageStart = tmp;
                },
                8 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_bool()?;
                    self.isMinMax = tmp;
                },
                9 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.unitM = tmp;
                },
                10 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.segmentType)?;
                },
                11 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.nrPoints = tmp;
                },
                12 => {
                    ::protobuf::rt::read_repeated_double_into(wire_type, is, &mut self.data)?;
                },
                13 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.pageEnd = tmp;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if self.startTs != 0 {
            my_size += ::protobuf::rt::value_size(1, self.startTs, ::protobuf::wire_format::WireTypeVarint);
        }
        if !self.source.is_empty() {
            my_size += ::protobuf::rt::string_size(2, &self.source);
        }
        if self.lastUsed != 0 {
            my_size += ::protobuf::rt::value_size(3, self.lastUsed, ::protobuf::wire_format::WireTypeVarint);
        }
        if !self.unit.is_empty() {
            my_size += ::protobuf::rt::string_size(4, &self.unit);
        }
        if self.samplePeriod != 0. {
            my_size += 9;
        }
        if self.requestedSamplePeriod != 0. {
            my_size += 9;
        }
        if self.pageStart != 0 {
            my_size += ::protobuf::rt::value_size(7, self.pageStart, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.isMinMax != false {
            my_size += 2;
        }
        if self.unitM != 0 {
            my_size += ::protobuf::rt::value_size(9, self.unitM, ::protobuf::wire_format::WireTypeVarint);
        }
        if !self.segmentType.is_empty() {
            my_size += ::protobuf::rt::string_size(10, &self.segmentType);
        }
        if self.nrPoints != 0 {
            my_size += ::protobuf::rt::value_size(11, self.nrPoints, ::protobuf::wire_format::WireTypeVarint);
        }
        if !self.data.is_empty() {
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(self.data.len() as u32) + (self.data.len() * 8) as u32;
        }
        if self.pageEnd != 0 {
            my_size += ::protobuf::rt::value_size(13, self.pageEnd, ::protobuf::wire_format::WireTypeVarint);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if self.startTs != 0 {
            os.write_uint64(1, self.startTs)?;
        }
        if !self.source.is_empty() {
            os.write_string(2, &self.source)?;
        }
        if self.lastUsed != 0 {
            os.write_uint64(3, self.lastUsed)?;
        }
        if !self.unit.is_empty() {
            os.write_string(4, &self.unit)?;
        }
        if self.samplePeriod != 0. {
            os.write_double(5, self.samplePeriod)?;
        }
        if self.requestedSamplePeriod != 0. {
            os.write_double(6, self.requestedSamplePeriod)?;
        }
        if self.pageStart != 0 {
            os.write_uint64(7, self.pageStart)?;
        }
        if self.isMinMax != false {
            os.write_bool(8, self.isMinMax)?;
        }
        if self.unitM != 0 {
            os.write_uint64(9, self.unitM)?;
        }
        if !self.segmentType.is_empty() {
            os.write_string(10, &self.segmentType)?;
        }
        if self.nrPoints != 0 {
            os.write_uint64(11, self.nrPoints)?;
        }
        if !self.data.is_empty() {
            os.write_tag(12, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            // TODO: Data size is computed again, it should be cached
            os.write_raw_varint32((self.data.len() * 8) as u32)?;
            for v in &self.data {
                os.write_double_no_tag(*v)?;
            };
        }
        if self.pageEnd != 0 {
            os.write_uint64(13, self.pageEnd)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for Segment {
    fn new() -> Segment {
        Segment::new()
    }

    fn descriptor_static(_: ::std::option::Option<Segment>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "startTs",
                    Segment::get_startTs_for_reflect,
                    Segment::mut_startTs_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "source",
                    Segment::get_source_for_reflect,
                    Segment::mut_source_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "lastUsed",
                    Segment::get_lastUsed_for_reflect,
                    Segment::mut_lastUsed_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "unit",
                    Segment::get_unit_for_reflect,
                    Segment::mut_unit_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                    "samplePeriod",
                    Segment::get_samplePeriod_for_reflect,
                    Segment::mut_samplePeriod_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                    "requestedSamplePeriod",
                    Segment::get_requestedSamplePeriod_for_reflect,
                    Segment::mut_requestedSamplePeriod_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "pageStart",
                    Segment::get_pageStart_for_reflect,
                    Segment::mut_pageStart_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeBool>(
                    "isMinMax",
                    Segment::get_isMinMax_for_reflect,
                    Segment::mut_isMinMax_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "unitM",
                    Segment::get_unitM_for_reflect,
                    Segment::mut_unitM_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "segmentType",
                    Segment::get_segmentType_for_reflect,
                    Segment::mut_segmentType_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "nrPoints",
                    Segment::get_nrPoints_for_reflect,
                    Segment::mut_nrPoints_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_vec_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                    "data",
                    Segment::get_data_for_reflect,
                    Segment::mut_data_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "pageEnd",
                    Segment::get_pageEnd_for_reflect,
                    Segment::mut_pageEnd_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<Segment>(
                    "Segment",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for Segment {
    fn clear(&mut self) {
        self.clear_startTs();
        self.clear_source();
        self.clear_lastUsed();
        self.clear_unit();
        self.clear_samplePeriod();
        self.clear_requestedSamplePeriod();
        self.clear_pageStart();
        self.clear_isMinMax();
        self.clear_unitM();
        self.clear_segmentType();
        self.clear_nrPoints();
        self.clear_data();
        self.clear_pageEnd();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Segment {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Segment {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct TimeSeriesMessage {
    // message fields
    pub segment: ::protobuf::SingularPtrField<Segment>,
    pub totalResponses: u64,
    pub responseSequenceId: u64,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for TimeSeriesMessage {}

impl TimeSeriesMessage {
    pub fn new() -> TimeSeriesMessage {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static TimeSeriesMessage {
        static mut instance: ::protobuf::lazy::Lazy<TimeSeriesMessage> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const TimeSeriesMessage,
        };
        unsafe {
            instance.get(TimeSeriesMessage::new)
        }
    }

    // .Segment segment = 3;

    pub fn clear_segment(&mut self) {
        self.segment.clear();
    }

    pub fn has_segment(&self) -> bool {
        self.segment.is_some()
    }

    // Param is passed by value, moved
    pub fn set_segment(&mut self, v: Segment) {
        self.segment = ::protobuf::SingularPtrField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_segment(&mut self) -> &mut Segment {
        if self.segment.is_none() {
            self.segment.set_default();
        }
        self.segment.as_mut().unwrap()
    }

    // Take field
    pub fn take_segment(&mut self) -> Segment {
        self.segment.take().unwrap_or_else(|| Segment::new())
    }

    pub fn get_segment(&self) -> &Segment {
        self.segment.as_ref().unwrap_or_else(|| Segment::default_instance())
    }

    fn get_segment_for_reflect(&self) -> &::protobuf::SingularPtrField<Segment> {
        &self.segment
    }

    fn mut_segment_for_reflect(&mut self) -> &mut ::protobuf::SingularPtrField<Segment> {
        &mut self.segment
    }

    // uint64 totalResponses = 7;

    pub fn clear_totalResponses(&mut self) {
        self.totalResponses = 0;
    }

    // Param is passed by value, moved
    pub fn set_totalResponses(&mut self, v: u64) {
        self.totalResponses = v;
    }

    pub fn get_totalResponses(&self) -> u64 {
        self.totalResponses
    }

    fn get_totalResponses_for_reflect(&self) -> &u64 {
        &self.totalResponses
    }

    fn mut_totalResponses_for_reflect(&mut self) -> &mut u64 {
        &mut self.totalResponses
    }

    // uint64 responseSequenceId = 8;

    pub fn clear_responseSequenceId(&mut self) {
        self.responseSequenceId = 0;
    }

    // Param is passed by value, moved
    pub fn set_responseSequenceId(&mut self, v: u64) {
        self.responseSequenceId = v;
    }

    pub fn get_responseSequenceId(&self) -> u64 {
        self.responseSequenceId
    }

    fn get_responseSequenceId_for_reflect(&self) -> &u64 {
        &self.responseSequenceId
    }

    fn mut_responseSequenceId_for_reflect(&mut self) -> &mut u64 {
        &mut self.responseSequenceId
    }
}

impl ::protobuf::Message for TimeSeriesMessage {
    fn is_initialized(&self) -> bool {
        for v in &self.segment {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                3 => {
                    ::protobuf::rt::read_singular_message_into(wire_type, is, &mut self.segment)?;
                },
                7 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.totalResponses = tmp;
                },
                8 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.responseSequenceId = tmp;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if let Some(ref v) = self.segment.as_ref() {
            let len = v.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        }
        if self.totalResponses != 0 {
            my_size += ::protobuf::rt::value_size(7, self.totalResponses, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.responseSequenceId != 0 {
            my_size += ::protobuf::rt::value_size(8, self.responseSequenceId, ::protobuf::wire_format::WireTypeVarint);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if let Some(ref v) = self.segment.as_ref() {
            os.write_tag(3, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        }
        if self.totalResponses != 0 {
            os.write_uint64(7, self.totalResponses)?;
        }
        if self.responseSequenceId != 0 {
            os.write_uint64(8, self.responseSequenceId)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for TimeSeriesMessage {
    fn new() -> TimeSeriesMessage {
        TimeSeriesMessage::new()
    }

    fn descriptor_static(_: ::std::option::Option<TimeSeriesMessage>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_singular_ptr_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<Segment>>(
                    "segment",
                    TimeSeriesMessage::get_segment_for_reflect,
                    TimeSeriesMessage::mut_segment_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "totalResponses",
                    TimeSeriesMessage::get_totalResponses_for_reflect,
                    TimeSeriesMessage::mut_totalResponses_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "responseSequenceId",
                    TimeSeriesMessage::get_responseSequenceId_for_reflect,
                    TimeSeriesMessage::mut_responseSequenceId_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<TimeSeriesMessage>(
                    "TimeSeriesMessage",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for TimeSeriesMessage {
    fn clear(&mut self) {
        self.clear_segment();
        self.clear_totalResponses();
        self.clear_responseSequenceId();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for TimeSeriesMessage {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for TimeSeriesMessage {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Datum {
    // message fields
    pub time: u64,
    pub value: f64,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for Datum {}

impl Datum {
    pub fn new() -> Datum {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static Datum {
        static mut instance: ::protobuf::lazy::Lazy<Datum> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const Datum,
        };
        unsafe {
            instance.get(Datum::new)
        }
    }

    // uint64 time = 1;

    pub fn clear_time(&mut self) {
        self.time = 0;
    }

    // Param is passed by value, moved
    pub fn set_time(&mut self, v: u64) {
        self.time = v;
    }

    pub fn get_time(&self) -> u64 {
        self.time
    }

    fn get_time_for_reflect(&self) -> &u64 {
        &self.time
    }

    fn mut_time_for_reflect(&mut self) -> &mut u64 {
        &mut self.time
    }

    // double value = 2;

    pub fn clear_value(&mut self) {
        self.value = 0.;
    }

    // Param is passed by value, moved
    pub fn set_value(&mut self, v: f64) {
        self.value = v;
    }

    pub fn get_value(&self) -> f64 {
        self.value
    }

    fn get_value_for_reflect(&self) -> &f64 {
        &self.value
    }

    fn mut_value_for_reflect(&mut self) -> &mut f64 {
        &mut self.value
    }
}

impl ::protobuf::Message for Datum {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.time = tmp;
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.value = tmp;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if self.time != 0 {
            my_size += ::protobuf::rt::value_size(1, self.time, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.value != 0. {
            my_size += 9;
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if self.time != 0 {
            os.write_uint64(1, self.time)?;
        }
        if self.value != 0. {
            os.write_double(2, self.value)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for Datum {
    fn new() -> Datum {
        Datum::new()
    }

    fn descriptor_static(_: ::std::option::Option<Datum>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "time",
                    Datum::get_time_for_reflect,
                    Datum::mut_time_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                    "value",
                    Datum::get_value_for_reflect,
                    Datum::mut_value_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<Datum>(
                    "Datum",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for Datum {
    fn clear(&mut self) {
        self.clear_time();
        self.clear_value();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Datum {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Datum {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct ChannelChunk {
    // message fields
    pub id: ::std::string::String,
    pub data: ::protobuf::RepeatedField<Datum>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for ChannelChunk {}

impl ChannelChunk {
    pub fn new() -> ChannelChunk {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static ChannelChunk {
        static mut instance: ::protobuf::lazy::Lazy<ChannelChunk> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ChannelChunk,
        };
        unsafe {
            instance.get(ChannelChunk::new)
        }
    }

    // string id = 1;

    pub fn clear_id(&mut self) {
        self.id.clear();
    }

    // Param is passed by value, moved
    pub fn set_id(&mut self, v: ::std::string::String) {
        self.id = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_id(&mut self) -> &mut ::std::string::String {
        &mut self.id
    }

    // Take field
    pub fn take_id(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.id, ::std::string::String::new())
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    fn get_id_for_reflect(&self) -> &::std::string::String {
        &self.id
    }

    fn mut_id_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.id
    }

    // repeated .Datum data = 2;

    pub fn clear_data(&mut self) {
        self.data.clear();
    }

    // Param is passed by value, moved
    pub fn set_data(&mut self, v: ::protobuf::RepeatedField<Datum>) {
        self.data = v;
    }

    // Mutable pointer to the field.
    pub fn mut_data(&mut self) -> &mut ::protobuf::RepeatedField<Datum> {
        &mut self.data
    }

    // Take field
    pub fn take_data(&mut self) -> ::protobuf::RepeatedField<Datum> {
        ::std::mem::replace(&mut self.data, ::protobuf::RepeatedField::new())
    }

    pub fn get_data(&self) -> &[Datum] {
        &self.data
    }

    fn get_data_for_reflect(&self) -> &::protobuf::RepeatedField<Datum> {
        &self.data
    }

    fn mut_data_for_reflect(&mut self) -> &mut ::protobuf::RepeatedField<Datum> {
        &mut self.data
    }
}

impl ::protobuf::Message for ChannelChunk {
    fn is_initialized(&self) -> bool {
        for v in &self.data {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.id)?;
                },
                2 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.data)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.id.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.id);
        }
        for value in &self.data {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if !self.id.is_empty() {
            os.write_string(1, &self.id)?;
        }
        for v in &self.data {
            os.write_tag(2, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for ChannelChunk {
    fn new() -> ChannelChunk {
        ChannelChunk::new()
    }

    fn descriptor_static(_: ::std::option::Option<ChannelChunk>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "id",
                    ChannelChunk::get_id_for_reflect,
                    ChannelChunk::mut_id_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<Datum>>(
                    "data",
                    ChannelChunk::get_data_for_reflect,
                    ChannelChunk::mut_data_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<ChannelChunk>(
                    "ChannelChunk",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for ChannelChunk {
    fn clear(&mut self) {
        self.clear_id();
        self.clear_data();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for ChannelChunk {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for ChannelChunk {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct ChunkResponse {
    // message fields
    pub channels: ::protobuf::RepeatedField<ChannelChunk>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for ChunkResponse {}

impl ChunkResponse {
    pub fn new() -> ChunkResponse {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static ChunkResponse {
        static mut instance: ::protobuf::lazy::Lazy<ChunkResponse> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ChunkResponse,
        };
        unsafe {
            instance.get(ChunkResponse::new)
        }
    }

    // repeated .ChannelChunk channels = 1;

    pub fn clear_channels(&mut self) {
        self.channels.clear();
    }

    // Param is passed by value, moved
    pub fn set_channels(&mut self, v: ::protobuf::RepeatedField<ChannelChunk>) {
        self.channels = v;
    }

    // Mutable pointer to the field.
    pub fn mut_channels(&mut self) -> &mut ::protobuf::RepeatedField<ChannelChunk> {
        &mut self.channels
    }

    // Take field
    pub fn take_channels(&mut self) -> ::protobuf::RepeatedField<ChannelChunk> {
        ::std::mem::replace(&mut self.channels, ::protobuf::RepeatedField::new())
    }

    pub fn get_channels(&self) -> &[ChannelChunk] {
        &self.channels
    }

    fn get_channels_for_reflect(&self) -> &::protobuf::RepeatedField<ChannelChunk> {
        &self.channels
    }

    fn mut_channels_for_reflect(&mut self) -> &mut ::protobuf::RepeatedField<ChannelChunk> {
        &mut self.channels
    }
}

impl ::protobuf::Message for ChunkResponse {
    fn is_initialized(&self) -> bool {
        for v in &self.channels {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.channels)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        for value in &self.channels {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        for v in &self.channels {
            os.write_tag(1, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for ChunkResponse {
    fn new() -> ChunkResponse {
        ChunkResponse::new()
    }

    fn descriptor_static(_: ::std::option::Option<ChunkResponse>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<ChannelChunk>>(
                    "channels",
                    ChunkResponse::get_channels_for_reflect,
                    ChunkResponse::mut_channels_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<ChunkResponse>(
                    "ChunkResponse",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for ChunkResponse {
    fn clear(&mut self) {
        self.clear_channels();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for ChunkResponse {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for ChunkResponse {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct StateMessage {
    // message fields
    pub status: ::std::string::String,
    pub description: ::std::string::String,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for StateMessage {}

impl StateMessage {
    pub fn new() -> StateMessage {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static StateMessage {
        static mut instance: ::protobuf::lazy::Lazy<StateMessage> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const StateMessage,
        };
        unsafe {
            instance.get(StateMessage::new)
        }
    }

    // string status = 1;

    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    // Param is passed by value, moved
    pub fn set_status(&mut self, v: ::std::string::String) {
        self.status = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_status(&mut self) -> &mut ::std::string::String {
        &mut self.status
    }

    // Take field
    pub fn take_status(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.status, ::std::string::String::new())
    }

    pub fn get_status(&self) -> &str {
        &self.status
    }

    fn get_status_for_reflect(&self) -> &::std::string::String {
        &self.status
    }

    fn mut_status_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.status
    }

    // string description = 2;

    pub fn clear_description(&mut self) {
        self.description.clear();
    }

    // Param is passed by value, moved
    pub fn set_description(&mut self, v: ::std::string::String) {
        self.description = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_description(&mut self) -> &mut ::std::string::String {
        &mut self.description
    }

    // Take field
    pub fn take_description(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.description, ::std::string::String::new())
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    fn get_description_for_reflect(&self) -> &::std::string::String {
        &self.description
    }

    fn mut_description_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.description
    }
}

impl ::protobuf::Message for StateMessage {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.status)?;
                },
                2 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.description)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.status.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.status);
        }
        if !self.description.is_empty() {
            my_size += ::protobuf::rt::string_size(2, &self.description);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if !self.status.is_empty() {
            os.write_string(1, &self.status)?;
        }
        if !self.description.is_empty() {
            os.write_string(2, &self.description)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for StateMessage {
    fn new() -> StateMessage {
        StateMessage::new()
    }

    fn descriptor_static(_: ::std::option::Option<StateMessage>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "status",
                    StateMessage::get_status_for_reflect,
                    StateMessage::mut_status_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "description",
                    StateMessage::get_description_for_reflect,
                    StateMessage::mut_description_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<StateMessage>(
                    "StateMessage",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for StateMessage {
    fn clear(&mut self) {
        self.clear_status();
        self.clear_description();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for StateMessage {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for StateMessage {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct AgentTimeSeriesResponse {
    // message oneof groups
    response_oneof: ::std::option::Option<AgentTimeSeriesResponse_oneof_response_oneof>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for AgentTimeSeriesResponse {}

#[derive(Clone,PartialEq)]
pub enum AgentTimeSeriesResponse_oneof_response_oneof {
    state(StateMessage),
    chunk(ChunkResponse),
}

impl AgentTimeSeriesResponse {
    pub fn new() -> AgentTimeSeriesResponse {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static AgentTimeSeriesResponse {
        static mut instance: ::protobuf::lazy::Lazy<AgentTimeSeriesResponse> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const AgentTimeSeriesResponse,
        };
        unsafe {
            instance.get(AgentTimeSeriesResponse::new)
        }
    }

    // .StateMessage state = 1;

    pub fn clear_state(&mut self) {
        self.response_oneof = ::std::option::Option::None;
    }

    pub fn has_state(&self) -> bool {
        match self.response_oneof {
            ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(..)) => true,
            _ => false,
        }
    }

    // Param is passed by value, moved
    pub fn set_state(&mut self, v: StateMessage) {
        self.response_oneof = ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(v))
    }

    // Mutable pointer to the field.
    pub fn mut_state(&mut self) -> &mut StateMessage {
        if let ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(_)) = self.response_oneof {
        } else {
            self.response_oneof = ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(StateMessage::new()));
        }
        match self.response_oneof {
            ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(ref mut v)) => v,
            _ => panic!(),
        }
    }

    // Take field
    pub fn take_state(&mut self) -> StateMessage {
        if self.has_state() {
            match self.response_oneof.take() {
                ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(v)) => v,
                _ => panic!(),
            }
        } else {
            StateMessage::new()
        }
    }

    pub fn get_state(&self) -> &StateMessage {
        match self.response_oneof {
            ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(ref v)) => v,
            _ => StateMessage::default_instance(),
        }
    }

    // .ChunkResponse chunk = 2;

    pub fn clear_chunk(&mut self) {
        self.response_oneof = ::std::option::Option::None;
    }

    pub fn has_chunk(&self) -> bool {
        match self.response_oneof {
            ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(..)) => true,
            _ => false,
        }
    }

    // Param is passed by value, moved
    pub fn set_chunk(&mut self, v: ChunkResponse) {
        self.response_oneof = ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(v))
    }

    // Mutable pointer to the field.
    pub fn mut_chunk(&mut self) -> &mut ChunkResponse {
        if let ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(_)) = self.response_oneof {
        } else {
            self.response_oneof = ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(ChunkResponse::new()));
        }
        match self.response_oneof {
            ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(ref mut v)) => v,
            _ => panic!(),
        }
    }

    // Take field
    pub fn take_chunk(&mut self) -> ChunkResponse {
        if self.has_chunk() {
            match self.response_oneof.take() {
                ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(v)) => v,
                _ => panic!(),
            }
        } else {
            ChunkResponse::new()
        }
    }

    pub fn get_chunk(&self) -> &ChunkResponse {
        match self.response_oneof {
            ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(ref v)) => v,
            _ => ChunkResponse::default_instance(),
        }
    }
}

impl ::protobuf::Message for AgentTimeSeriesResponse {
    fn is_initialized(&self) -> bool {
        if let Some(AgentTimeSeriesResponse_oneof_response_oneof::state(ref v)) = self.response_oneof {
            if !v.is_initialized() {
                return false;
            }
        }
        if let Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(ref v)) = self.response_oneof {
            if !v.is_initialized() {
                return false;
            }
        }
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeLengthDelimited {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    self.response_oneof = ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::state(is.read_message()?));
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeLengthDelimited {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    self.response_oneof = ::std::option::Option::Some(AgentTimeSeriesResponse_oneof_response_oneof::chunk(is.read_message()?));
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if let ::std::option::Option::Some(ref v) = self.response_oneof {
            match v {
                &AgentTimeSeriesResponse_oneof_response_oneof::state(ref v) => {
                    let len = v.compute_size();
                    my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
                },
                &AgentTimeSeriesResponse_oneof_response_oneof::chunk(ref v) => {
                    let len = v.compute_size();
                    my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
                },
            };
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if let ::std::option::Option::Some(ref v) = self.response_oneof {
            match v {
                &AgentTimeSeriesResponse_oneof_response_oneof::state(ref v) => {
                    os.write_tag(1, ::protobuf::wire_format::WireTypeLengthDelimited)?;
                    os.write_raw_varint32(v.get_cached_size())?;
                    v.write_to_with_cached_sizes(os)?;
                },
                &AgentTimeSeriesResponse_oneof_response_oneof::chunk(ref v) => {
                    os.write_tag(2, ::protobuf::wire_format::WireTypeLengthDelimited)?;
                    os.write_raw_varint32(v.get_cached_size())?;
                    v.write_to_with_cached_sizes(os)?;
                },
            };
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for AgentTimeSeriesResponse {
    fn new() -> AgentTimeSeriesResponse {
        AgentTimeSeriesResponse::new()
    }

    fn descriptor_static(_: ::std::option::Option<AgentTimeSeriesResponse>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_singular_message_accessor::<_, StateMessage>(
                    "state",
                    AgentTimeSeriesResponse::has_state,
                    AgentTimeSeriesResponse::get_state,
                ));
                fields.push(::protobuf::reflect::accessor::make_singular_message_accessor::<_, ChunkResponse>(
                    "chunk",
                    AgentTimeSeriesResponse::has_chunk,
                    AgentTimeSeriesResponse::get_chunk,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<AgentTimeSeriesResponse>(
                    "AgentTimeSeriesResponse",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for AgentTimeSeriesResponse {
    fn clear(&mut self) {
        self.clear_state();
        self.clear_chunk();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for AgentTimeSeriesResponse {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for AgentTimeSeriesResponse {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef<'_> {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &[u8] = b"\
    \n\x20resources/proto/timeseries.proto\"\x85\x03\n\x07Segment\x12\x18\n\
    \x07startTs\x18\x01\x20\x01(\x04R\x07startTs\x12\x16\n\x06source\x18\x02\
    \x20\x01(\tR\x06source\x12\x1a\n\x08lastUsed\x18\x03\x20\x01(\x04R\x08la\
    stUsed\x12\x12\n\x04unit\x18\x04\x20\x01(\tR\x04unit\x12\"\n\x0csamplePe\
    riod\x18\x05\x20\x01(\x01R\x0csamplePeriod\x124\n\x15requestedSamplePeri\
    od\x18\x06\x20\x01(\x01R\x15requestedSamplePeriod\x12\x1c\n\tpageStart\
    \x18\x07\x20\x01(\x04R\tpageStart\x12\x1a\n\x08isMinMax\x18\x08\x20\x01(\
    \x08R\x08isMinMax\x12\x14\n\x05unitM\x18\t\x20\x01(\x04R\x05unitM\x12\
    \x20\n\x0bsegmentType\x18\n\x20\x01(\tR\x0bsegmentType\x12\x1a\n\x08nrPo\
    ints\x18\x0b\x20\x01(\x04R\x08nrPoints\x12\x16\n\x04data\x18\x0c\x20\x03\
    (\x01R\x04dataB\x02\x10\x01\x12\x18\n\x07pageEnd\x18\r\x20\x01(\x04R\x07\
    pageEnd\"\x8f\x01\n\x11TimeSeriesMessage\x12\"\n\x07segment\x18\x03\x20\
    \x01(\x0b2\x08.SegmentR\x07segment\x12&\n\x0etotalResponses\x18\x07\x20\
    \x01(\x04R\x0etotalResponses\x12.\n\x12responseSequenceId\x18\x08\x20\
    \x01(\x04R\x12responseSequenceId\"1\n\x05Datum\x12\x12\n\x04time\x18\x01\
    \x20\x01(\x04R\x04time\x12\x14\n\x05value\x18\x02\x20\x01(\x01R\x05value\
    \":\n\x0cChannelChunk\x12\x0e\n\x02id\x18\x01\x20\x01(\tR\x02id\x12\x1a\
    \n\x04data\x18\x02\x20\x03(\x0b2\x06.DatumR\x04data\":\n\rChunkResponse\
    \x12)\n\x08channels\x18\x01\x20\x03(\x0b2\r.ChannelChunkR\x08channels\"H\
    \n\x0cStateMessage\x12\x16\n\x06status\x18\x01\x20\x01(\tR\x06status\x12\
    \x20\n\x0bdescription\x18\x02\x20\x01(\tR\x0bdescription\"z\n\x17AgentTi\
    meSeriesResponse\x12%\n\x05state\x18\x01\x20\x01(\x0b2\r.StateMessageH\0\
    R\x05state\x12&\n\x05chunk\x18\x02\x20\x01(\x0b2\x0e.ChunkResponseH\0R\
    \x05chunkB\x10\n\x0eresponse_oneofJ\xf5\x0f\n\x06\x12\x04\0\00\x01\n\x08\
    \n\x01\x0c\x12\x03\0\0\x12\n\n\n\x02\x04\0\x12\x04\x02\0\x10\x01\n\n\n\
    \x03\x04\0\x01\x12\x03\x02\x08\x0f\n\x0b\n\x04\x04\0\x02\0\x12\x03\x03\
    \x02\x15\n\r\n\x05\x04\0\x02\0\x04\x12\x04\x03\x02\x02\x11\n\x0c\n\x05\
    \x04\0\x02\0\x05\x12\x03\x03\x02\x08\n\x0c\n\x05\x04\0\x02\0\x01\x12\x03\
    \x03\t\x10\n\x0c\n\x05\x04\0\x02\0\x03\x12\x03\x03\x13\x14\n\x0b\n\x04\
    \x04\0\x02\x01\x12\x03\x04\x02\x14\n\r\n\x05\x04\0\x02\x01\x04\x12\x04\
    \x04\x02\x03\x15\n\x0c\n\x05\x04\0\x02\x01\x05\x12\x03\x04\x02\x08\n\x0c\
    \n\x05\x04\0\x02\x01\x01\x12\x03\x04\t\x0f\n\x0c\n\x05\x04\0\x02\x01\x03\
    \x12\x03\x04\x12\x13\n\x0b\n\x04\x04\0\x02\x02\x12\x03\x05\x02\x16\n\r\n\
    \x05\x04\0\x02\x02\x04\x12\x04\x05\x02\x04\x14\n\x0c\n\x05\x04\0\x02\x02\
    \x05\x12\x03\x05\x02\x08\n\x0c\n\x05\x04\0\x02\x02\x01\x12\x03\x05\t\x11\
    \n\x0c\n\x05\x04\0\x02\x02\x03\x12\x03\x05\x14\x15\n\x0b\n\x04\x04\0\x02\
    \x03\x12\x03\x06\x02\x12\n\r\n\x05\x04\0\x02\x03\x04\x12\x04\x06\x02\x05\
    \x16\n\x0c\n\x05\x04\0\x02\x03\x05\x12\x03\x06\x02\x08\n\x0c\n\x05\x04\0\
    \x02\x03\x01\x12\x03\x06\t\r\n\x0c\n\x05\x04\0\x02\x03\x03\x12\x03\x06\
    \x10\x11\n\x0b\n\x04\x04\0\x02\x04\x12\x03\x07\x02\x1a\n\r\n\x05\x04\0\
    \x02\x04\x04\x12\x04\x07\x02\x06\x12\n\x0c\n\x05\x04\0\x02\x04\x05\x12\
    \x03\x07\x02\x08\n\x0c\n\x05\x04\0\x02\x04\x01\x12\x03\x07\t\x15\n\x0c\n\
    \x05\x04\0\x02\x04\x03\x12\x03\x07\x18\x19\n\x0b\n\x04\x04\0\x02\x05\x12\
    \x03\x08\x02#\n\r\n\x05\x04\0\x02\x05\x04\x12\x04\x08\x02\x07\x1a\n\x0c\
    \n\x05\x04\0\x02\x05\x05\x12\x03\x08\x02\x08\n\x0c\n\x05\x04\0\x02\x05\
    \x01\x12\x03\x08\t\x1e\n\x0c\n\x05\x04\0\x02\x05\x03\x12\x03\x08!\"\n\
    \x0b\n\x04\x04\0\x02\x06\x12\x03\t\x02\x18\n\r\n\x05\x04\0\x02\x06\x04\
    \x12\x04\t\x02\x08#\n\x0c\n\x05\x04\0\x02\x06\x05\x12\x03\t\x02\x08\n\
    \x0c\n\x05\x04\0\x02\x06\x01\x12\x03\t\n\x13\n\x0c\n\x05\x04\0\x02\x06\
    \x03\x12\x03\t\x16\x17\n\x0b\n\x04\x04\0\x02\x07\x12\x03\n\x02\x14\n\r\n\
    \x05\x04\0\x02\x07\x04\x12\x04\n\x02\t\x18\n\x0c\n\x05\x04\0\x02\x07\x05\
    \x12\x03\n\x02\x06\n\x0c\n\x05\x04\0\x02\x07\x01\x12\x03\n\x07\x0f\n\x0c\
    \n\x05\x04\0\x02\x07\x03\x12\x03\n\x12\x13\n\x0b\n\x04\x04\0\x02\x08\x12\
    \x03\x0b\x02\x13\n\r\n\x05\x04\0\x02\x08\x04\x12\x04\x0b\x02\n\x14\n\x0c\
    \n\x05\x04\0\x02\x08\x05\x12\x03\x0b\x02\x08\n\x0c\n\x05\x04\0\x02\x08\
    \x01\x12\x03\x0b\t\x0e\n\x0c\n\x05\x04\0\x02\x08\x03\x12\x03\x0b\x11\x12\
    \n\x0b\n\x04\x04\0\x02\t\x12\x03\x0c\x02\x1a\n\r\n\x05\x04\0\x02\t\x04\
    \x12\x04\x0c\x02\x0b\x13\n\x0c\n\x05\x04\0\x02\t\x05\x12\x03\x0c\x02\x08\
    \n\x0c\n\x05\x04\0\x02\t\x01\x12\x03\x0c\t\x14\n\x0c\n\x05\x04\0\x02\t\
    \x03\x12\x03\x0c\x17\x19\n\x0b\n\x04\x04\0\x02\n\x12\x03\r\x02\x17\n\r\n\
    \x05\x04\0\x02\n\x04\x12\x04\r\x02\x0c\x1a\n\x0c\n\x05\x04\0\x02\n\x05\
    \x12\x03\r\x02\x08\n\x0c\n\x05\x04\0\x02\n\x01\x12\x03\r\t\x11\n\x0c\n\
    \x05\x04\0\x02\n\x03\x12\x03\r\x14\x16\n\x0b\n\x04\x04\0\x02\x0b\x12\x03\
    \x0e\x02*\n\x0c\n\x05\x04\0\x02\x0b\x04\x12\x03\x0e\x02\n\n\x0c\n\x05\
    \x04\0\x02\x0b\x05\x12\x03\x0e\x0b\x11\n\x0c\n\x05\x04\0\x02\x0b\x01\x12\
    \x03\x0e\x12\x16\n\x0c\n\x05\x04\0\x02\x0b\x03\x12\x03\x0e\x19\x1b\n\x0c\
    \n\x05\x04\0\x02\x0b\x08\x12\x03\x0e\x1c)\n\x0f\n\x08\x04\0\x02\x0b\x08\
    \xe7\x07\0\x12\x03\x0e\x1d(\n\x10\n\t\x04\0\x02\x0b\x08\xe7\x07\0\x02\
    \x12\x03\x0e\x1d#\n\x11\n\n\x04\0\x02\x0b\x08\xe7\x07\0\x02\0\x12\x03\
    \x0e\x1d#\n\x12\n\x0b\x04\0\x02\x0b\x08\xe7\x07\0\x02\0\x01\x12\x03\x0e\
    \x1d#\n\x10\n\t\x04\0\x02\x0b\x08\xe7\x07\0\x03\x12\x03\x0e$(\n\x0b\n\
    \x04\x04\0\x02\x0c\x12\x03\x0f\x02\x16\n\r\n\x05\x04\0\x02\x0c\x04\x12\
    \x04\x0f\x02\x0e*\n\x0c\n\x05\x04\0\x02\x0c\x05\x12\x03\x0f\x02\x08\n\
    \x0c\n\x05\x04\0\x02\x0c\x01\x12\x03\x0f\t\x10\n\x0c\n\x05\x04\0\x02\x0c\
    \x03\x12\x03\x0f\x13\x15\n\n\n\x02\x04\x01\x12\x04\x12\0\x16\x01\n\n\n\
    \x03\x04\x01\x01\x12\x03\x12\x08\x19\n\x0b\n\x04\x04\x01\x02\0\x12\x03\
    \x13\x02\x16\n\r\n\x05\x04\x01\x02\0\x04\x12\x04\x13\x02\x12\x1b\n\x0c\n\
    \x05\x04\x01\x02\0\x06\x12\x03\x13\x02\t\n\x0c\n\x05\x04\x01\x02\0\x01\
    \x12\x03\x13\n\x11\n\x0c\n\x05\x04\x01\x02\0\x03\x12\x03\x13\x14\x15\n\
    \x0b\n\x04\x04\x01\x02\x01\x12\x03\x14\x02\x1c\n\r\n\x05\x04\x01\x02\x01\
    \x04\x12\x04\x14\x02\x13\x16\n\x0c\n\x05\x04\x01\x02\x01\x05\x12\x03\x14\
    \x02\x08\n\x0c\n\x05\x04\x01\x02\x01\x01\x12\x03\x14\t\x17\n\x0c\n\x05\
    \x04\x01\x02\x01\x03\x12\x03\x14\x1a\x1b\n\x0b\n\x04\x04\x01\x02\x02\x12\
    \x03\x15\x02\x20\n\r\n\x05\x04\x01\x02\x02\x04\x12\x04\x15\x02\x14\x1c\n\
    \x0c\n\x05\x04\x01\x02\x02\x05\x12\x03\x15\x02\x08\n\x0c\n\x05\x04\x01\
    \x02\x02\x01\x12\x03\x15\t\x1b\n\x0c\n\x05\x04\x01\x02\x02\x03\x12\x03\
    \x15\x1e\x1f\n\n\n\x02\x04\x02\x12\x04\x18\0\x1b\x01\n\n\n\x03\x04\x02\
    \x01\x12\x03\x18\x08\r\n\x0b\n\x04\x04\x02\x02\0\x12\x03\x19\x02\x12\n\r\
    \n\x05\x04\x02\x02\0\x04\x12\x04\x19\x02\x18\x0f\n\x0c\n\x05\x04\x02\x02\
    \0\x05\x12\x03\x19\x02\x08\n\x0c\n\x05\x04\x02\x02\0\x01\x12\x03\x19\t\r\
    \n\x0c\n\x05\x04\x02\x02\0\x03\x12\x03\x19\x10\x11\n\x0b\n\x04\x04\x02\
    \x02\x01\x12\x03\x1a\x02\x13\n\r\n\x05\x04\x02\x02\x01\x04\x12\x04\x1a\
    \x02\x19\x12\n\x0c\n\x05\x04\x02\x02\x01\x05\x12\x03\x1a\x02\x08\n\x0c\n\
    \x05\x04\x02\x02\x01\x01\x12\x03\x1a\t\x0e\n\x0c\n\x05\x04\x02\x02\x01\
    \x03\x12\x03\x1a\x11\x12\n\n\n\x02\x04\x03\x12\x04\x1d\0\x20\x01\n\n\n\
    \x03\x04\x03\x01\x12\x03\x1d\x08\x14\n\x0b\n\x04\x04\x03\x02\0\x12\x03\
    \x1e\x02\x10\n\r\n\x05\x04\x03\x02\0\x04\x12\x04\x1e\x02\x1d\x16\n\x0c\n\
    \x05\x04\x03\x02\0\x05\x12\x03\x1e\x02\x08\n\x0c\n\x05\x04\x03\x02\0\x01\
    \x12\x03\x1e\t\x0b\n\x0c\n\x05\x04\x03\x02\0\x03\x12\x03\x1e\x0e\x0f\n\
    \x0b\n\x04\x04\x03\x02\x01\x12\x03\x1f\x02\x1a\n\x0c\n\x05\x04\x03\x02\
    \x01\x04\x12\x03\x1f\x02\n\n\x0c\n\x05\x04\x03\x02\x01\x06\x12\x03\x1f\
    \x0b\x10\n\x0c\n\x05\x04\x03\x02\x01\x01\x12\x03\x1f\x11\x15\n\x0c\n\x05\
    \x04\x03\x02\x01\x03\x12\x03\x1f\x18\x19\n\n\n\x02\x04\x04\x12\x04\"\0$\
    \x01\n\n\n\x03\x04\x04\x01\x12\x03\"\x08\x15\n\x0b\n\x04\x04\x04\x02\0\
    \x12\x03#\x02%\n\x0c\n\x05\x04\x04\x02\0\x04\x12\x03#\x02\n\n\x0c\n\x05\
    \x04\x04\x02\0\x06\x12\x03#\x0b\x17\n\x0c\n\x05\x04\x04\x02\0\x01\x12\
    \x03#\x18\x20\n\x0c\n\x05\x04\x04\x02\0\x03\x12\x03##$\n\n\n\x02\x04\x05\
    \x12\x04&\0)\x01\n\n\n\x03\x04\x05\x01\x12\x03&\x08\x14\n\x0b\n\x04\x04\
    \x05\x02\0\x12\x03'\x02\x14\n\r\n\x05\x04\x05\x02\0\x04\x12\x04'\x02&\
    \x16\n\x0c\n\x05\x04\x05\x02\0\x05\x12\x03'\x02\x08\n\x0c\n\x05\x04\x05\
    \x02\0\x01\x12\x03'\t\x0f\n\x0c\n\x05\x04\x05\x02\0\x03\x12\x03'\x12\x13\
    \n\x0b\n\x04\x04\x05\x02\x01\x12\x03(\x02\x19\n\r\n\x05\x04\x05\x02\x01\
    \x04\x12\x04(\x02'\x14\n\x0c\n\x05\x04\x05\x02\x01\x05\x12\x03(\x02\x08\
    \n\x0c\n\x05\x04\x05\x02\x01\x01\x12\x03(\t\x14\n\x0c\n\x05\x04\x05\x02\
    \x01\x03\x12\x03(\x17\x18\n\n\n\x02\x04\x06\x12\x04+\00\x01\n\n\n\x03\
    \x04\x06\x01\x12\x03+\x08\x1f\n\x0c\n\x04\x04\x06\x08\0\x12\x04,\x02/\
    \x03\n\x0c\n\x05\x04\x06\x08\0\x01\x12\x03,\x08\x16\n\x0b\n\x04\x04\x06\
    \x02\0\x12\x03-\x04\x1b\n\x0c\n\x05\x04\x06\x02\0\x06\x12\x03-\x04\x10\n\
    \x0c\n\x05\x04\x06\x02\0\x01\x12\x03-\x11\x16\n\x0c\n\x05\x04\x06\x02\0\
    \x03\x12\x03-\x19\x1a\n\x0b\n\x04\x04\x06\x02\x01\x12\x03.\x04\x1c\n\x0c\
    \n\x05\x04\x06\x02\x01\x06\x12\x03.\x04\x11\n\x0c\n\x05\x04\x06\x02\x01\
    \x01\x12\x03.\x12\x17\n\x0c\n\x05\x04\x06\x02\x01\x03\x12\x03.\x1a\x1bb\
    \x06proto3\
";

static mut file_descriptor_proto_lazy: ::protobuf::lazy::Lazy<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::lazy::Lazy {
    lock: ::protobuf::lazy::ONCE_INIT,
    ptr: 0 as *const ::protobuf::descriptor::FileDescriptorProto,
};

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    unsafe {
        file_descriptor_proto_lazy.get(|| {
            parse_descriptor_proto()
        })
    }
}
