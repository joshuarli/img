//! Bounded ISOBMFF/AVIF still-image container reader.
//!
//! Single pass over the bytes: ftyp first, then meta (hdlr, pitm, iloc, iinf,
//! iref, iprp) and mdat/idat are located and bounds-checked, and every count
//! taken from the file is capped before a corresponding allocation.

use std::borrow::Cow;
use std::num::NonZeroU32;

use super::{Error, ErrorKind, IMAGE_MEMORY_BYTES};
use crate::cancellation::Cancellation;

const MAX_META_BOXES: usize = 256;
const MAX_TOP_LEVEL_BOXES: usize = 64;
const MAX_ITEMS: usize = 1024;
const MAX_EXTENTS: usize = 16_384;
const MAX_PROPERTIES: usize = 4096;
const MAX_REFERENCES: usize = 16_384;
const MAX_BRANDS: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Fourcc([u8; 4]);

impl Fourcc {
    const FTYP: Fourcc = Fourcc(*b"ftyp");
    const META: Fourcc = Fourcc(*b"meta");
    const MDAT: Fourcc = Fourcc(*b"mdat");
    const MOOV: Fourcc = Fourcc(*b"moov");
    const FREE: Fourcc = Fourcc(*b"free");
    const SKIP: Fourcc = Fourcc(*b"skip");
    const IDAT: Fourcc = Fourcc(*b"idat");
    const PITM: Fourcc = Fourcc(*b"pitm");
    const ILOC: Fourcc = Fourcc(*b"iloc");
    const IINF: Fourcc = Fourcc(*b"iinf");
    const INFE: Fourcc = Fourcc(*b"infe");
    const IREF: Fourcc = Fourcc(*b"iref");
    const IPRP: Fourcc = Fourcc(*b"iprp");
    const IPCO: Fourcc = Fourcc(*b"ipco");
    const IPMA: Fourcc = Fourcc(*b"ipma");
    const HDLR: Fourcc = Fourcc(*b"hdlr");
    const AV1C: Fourcc = Fourcc(*b"av1C");
    const COLR: Fourcc = Fourcc(*b"colr");
    const PASP: Fourcc = Fourcc(*b"pasp");
    const IROT: Fourcc = Fourcc(*b"irot");
    const IMIR: Fourcc = Fourcc(*b"imir");
    const CLAP: Fourcc = Fourcc(*b"clap");
    const CLLI: Fourcc = Fourcc(*b"clli");
    const MDCV: Fourcc = Fourcc(*b"mdcv");
    const AUXL: Fourcc = Fourcc(*b"auxl");
    const TIME: Fourcc = Fourcc(*b"time");
    const PREM: Fourcc = Fourcc(*b"prem");
    const GRID: Fourcc = Fourcc(*b"grid");
    const TMAP: Fourcc = Fourcc(*b"tmap");
    const GAIN: Fourcc = Fourcc(*b"gain");
    const EXIF: Fourcc = Fourcc(*b"Exif");
    const MIME: Fourcc = Fourcc(*b"mime");
    const URI: Fourcc = Fourcc(*b"uri ");
    const BRAND_AVIF: Fourcc = Fourcc(*b"avif");
    const BRAND_AVIS: Fourcc = Fourcc(*b"avis");
    const BRAND_MIF1: Fourcc = Fourcc(*b"mif1");
    const BRAND_MSF1: Fourcc = Fourcc(*b"msf1");

    fn is_image_brand(self) -> bool {
        matches!(self, Fourcc::BRAND_AVIF | Fourcc::BRAND_AVIS | Fourcc::BRAND_MIF1 | Fourcc::BRAND_MSF1)
    }
}

pub(super) struct Av1Config {
    pub bit_depth: u8,
    pub monochrome: bool,
}

pub(super) struct PrimaryMetadata {
    pub max_frame_width: NonZeroU32,
    pub max_frame_height: NonZeroU32,
    pub still_picture: bool,
}

pub(super) struct PixelAspectRatio {
    pub h_spacing: u32,
    pub v_spacing: u32,
}

pub(super) struct GridConfig {
    pub output_width: u32,
    pub output_height: u32,
}

pub(super) struct ContentLightLevel;

pub(super) struct MasteringDisplay;

pub(super) struct AnimationInfo;

pub(super) struct AlphaMetadata;

pub(super) struct CleanAperture;

pub(super) struct Rotation {
    pub angle: u16,
}

pub(super) struct Mirror {
    pub axis: u8,
}

pub(super) enum ColorInformation {
    Nclx {
        color_primaries: u16,
        transfer_characteristics: u16,
        matrix_coefficients: u16,
        full_range: bool,
    },
    IccProfile(Vec<u8>),
}

#[derive(Clone, Copy, Default)]
struct Extent {
    offset: u64,
    length: u64,
}

struct Item {
    id: u32,
    item_type: Fourcc,
    content_type: Option<Vec<u8>>,
    construction_method: u8,
    extents: Vec<Extent>,
}

#[derive(Clone, Copy)]
struct Reference {
    reference_type: Fourcc,
    to_item: u32,
}

enum Property {
    Other,
    Av1C {
        high_bitdepth: bool,
        twelve_bit: bool,
        monochrome: bool,
        sequence: Option<Av1SequenceInfo>,
    },
    ColrNclx {
        color_primaries: u16,
        transfer_characteristics: u16,
        matrix_coefficients: u16,
        full_range: bool,
    },
    ColrIcc(Vec<u8>),
    Pasp {
        h_spacing: u32,
        v_spacing: u32,
    },
    Irot {
        angle: u8,
    },
    Imir {
        axis: u8,
    },
    Clap,
    Clli,
    Mdcv,
}

#[derive(Clone, Copy)]
struct Av1SequenceInfo {
    still_picture: bool,
    max_frame_width: u32,
    max_frame_height: u32,
    monochrome: bool,
    bit_depth: u8,
}

