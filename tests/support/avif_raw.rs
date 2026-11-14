pub fn primary_data(bytes: &[u8]) -> Vec<u8> {
    let mut offset = 0;
    let mut primary_item = None;
    let mut items: Vec<(u16, Vec<(u32, u32)>)> = Vec::new();
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let payload = offset + 8;
        match kind {
            b"meta" => {
                let mut inner = payload + 4;
                while inner < offset + size {
                    let isize = u32::from_be_bytes(bytes[inner..inner + 4].try_into().unwrap()) as usize;
                    let ikind = &bytes[inner + 4..inner + 8];
                    let ipayload = inner + 8;
                    match ikind {
                        b"pitm" => { primary_item = Some(u16::from_be_bytes(bytes[ipayload + 4..ipayload + 6].try_into().unwrap())); }
                        b"iloc" => {
                            let b0 = bytes[ipayload + 4];
                            assert_eq!(b0, 0x44, "test extractor only supports 32-bit iloc offsets");
                            let count = u16::from_be_bytes(bytes[ipayload + 6..ipayload + 8].try_into().unwrap()) as usize;
                            let mut at = ipayload + 8;
                            for _ in 0..count {
                                let item_id = u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap());
                                let _dri = u16::from_be_bytes(bytes[at + 2..at + 4].try_into().unwrap());
                                let extents = u16::from_be_bytes(bytes[at + 4..at + 6].try_into().unwrap()) as usize;
                                at += 6;
                                let mut extents_list = Vec::with_capacity(extents);
                                for _ in 0..extents {
                                    let offset = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
                                    let length = u32::from_be_bytes(bytes[at + 4..at + 8].try_into().unwrap());
                                    at += 8;
                                    extents_list.push((offset, length));
                                }
                                items.push((item_id, extents_list));
                            }
                        }
                        _ => {}
                    }
                    inner += isize;
                }
            }
            _ => {}
        }
        offset += size;
    }
    let primary = primary_item.expect("pitm box");
    let (_, extents) = items.into_iter().find(|(id, _)| *id == primary).expect("iloc entry for primary item");
    let mut out = Vec::new();
    for (offset, length) in extents {
        let end = offset as usize + length as usize;
        out.extend_from_slice(&bytes[offset as usize..end]);
    }
    out
}

#[allow(dead_code)]
pub struct Av1cFields {
    pub profile: u8,
    pub level: u8,
    pub tier: bool,
    pub high_bitdepth: bool,
    pub chroma_subsampling_x: bool,
    pub chroma_subsampling_y: bool,
    pub chroma_sample_position: u8,
}

#[allow(dead_code)]
pub fn av1c_fields(bytes: &[u8]) -> Av1cFields {
    let at = bytes.windows(4).position(|window| window == b"av1C").expect("av1C box");
    let payload = at + 4;
    let byte1 = bytes[payload + 1];
    let byte2 = bytes[payload + 2];
    Av1cFields {
        profile: byte1 >> 5,
        level: byte1 & 0x1f,
        tier: byte2 >> 7 != 0,
        high_bitdepth: byte2 & 0x40 != 0,
        chroma_subsampling_x: byte2 & 0x08 != 0,
        chroma_subsampling_y: byte2 & 0x04 != 0,
        chroma_sample_position: byte2 & 0x03,
    }
}

#[allow(dead_code)]
fn obu_walk<'a>(mut data: &'a [u8]) -> impl Iterator<Item = (u8, &'a [u8])> {
    std::iter::from_fn(move || {
        if data.is_empty() { return None; }
        let header = data[0];
        data = &data[1..];
        let mut extension = false;
        if header & 0x04 != 0 { data = &data[1..]; extension = true; }
        let _ = extension;
        let size = if header & 0x02 != 0 {
            let mut value = 0usize;
            let mut shift = 0;
            loop {
                let byte = data[0];
                data = &data[1..];
                value |= ((byte & 127) as usize) << shift;
                if byte & 128 == 0 { break; }
                shift += 7;
            }
            value
        } else {
            data.len()
        };
        let payload = data.get(..size)?;
        data = &data[size..];
        Some((header, payload))
    })
}

#[allow(dead_code)]
fn read_bits_msb(bytes: &[u8], mut position: usize, count: usize) -> u64 {
    let mut value = 0u64;
    for _ in 0..count {
        let bit = (bytes[position / 8] >> (7 - position % 8)) & 1;
        value = (value << 1) | u64::from(bit);
        position += 1;
    }
    value
}

#[allow(dead_code)]
pub fn still_picture(bytes: &[u8]) -> bool {
    let data = primary_data(bytes);
    for (header, payload) in obu_walk(&data) {
        if (header >> 3) & 0x0f == 1 {
            return read_bits_msb(payload, 3, 1) != 0;
        }
    }
    panic!("sequence header OBU missing");
}