pub(super) struct AvifContainer<'a> {
    bytes: &'a [u8],
    brands: Vec<Fourcc>,
    has_moov: bool,
    primary_item_id: u32,
    items: Vec<Item>,
    properties: Vec<Property>,
    item_properties: Vec<(u32, Vec<u32>)>,
    references: Vec<Reference>,
}

fn invalid(message: &'static str) -> Error {
    Error::new(ErrorKind::Invalid, message)
}

fn check_stop(stop: &Cancellation) -> Result<(), Error> {
    if stop.is_cancelled() { Err(Error::new(ErrorKind::Invalid, "operation cancelled")) } else { Ok(()) }
}

struct BoxSpan {
    payload_start: usize,
    payload_end: usize,
    box_type: Fourcc,
}

fn read_box(bytes: &[u8], offset: usize) -> Result<BoxSpan, Error> {
    let Some(header) = bytes.get(offset..offset.checked_add(8).ok_or_else(|| invalid("box header overflows"))?) else {
        return Err(invalid("truncated box header"));
    };
    let size = u32::from_be_bytes(header[..4].try_into().unwrap()) as u64;
    let box_type = Fourcc(header[4..8].try_into().unwrap());
    let (payload_start, payload_end) = if size == 1 {
        let Some(large_header) = bytes.get(offset + 8..offset.checked_add(16).ok_or_else(|| invalid("box size overflows"))?) else {
            return Err(invalid("truncated large box header"));
        };
        let large_size = u64::from_be_bytes(large_header.try_into().unwrap());
        if large_size < 16 { return Err(invalid("box size smaller than its header")); }
        let large_size = usize::try_from(large_size).map_err(|_| invalid("box size exceeds file bounds"))?;
        let end = offset.checked_add(large_size).ok_or_else(|| invalid("box exceeds file bounds"))?;
        (offset + 16, end)
    } else if size == 0 {
        (offset + 8, bytes.len())
    } else {
        if size < 8 { return Err(invalid("box size smaller than its header")); }
        let end = offset.checked_add(size as usize).ok_or_else(|| invalid("box exceeds file bounds"))?;
        (offset + 8, end)
    };
    if payload_end > bytes.len() { return Err(invalid("box exceeds file bounds")); }
    Ok(BoxSpan { payload_start, payload_end, box_type })
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    end: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], start: usize, end: usize) -> Self {
        Self { bytes, position: start, end }
    }

    fn remaining(&self) -> usize { self.end - self.position }

    fn take(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let end = self.position.checked_add(length).ok_or_else(|| invalid("box field overflows"))?;
        if end > self.end { return Err(invalid("truncated box payload")); }
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, Error> { Ok(self.take(1)?[0]) }

    fn u16(&mut self) -> Result<u16, Error> { Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap())) }

    fn u24(&mut self) -> Result<u32, Error> {
        let slice = self.take(3)?;
        Ok(u32::from_be_bytes([0, slice[0], slice[1], slice[2]]))
    }

    fn u32(&mut self) -> Result<u32, Error> { Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap())) }

    fn u64(&mut self) -> Result<u64, Error> { Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap())) }

    fn c_string(&mut self) -> Result<Vec<u8>, Error> {
        for index in self.position..self.end {
            if self.bytes[index] == 0 {
                let value = self.bytes[self.position..index].to_vec();
                self.position = index + 1;
                return Ok(value);
            }
        }
        Err(invalid("unterminated string in box"))
    }
}

fn full_box_version(bytes: &[u8], span: &BoxSpan) -> Result<u8, Error> {
    let version = bytes.get(span.payload_start).ok_or_else(|| invalid("truncated full box header"))?;
    if span.payload_end - span.payload_start < 4 { return Err(invalid("truncated full box header")); }
    Ok(*version)
}

#[derive(Default)]
struct ParseState {
    major_brand: Option<Fourcc>,
    brands: Vec<Fourcc>,
    has_moov: bool,
    primary_item_id: Option<u32>,
    items: Vec<Item>,
    properties: Vec<Property>,
    item_properties: Vec<(u32, Vec<u32>)>,
    references: Vec<Reference>,
    idat_payload_start: Option<usize>,
    extent_total: u64,
    seen_meta: bool,
    seen_pitm: bool,
    seen_iloc: bool,
    seen_iinf: bool,
    seen_iprp: bool,
    seen_ipco: bool,
    seen_ipma: bool,
}

impl ParseState {
    fn item_mut(&mut self, id: u32) -> &mut Item {
        if let Some(index) = self.items.iter().position(|item| item.id == id) {
            return &mut self.items[index];
        }
        self.items.push(Item { id, item_type: Fourcc([0; 4]), content_type: None, construction_method: 0, extents: Vec::new() });
        self.items.last_mut().unwrap()
    }

    fn add_extents(&mut self, count: u64) -> Result<(), Error> {
        self.extent_total = self.extent_total.checked_add(count).ok_or_else(|| invalid("extent count overflow"))?;
        if self.extent_total > MAX_EXTENTS as u64 { return Err(invalid("too many extents")); }
        Ok(())
    }
}

fn parse_ftyp(bytes: &[u8], span: &BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    if state.major_brand.is_some() { return Err(invalid("duplicate ftyp box")); }
    let mut reader = Reader::new(bytes, span.payload_start, span.payload_end);
    state.major_brand = Some(Fourcc(reader.take(4)?.try_into().unwrap()));
    let _minor_version = reader.u32()?;
    while reader.remaining() >= 4 {
        check_stop(stop)?;
        if state.brands.len() >= MAX_BRANDS { return Err(invalid("too many compatible brands")); }
        state.brands.push(Fourcc(reader.take(4)?.try_into().unwrap()));
    }
    if reader.remaining() != 0 { return Err(invalid("truncated compatible brand")); }
    Ok(())
}

fn parse_top_level_box(bytes: &[u8], span: BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    match span.box_type {
        Fourcc::FTYP => parse_ftyp(bytes, &span, stop, state),
        Fourcc::META => {
            if state.seen_meta { return Err(invalid("duplicate meta box")); }
            state.seen_meta = true;
            if full_box_version(bytes, &span)? != 0 { return Err(invalid("unsupported meta box version")); }
            parse_meta_children(bytes, &span, stop, state)
        }
        Fourcc::MDAT => Ok(()),
        Fourcc::MOOV => { state.has_moov = true; Ok(()) }
        Fourcc::FREE | Fourcc::SKIP => Ok(()),
        Fourcc::IDAT => {
            if state.idat_payload_start.is_none() { state.idat_payload_start = Some(span.payload_start); }
            Ok(())
        }
        _ => Err(invalid("unsupported top-level box")),
    }
}

fn parse_meta_children(bytes: &[u8], span: &BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    let payload_start = span.payload_start.checked_add(4).ok_or_else(|| invalid("meta box overflows"))?;
    if payload_start > span.payload_end { return Err(invalid("truncated meta box")); }
    let mut offset = payload_start;
    let mut boxes = 0;
    while offset < span.payload_end {
        check_stop(stop)?;
        boxes += 1;
        if boxes > MAX_META_BOXES { return Err(invalid("too many boxes inside meta")); }
        let child = read_box(bytes, offset)?;
        if child.payload_end > span.payload_end { return Err(invalid("box inside meta exceeds its parent")); }
        match child.box_type {
            Fourcc::HDLR => {}
            Fourcc::PITM => {
                if state.seen_pitm { return Err(invalid("duplicate pitm box")); }
                state.seen_pitm = true;
                let version = full_box_version(bytes, &child)?;
                let mut reader = Reader::new(bytes, child.payload_start + 4, child.payload_end);
                state.primary_item_id = Some(if version == 0 { reader.u16()? as u32 } else { reader.u32()? });
            }
            Fourcc::ILOC => {
                if state.seen_iloc { return Err(invalid("duplicate iloc box")); }
                state.seen_iloc = true;
                parse_iloc(bytes, &child, state)?;
            }
            Fourcc::IINF => {
                if state.seen_iinf { return Err(invalid("duplicate iinf box")); }
                state.seen_iinf = true;
                parse_iinf(bytes, &child, stop, state)?;
            }
            Fourcc::IREF => parse_iref(bytes, &child, stop, state)?,
            Fourcc::IPRP => {
                if state.seen_iprp { return Err(invalid("duplicate iprp box")); }
                state.seen_iprp = true;
                parse_iprp(bytes, &child, stop, state)?;
            }
            Fourcc::IDAT => {
                if state.idat_payload_start.is_none() { state.idat_payload_start = Some(child.payload_start); }
            }
            _ => {}
        }
        offset = child.payload_end;
    }
    Ok(())
}

fn parse_iloc(bytes: &[u8], span: &BoxSpan, state: &mut ParseState) -> Result<(), Error> {
    let version = full_box_version(bytes, span)?;
    if version > 2 { return Err(invalid("unsupported iloc version")); }
    let mut reader = Reader::new(bytes, span.payload_start + 4, span.payload_end);
    let sizes = reader.u8()?;
    let offset_size = (sizes >> 4) as usize;
    let length_size = (sizes & 0x0f) as usize;
    if offset_size != 4 && offset_size != 8 { return Err(invalid("unsupported iloc offset size")); }
    if length_size != 4 && length_size != 8 { return Err(invalid("unsupported iloc length size")); }
    let base_sizes = reader.u8()?;
    let base_offset_size = (base_sizes >> 4) as usize;
    if base_offset_size != 0 && base_offset_size != 4 && base_offset_size != 8 { return Err(invalid("unsupported iloc base offset size")); }
    let index_size = (base_sizes & 0x0f) as usize;
    if index_size != 0 && index_size != 4 && index_size != 8 { return Err(invalid("unsupported iloc index size")); }
    let item_count = if version < 2 { reader.u16()? as u64 } else { reader.u32()? as u64 };
    if item_count > MAX_ITEMS as u64 { return Err(invalid("too many items in iloc")); }
    for _ in 0..item_count {
        let item_id = if version < 2 { reader.u16()? as u32 } else { reader.u32()? };
        let construction_method = if version >= 1 {
            let raw = reader.u16()?;
            if raw & 0x3ffc != 0 { return Err(invalid("unsupported iloc reserved bits")); }
            (raw & 0x0003) as u8
        } else { 0 };
        if construction_method > 1 { return Err(invalid("unsupported iloc construction method")); }
        let _data_reference_index = reader.u16()?;
        let base_offset = match base_offset_size {
            0 => 0u64,
            4 => reader.u32()? as u64,
            _ => reader.u64()?,
        };
        let _extent_index = if version >= 1 {
            match index_size {
                0 => 0u64,
                4 => reader.u32()? as u64,
                _ => reader.u64()?,
            }
        } else { 0 };
        let extent_count = reader.u16()? as u64;
        if extent_count == 0 { return Err(invalid("item without extents")); }
        state.add_extents(extent_count)?;
        let mut extents = Vec::with_capacity(extent_count as usize);
        for _ in 0..extent_count {
            let extent_offset = if offset_size == 4 { reader.u32()? as u64 } else { reader.u64()? };
            let extent_length = if length_size == 4 { reader.u32()? as u64 } else { reader.u64()? };
            let absolute = base_offset.checked_add(extent_offset).ok_or_else(|| invalid("extent offset overflows"))?;
            extents.push(Extent { offset: absolute, length: extent_length });
        }
        let item = state.item_mut(item_id);
        item.construction_method = construction_method;
        item.extents = extents;
    }
    Ok(())
}

fn parse_iinf(bytes: &[u8], span: &BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    let version = full_box_version(bytes, span)?;
    let mut reader = Reader::new(bytes, span.payload_start + 4, span.payload_end);
    let entry_count = if version >= 1 { reader.u32()? as u64 } else { reader.u16()? as u64 };
    if entry_count > MAX_ITEMS as u64 { return Err(invalid("too many item info entries")); }
    let mut offset = reader.position;
    for _ in 0..entry_count {
        check_stop(stop)?;
        if state.items.len() >= MAX_ITEMS { return Err(invalid("too many items")); }
        let Some(header) = bytes.get(offset..offset.checked_add(8).ok_or_else(|| invalid("item info box overflows"))?) else {
            return Err(invalid("truncated item info box"));
        };
        if &header[4..8] != Fourcc::INFE.0.as_slice() { return Err(invalid("expected infe box inside iinf")); }
        let size = u32::from_be_bytes(header[..4].try_into().unwrap()) as u64;
        let end = if size == 0 { span.payload_end } else { offset.checked_add(size as usize).ok_or_else(|| invalid("item info box exceeds iinf"))? };
        if end > span.payload_end || end < offset + 8 { return Err(invalid("item info box exceeds iinf")); }
        let mut info = Reader::new(bytes, offset + 8, end);
        let item_version = info.u8()?;
        let _flags = info.u24()?;
        let (id, item_type, content_type) = if item_version < 2 {
            let id = info.u16()? as u32;
            let _protection = info.u16()?;
            let _name = info.c_string()?;
            (id, Fourcc([0; 4]), None)
        } else if item_version == 2 {
            let id = info.u16()? as u32;
            let _protection = info.u16()?;
            let item_type = Fourcc(info.take(4)?.try_into().unwrap());
            let _name = info.c_string()?;
            let content_type = read_content_type(&mut info, item_type)?;
            (id, item_type, content_type)
        } else {
            let id = info.u32()?;
            let _protection = info.u16()?;
            let item_type = Fourcc(info.take(4)?.try_into().unwrap());
            let _name = info.c_string()?;
            let content_type = read_content_type(&mut info, item_type)?;
            (id, item_type, content_type)
        };
        let item = state.item_mut(id);
        item.item_type = item_type;
        item.content_type = content_type;
        offset = end;
    }
    Ok(())
}

fn read_content_type(reader: &mut Reader<'_>, item_type: Fourcc) -> Result<Option<Vec<u8>>, Error> {
    if item_type == Fourcc::MIME || item_type == Fourcc::URI {
        let content_type = reader.c_string()?;
        if reader.remaining() > 0 {
            let _content_encoding = reader.c_string()?;
        }
        Ok(Some(content_type))
    } else {
        Ok(None)
    }
}

fn parse_iref(bytes: &[u8], span: &BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    let version = full_box_version(bytes, span)?;
    let mut offset = span.payload_start.checked_add(4).ok_or_else(|| invalid("iref box overflows"))?;
    if offset > span.payload_end { return Err(invalid("truncated iref box")); }
    while offset < span.payload_end {
        check_stop(stop)?;
        let child = read_box(bytes, offset)?;
        if child.payload_end > span.payload_end { return Err(invalid("reference box exceeds iref")); }
        let mut reader = Reader::new(bytes, child.payload_start, child.payload_end);
        let _from_item = if version == 0 { reader.u16()? as u32 } else { reader.u32()? };
        let count = reader.u16()? as u64;
        state.references.reserve(count as usize);
        for _ in 0..count {
            let to_item = if version == 0 { reader.u16()? as u32 } else { reader.u32()? };
            if state.references.len() >= MAX_REFERENCES { return Err(invalid("too many item references")); }
            state.references.push(Reference { reference_type: child.box_type, to_item });
        }
        if reader.remaining() != 0 { return Err(invalid("trailing bytes in reference box")); }
        offset = child.payload_end;
    }
    Ok(())
}

fn parse_iprp(bytes: &[u8], span: &BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    let mut offset = span.payload_start;
    let mut boxes = 0;
    while offset < span.payload_end {
        check_stop(stop)?;
        boxes += 1;
        if boxes > MAX_META_BOXES { return Err(invalid("too many boxes inside iprp")); }
        let child = read_box(bytes, offset)?;
        if child.payload_end > span.payload_end { return Err(invalid("property box exceeds iprp")); }
        match child.box_type {
            Fourcc::IPCO => {
                if state.seen_ipco { return Err(invalid("duplicate ipco box")); }
                state.seen_ipco = true;
                parse_ipco(bytes, &child, stop, state)?;
            }
            Fourcc::IPMA => {
                if state.seen_ipma { return Err(invalid("duplicate ipma box")); }
                state.seen_ipma = true;
                parse_ipma(bytes, &child, state)?;
            }
            _ => {}
        }
        offset = child.payload_end;
    }
    Ok(())
}


fn parse_ipma_entries(bytes: &[u8], span: &BoxSpan, wide_counts: bool) -> Result<Vec<(u32, Vec<u32>)>, Error> {
    let version = full_box_version(bytes, span)?;
    let wide_indices = version >= 1 && (bytes[span.payload_start + 3] & 1) == 1;
    let mut reader = Reader::new(bytes, span.payload_start + 4, span.payload_end);
    let entry_count = if wide_counts { reader.u32()? as u64 } else { reader.u16()? as u64 };
    if entry_count > MAX_ITEMS as u64 { return Err(invalid("too many property association entries")); }
    let mut entries = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        let item_id = if version >= 2 { reader.u32()? } else { reader.u16()? as u32 };
        let association_count = reader.u8()? as u64;
        let mut associations = Vec::with_capacity(association_count as usize);
        for _ in 0..association_count {
            let raw = if wide_indices { reader.u16()? as u32 } else { reader.u8()? as u32 };
            let index = raw & (if wide_indices { 0x7fff } else { 0x7f });
            if index == 0 { return Err(invalid("property association index out of range")); }
            associations.push(index - 1);
        }
        entries.push((item_id, associations));
    }
    if reader.remaining() != 0 { return Err(invalid("trailing bytes in ipma box")); }
    Ok(entries)
}

fn parse_ipma(bytes: &[u8], span: &BoxSpan, state: &mut ParseState) -> Result<(), Error> {
    let wide_entries = parse_ipma_entries(bytes, span, true);
    match wide_entries {
        Ok(entries) => { state.item_properties.extend(entries); Ok(()) }
        Err(_) => {
            let entries = parse_ipma_entries(bytes, span, false)?;
            state.item_properties.extend(entries);
            Ok(())
        }
    }
}

fn parse_ipco(bytes: &[u8], span: &BoxSpan, stop: &Cancellation, state: &mut ParseState) -> Result<(), Error> {
    let mut offset = span.payload_start;
    while offset < span.payload_end {
        check_stop(stop)?;
        if state.properties.len() >= MAX_PROPERTIES { return Err(invalid("too many item properties")); }
        let child = read_box(bytes, offset)?;
        if child.payload_end > span.payload_end { return Err(invalid("property exceeds ipco")); }
        let property = parse_property(bytes, &child)?;
        state.properties.push(property);
        offset = child.payload_end;
    }
    Ok(())
}

fn parse_property(bytes: &[u8], span: &BoxSpan) -> Result<Property, Error> {
    let box_type = span.box_type;
    match box_type {
        box_type if box_type == Fourcc(*b"ispe") || box_type == Fourcc(*b"pixi") || box_type == Fourcc(*b"auxC") => Ok(Property::Other),
        Fourcc::AV1C => parse_av1c(bytes, span),
        Fourcc::COLR => parse_colr(bytes, span),
        Fourcc::PASP => {
            if span.payload_end - span.payload_start != 8 { return Err(invalid("malformed pasp property")); }
            let mut reader = Reader::new(bytes, span.payload_start, span.payload_end);
            Ok(Property::Pasp { h_spacing: reader.u32()?, v_spacing: reader.u32()? })
        }
        Fourcc::IROT => {
            if span.payload_end - span.payload_start != 1 { return Err(invalid("malformed irot property")); }
            Ok(Property::Irot { angle: bytes[span.payload_start] & 0x03 })
        }
        Fourcc::IMIR => {
            if span.payload_end - span.payload_start != 1 { return Err(invalid("malformed imir property")); }
            Ok(Property::Imir { axis: bytes[span.payload_start] & 0x01 })
        }
        Fourcc::CLAP => {
            if span.payload_end - span.payload_start < 32 { return Err(invalid("malformed clap property")); }
            Ok(Property::Clap)
        }
        Fourcc::CLLI => {
            if span.payload_end - span.payload_start < 4 { return Err(invalid("malformed clli property")); }
            Ok(Property::Clli)
        }
        Fourcc::MDCV => {
            if span.payload_end - span.payload_start < 24 { return Err(invalid("malformed mdcv property")); }
            Ok(Property::Mdcv)
        }
        _ => Ok(Property::Other),
    }
}

fn parse_colr(bytes: &[u8], span: &BoxSpan) -> Result<Property, Error> {
    let remaining = &bytes[span.payload_start..span.payload_end];
    if remaining.len() >= 4 && &remaining[..4] == b"nclx" {
        if remaining.len() != 11 { return Err(invalid("malformed colr nclx property")); }
        let color_primaries = u16::from_be_bytes([remaining[4], remaining[5]]);
        let transfer_characteristics = u16::from_be_bytes([remaining[6], remaining[7]]);
        let matrix_coefficients = u16::from_be_bytes([remaining[8], remaining[9]]);
        let full_range = remaining[10] & 0x80 != 0;
        return Ok(Property::ColrNclx { color_primaries, transfer_characteristics, matrix_coefficients, full_range });
    }
    if remaining.len() >= 4 && (&remaining[..4] == b"prof" || &remaining[..4] == b"rICC" || &remaining[..4] == b"ricC") {
        return Ok(Property::ColrIcc(remaining[4..].to_vec()));
    }
    Ok(Property::Other)
}

fn parse_av1c(bytes: &[u8], span: &BoxSpan) -> Result<Property, Error> {
    let remaining = &bytes[span.payload_start..span.payload_end];
    if remaining.len() < 4 { return Err(invalid("malformed av1C property")); }
    let byte0 = remaining[0];
    if byte0 & 0x80 == 0 { return Err(invalid("av1C marker bit is not set")); }
    let version = byte0 & 0x7f;
    if version != 1 { return Err(invalid("unsupported av1C version")); }
    let profile = remaining[1] >> 5;
    if profile > 2 { return Err(invalid("av1C seq_profile out of range")); }
    let _level = remaining[1] & 0x1f;
    let high_bitdepth = (remaining[2] >> 6) & 1;
    let twelve_bit = (remaining[2] >> 5) & 1;
    let monochrome = (remaining[2] >> 4) & 1;
    let _initial_presentation = remaining[3];
    let sequence = parse_config_obus(&remaining[4..]).ok().flatten();
    Ok(Property::Av1C { high_bitdepth: high_bitdepth != 0, twelve_bit: twelve_bit != 0, monochrome: monochrome != 0, sequence })
}

fn parse_config_obus(data: &[u8]) -> Result<Option<Av1SequenceInfo>, Error> {
    let mut offset = 0usize;
    while offset < data.len() {
        let header = data[offset];
        if header & 0x81 != 0 { return Ok(None); }
        offset += 1;
        if header & 0x04 != 0 {
            if offset >= data.len() { return Ok(None); }
            offset += 1;
        }
        let size = if header & 0x02 != 0 {
            let (value, next) = read_uleb128(data, offset)?;
            offset = next;
            usize::try_from(value).map_err(|_| invalid("sequence header OBU exceeds av1C"))?
        } else {
            data.len() - offset
        };
        let end = offset.checked_add(size).ok_or_else(|| invalid("sequence header OBU exceeds av1C"))?;
        if end > data.len() { return Ok(None); }
        if (header >> 3) & 0x0f == 1 {
            return Ok(Some(parse_sequence_header(&data[offset..end])?));
        }
        offset = end;
    }
    Ok(None)
}

fn sequence_header_in(data: &[u8]) -> Option<Av1SequenceInfo> {
    parse_config_obus(data).ok().flatten()
}

fn read_uleb128(data: &[u8], offset: usize) -> Result<(u64, usize), Error> {
    let mut value = 0u64;
    for shift in (0..56).step_by(7) {
        let index = offset + shift / 7;
        let Some(&byte) = data.get(index) else { return Err(invalid("truncated AV1 OBU length")); };
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok((value, index + 1));
        }
    }
    Err(invalid("invalid AV1 OBU length"))
}

struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn bit(&mut self) -> Result<u8, Error> {
        let byte_index = self.position / 8;
        let bit_index = 7 - (self.position % 8);
        let byte = *self.bytes.get(byte_index).ok_or_else(|| invalid("truncated AV1 sequence header"))?;
        self.position += 1;
        Ok((byte >> bit_index) & 1)
    }

    fn bits(&mut self, count: u32) -> Result<u64, Error> {
        let mut value = 0u64;
        for _ in 0..count {
            value = (value << 1) | u64::from(self.bit()?);
        }
        Ok(value)
    }

    fn uvlc(&mut self) -> Result<u64, Error> {
        let mut leading_zeros = 0u32;
        loop {
            let bit = self.bit()?;
            if bit == 1 { break; }
            leading_zeros += 1;
            if leading_zeros >= 32 { return Err(invalid("AV1 sequence header exceeds uvlc limit")); }
        }
        if leading_zeros == 0 { return Ok(0); }
        let suffix = self.bits(leading_zeros)?;
        Ok((1u64 << leading_zeros) - 1 + suffix)
    }
}

fn parse_sequence_header(payload: &[u8]) -> Result<Av1SequenceInfo, Error> {
    let mut reader = BitReader::new(payload);
    let profile = reader.bits(3)? as u8;
    let still_picture = reader.bit()? != 0;
    let reduced_still_picture_header = reader.bit()? != 0;
    let mut decoder_model_info_present = false;
    let mut decoder_buffer_delay_length = 0u32;
    if reduced_still_picture_header {
        let _seq_level_idx_0 = reader.bits(5)?;
    } else {
        let timing_info_present = reader.bit()? != 0;
        if timing_info_present {
            let _num_units_in_tick = reader.bits(32)?;
            let _time_scale = reader.bits(32)?;
            let equal_picture_interval = reader.bit()? != 0;
            if equal_picture_interval {
                let _num_ticks_per_picture_minus_1 = reader.uvlc()?;
            }
            decoder_model_info_present = reader.bit()? != 0;
            if decoder_model_info_present {
                decoder_buffer_delay_length = (reader.bits(5)? + 1) as u32;
                let _num_units_in_decoding_tick = reader.bits(32)?;
                let _buffer_removal_delay_length = reader.bits(5)? + 1;
                let _frame_presentation_delay_length = reader.bits(5)? + 1;
            }
        }
        let display_model_info_present = reader.bit()? != 0;
        let operating_points_cnt_minus_1 = reader.bits(5)?;
        for _ in 0..=operating_points_cnt_minus_1 {
            let _operating_point_idc = reader.bits(12)?;
            let major_level = reader.bits(3)? + 2;
            let _minor_level = reader.bits(2)?;
            if major_level > 3 { let _seq_tier = reader.bit()?; }
            if decoder_model_info_present {
                let decoder_model_param_present = reader.bit()? != 0;
                if decoder_model_param_present {
                    let _decoder_buffer_delay = reader.bits(decoder_buffer_delay_length)?;
                    let _encoder_buffer_delay = reader.bits(decoder_buffer_delay_length)?;
                    let _low_delay_mode = reader.bit()?;
                }
            }
            let mut display_model_param_present = false;
            if display_model_info_present {
                display_model_param_present = reader.bit()? != 0;
            }
            if display_model_param_present {
                let _initial_display_delay_minus_1 = reader.bits(4)?;
            }
        }
    }
    let width_bits = (reader.bits(4)? + 1) as u32;
    let height_bits = (reader.bits(4)? + 1) as u32;
    let max_frame_width = (reader.bits(width_bits)? + 1) as u32;
    let max_frame_height = (reader.bits(height_bits)? + 1) as u32;
    if !reduced_still_picture_header {
        let frame_id_numbers_present = reader.bit()? != 0;
        if frame_id_numbers_present {
            let _delta_frame_id_length_minus_2 = reader.bits(4)?;
            let _additional_frame_id_length_minus_1 = reader.bits(3)?;
        }
    }
    let _use_128x128_superblock = reader.bit()?;
    let _enable_filter_intra = reader.bit()?;
    let _enable_intra_edge_filter = reader.bit()?;
    if reduced_still_picture_header {
    } else {
        let _enable_interintra_compound = reader.bit()?;
        let _enable_masked_compound = reader.bit()?;
        let _enable_warped_motion = reader.bit()?;
        let _enable_dual_filter = reader.bit()?;
        let enable_order_hint = reader.bit()? != 0;
        if enable_order_hint {
            let _enable_jnt_comp = reader.bit()?;
            let _enable_ref_frame_mvs = reader.bit()?;
        }
        let choose_screen_content_tools = reader.bit()? != 0;
        let mut force_screen_content_tools = 2u8;
        if !choose_screen_content_tools {
            force_screen_content_tools = reader.bit()? as u8;
        }
        if force_screen_content_tools != 0 {
            let choose_integer_mv = reader.bit()? != 0;
            if !choose_integer_mv { let _seq_force_integer_mv = reader.bit()?; }
        }
        if enable_order_hint {
            let _order_hint_bits_minus_1 = reader.bits(3)?;
        }
    }
    let _enable_superres = reader.bit()?;
    let _enable_cdef = reader.bit()?;
    let _enable_restoration = reader.bit()?;
    let mut high_bitdepth = reader.bit()? as u8;
    if profile == 2 && high_bitdepth != 0 && reader.bit()? != 0 {
        high_bitdepth = 2;
    }
    let monochrome = if profile == 1 { false } else { reader.bit()? != 0 };
    let _color_description_present = reader.bit()?;
    let bit_depth = if high_bitdepth == 2 { 12 } else if high_bitdepth == 1 { 10 } else { 8 };
    Ok(Av1SequenceInfo { still_picture, max_frame_width, max_frame_height, monochrome, bit_depth })
}

impl AvifContainer<'_> {
    fn property_associations(&self, item_id: u32) -> Option<&[u32]> {
        self.item_properties.iter().find(|(id, _)| *id == item_id).map(|(_, associations)| associations.as_slice())
    }

    fn on_primary_item(&self, predicate: impl Fn(&Property) -> bool) -> bool {
        match self.property_associations(self.primary_item_id) {
            Some(associations) => associations.iter().any(|&index| self.properties.get(index as usize).is_some_and(&predicate)),
            None => self.properties.iter().any(&predicate),
        }
    }

    fn primary_property<R>(&self, select: impl Fn(&Property) -> Option<R>) -> Option<R> {
        match self.property_associations(self.primary_item_id) {
            Some(associations) => associations.iter().find_map(|&index| self.properties.get(index as usize).and_then(&select)),
            None => self.properties.iter().find_map(&select),
        }
    }

    fn any_property<R>(&self, select: impl Fn(&Property) -> Option<R>) -> Option<R> {
        self.properties.iter().find_map(select)
    }

    fn primary_item(&self) -> Result<&Item, Error> {
        self.items.iter().find(|item| item.id == self.primary_item_id).ok_or_else(|| invalid("primary item missing"))
    }

    fn item_data(&self, item: &Item) -> Result<Vec<u8>, Error> {
        let mut total = 0u64;
        for extent in &item.extents {
            total = total.checked_add(extent.length).ok_or_else(|| invalid("item size overflow"))?;
            if total > IMAGE_MEMORY_BYTES { return Err(Error::new(ErrorKind::Limit, "item data exceeds the memory ceiling")); }
        }
        let mut data = Vec::new();
        data.try_reserve_exact(total as usize).map_err(|_| Error::new(ErrorKind::Limit, "item data exceeds the memory ceiling"))?;
        for extent in &item.extents {
            let start = extent.offset as usize;
            let end = start.checked_add(extent.length as usize).ok_or_else(|| invalid("item extent exceeds the file"))?;
            data.extend_from_slice(&self.bytes[start..end]);
        }
        Ok(data)
    }

    fn is_animation(&self) -> bool {
        self.brands.contains(&Fourcc::BRAND_AVIS)
            || self.has_moov
            || self.references.iter().any(|reference| reference.reference_type == Fourcc::TIME)
    }

    pub(super) fn primary_metadata(&self) -> Result<PrimaryMetadata, Error> {
        let data = self.primary_data()?;
        let sequence = sequence_header_in(&data).ok_or_else(|| invalid("AV1 sequence header missing"))?;
        Ok(PrimaryMetadata {
            max_frame_width: NonZeroU32::new(sequence.max_frame_width).ok_or_else(|| invalid("AV1 frame width is zero"))?,
            max_frame_height: NonZeroU32::new(sequence.max_frame_height).ok_or_else(|| invalid("AV1 frame height is zero"))?,
            still_picture: sequence.still_picture,
        })
    }

    pub(super) fn av1_config(&self) -> Option<Av1Config> {
        let (high_bitdepth, twelve_bit, monochrome, sequence) = self.primary_property(|property| match property {
            Property::Av1C { high_bitdepth, twelve_bit, monochrome, sequence, .. } => Some((*high_bitdepth, *twelve_bit, *monochrome, *sequence)),
            _ => None,
        })?;
        let flag_depth = if twelve_bit { 12 } else if high_bitdepth { 10 } else { 8 };
        let mut sequence = sequence;
        if sequence.is_none() {
            sequence = self.primary_item().ok().and_then(|item| self.item_data(item).ok()).and_then(|data| sequence_header_in(&data));
        }
        let (bit_depth, monochrome) = sequence.map_or((flag_depth, monochrome), |sequence| (sequence.bit_depth, sequence.monochrome));
        Some(Av1Config { bit_depth, monochrome })
    }

    pub(super) fn primary_data(&self) -> Result<Cow<'_, [u8]>, Error> {
        let item = self.primary_item()?;
        Ok(Cow::Owned(self.item_data(item)?))
    }

    pub(super) fn grid_config(&self) -> Option<GridConfig> {
        let item = self.items.iter().find(|item| item.id == self.primary_item_id)?;
        if item.item_type != Fourcc::GRID { return None; }
        let data = self.item_data(item).ok()?;
        let mut reader = Reader::new(&data, 0, data.len());
        let _version = reader.u8().ok()?;
        let flags = reader.u8().ok()?;
        let _rows_minus_one = reader.u8().ok()?;
        let _columns_minus_one = reader.u8().ok()?;
        let wide = flags & 1 == 1;
        let output_width = if wide { reader.u32().ok()? } else { reader.u16().ok()? as u32 };
        let output_height = if wide { reader.u32().ok()? } else { reader.u16().ok()? as u32 };
        Some(GridConfig { output_width, output_height })
    }

    pub(super) fn color_info(&self) -> Option<ColorInformation> {
        self.primary_property(|property| match property {
            Property::ColrNclx { color_primaries, transfer_characteristics, matrix_coefficients, full_range } => Some(ColorInformation::Nclx { color_primaries: *color_primaries, transfer_characteristics: *transfer_characteristics, matrix_coefficients: *matrix_coefficients, full_range: *full_range }),
            Property::ColrIcc(bytes) => Some(ColorInformation::IccProfile(bytes.clone())),
            _ => None,
        })
    }

    pub(super) fn alpha_metadata(&self) -> Option<AlphaMetadata> {
        let primary = self.primary_item_id;
        self.references.iter().any(|reference| reference.reference_type == Fourcc::AUXL && reference.to_item == primary).then_some(AlphaMetadata)
    }

    pub(super) fn animation_info(&self) -> Option<AnimationInfo> {
        if self.is_animation() { Some(AnimationInfo) } else { None }
    }

    pub(super) fn content_light_level(&self) -> Option<ContentLightLevel> {
        self.any_property(|property| match property {
            Property::Clli => Some(ContentLightLevel),
            _ => None,
        })
    }

    pub(super) fn mastering_display(&self) -> Option<MasteringDisplay> {
        self.on_primary_item(|property| matches!(property, Property::Mdcv)).then_some(MasteringDisplay)
    }

    pub(super) fn gain_map_metadata(&self) -> Option<()> {
        self.items.iter().any(|item| item.item_type == Fourcc::GAIN || item.item_type == Fourcc::TMAP).then_some(())
    }

    pub(super) fn pixel_aspect_ratio(&self) -> Option<PixelAspectRatio> {
        self.primary_property(|property| match property {
            Property::Pasp { h_spacing, v_spacing } => Some(PixelAspectRatio { h_spacing: *h_spacing, v_spacing: *v_spacing }),
            _ => None,
        })
    }

    pub(super) fn rotation(&self) -> Option<Rotation> {
        self.primary_property(|property| match property {
            Property::Irot { angle } => Some(Rotation { angle: u16::from(*angle) * 90 }),
            _ => None,
        })
    }

    pub(super) fn mirror(&self) -> Option<Mirror> {
        self.primary_property(|property| match property {
            Property::Imir { axis } => Some(Mirror { axis: *axis }),
            _ => None,
        })
    }

    pub(super) fn clean_aperture(&self) -> Option<CleanAperture> {
        self.on_primary_item(|property| matches!(property, Property::Clap)).then_some(CleanAperture)
    }

    pub(super) fn premultiplied_alpha(&self) -> bool {
        self.references.iter().any(|reference| reference.reference_type == Fourcc::PREM)
    }

    pub(super) fn exif(&self) -> Option<Result<Cow<'_, [u8]>, Error>> {
        let item = self.items.iter().find(|item| item.item_type == Fourcc::EXIF)?;
        Some(self.exif_payload(item).map(Cow::Owned))
    }

    fn exif_payload(&self, item: &Item) -> Result<Vec<u8>, Error> {
        let payload = self.item_data(item)?;
        if payload.len() < 4 { return Err(Error::new(ErrorKind::Invalid, "EXIF item too short")); }
        let offset = u32::from_be_bytes(payload[..4].try_into().unwrap()) as usize;
        let start = 4usize.checked_add(offset).ok_or_else(|| Error::new(ErrorKind::Invalid, "EXIF offset exceeds item size"))?;
        if start > payload.len() { return Err(Error::new(ErrorKind::Invalid, "EXIF offset exceeds item size")); }
        Ok(payload[start..].to_vec())
    }

    pub(super) fn xmp(&self) -> Option<Result<Cow<'_, [u8]>, Error>> {
        let item = self.items.iter().find(|item| item.item_type == Fourcc::MIME && item.content_type.as_deref().is_some_and(content_type_is_xmp))?;
        Some(self.item_data(item).map(Cow::Owned))
    }
}

fn content_type_is_xmp(content_type: &[u8]) -> bool {
    let base = content_type.split(|byte| *byte == b';').next().unwrap_or(content_type);
    let mut bytes: Vec<u8> = base.to_vec();
    bytes.retain(|byte| !byte.is_ascii_whitespace());
    bytes.make_ascii_lowercase();
    bytes == b"application/rdf+xml" || bytes == b"application/xmp+xml"
}

pub(super) fn read<'a>(bytes: &'a [u8], stop: &Cancellation) -> Result<AvifContainer<'a>, Error> {
    let mut state = ParseState::default();
    let mut offset = 0usize;
    let mut boxes = 0;
    let mut first = true;
    while offset < bytes.len() {
        check_stop(stop)?;
        boxes += 1;
        if boxes > MAX_TOP_LEVEL_BOXES { return Err(invalid("too many top-level boxes")); }
        let span = read_box(bytes, offset)?;
        if first && span.box_type != Fourcc::FTYP { return Err(invalid("AVIF file must start with ftyp")); }
        first = false;
        let next = span.payload_end;
        parse_top_level_box(bytes, span, stop, &mut state)?;
        offset = next;
    }
    let major_brand = state.major_brand.ok_or_else(|| invalid("missing ftyp box"))?;
    if !major_brand.is_image_brand() && !state.brands.iter().any(|brand| brand.is_image_brand()) {
        return Err(invalid("unsupported image brand"));
    }
    let primary_item_id = state.primary_item_id.ok_or_else(|| invalid("missing pitm box"))?;
    let idat = state.idat_payload_start;
    for item in state.items.iter_mut() {
        for extent in item.extents.iter_mut() {
            let absolute = if item.construction_method == 1 {
                let base = idat.ok_or_else(|| invalid("item data references missing idat box"))? as u64;
                base.checked_add(extent.offset).ok_or_else(|| invalid("item extent exceeds the file"))?
            } else {
                extent.offset
            };
            let end = absolute.checked_add(extent.length).ok_or_else(|| invalid("item extent exceeds the file"))?;
            if end > bytes.len() as u64 { return Err(invalid("item extent exceeds the file")); }
            extent.offset = absolute;
        }
    }
    for (item_id, associations) in &state.item_properties {
        for &index in associations {
            if index as usize >= state.properties.len() { return Err(invalid("property association index out of range")); }
        }
        if state.items.iter().all(|item| item.id != *item_id) { return Err(invalid("property association references a missing item")); }
    }
    if state.items.iter().all(|item| item.id != primary_item_id) { return Err(invalid("primary item missing from iinf")); }
    Ok(AvifContainer { bytes, brands: std::iter::once(major_brand).chain(state.brands).collect(), has_moov: state.has_moov, primary_item_id, items: state.items, properties: state.properties, item_properties: state.item_properties, references: state.references })
}
