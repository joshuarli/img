use super::{Error, ErrorKind};

pub(super) struct Structure {
    pub auxiliary: bool,
    pub hdr: bool,
    pub late_metadata: bool,
    pub icc: bool,
    pub exif: bool,
    pub xmp: bool,
    pub orientation: Option<u8>,
    pub end: usize,
}

enum Position { Headers, Entropy, BetweenScans }

fn invalid() -> Error { Error::new(ErrorKind::Invalid,"invalid or truncated JPEG marker stream") }

pub(super) fn structure(bytes: &[u8]) -> Result<Structure, Error> {
    if !bytes.starts_with(&[255,216]) { return Err(invalid()); }
    let mut result=Structure {auxiliary:false,hdr:false,late_metadata:false,icc:false,exif:false,xmp:false,orientation:None,end:0};
    let mut position=Position::Headers;
    let mut at=2;
    loop {
        if matches!(position,Position::Entropy) {
            at+=bytes.get(at..).ok_or_else(invalid)?.iter().position(|&byte|byte==255).ok_or_else(invalid)?;
            let start=at;
            while bytes.get(at)==Some(&255) {at+=1;}
            let marker=*bytes.get(at).ok_or_else(invalid)?;
            if marker==0 || (0xD0..=0xD7).contains(&marker) {at+=1;continue;}
            at=start;
            position=Position::BetweenScans;
        }
        if bytes.get(at)!=Some(&255) {return Err(invalid());}
        while bytes.get(at)==Some(&255) {at+=1;}
        let marker=*bytes.get(at).ok_or_else(invalid)?;at+=1;
        match marker {
            0xD9=>{result.end=at;return Ok(result);},
            0|0xD8=>return Err(invalid()),
            1|0xD0..=0xD7=>continue,
            _=>{},
        }
        let length=bytes.get(at..at+2).ok_or_else(invalid)?;
        let length=u16::from_be_bytes(length.try_into().unwrap()) as usize;
        if length<2 {return Err(invalid());}
        let end=at.checked_add(length).ok_or_else(invalid)?;
        let payload=bytes.get(at+2..end).ok_or_else(invalid)?;
        if (0xE0..=0xEF).contains(&marker) {
            result.late_metadata|=matches!(position,Position::BetweenScans);
            result.auxiliary|=marker==0xE2 && payload.starts_with(b"MPF\0");
            result.icc|=marker==0xE2 && payload.starts_with(b"ICC_PROFILE\0");
            result.hdr|=marker==0xE2 && payload.starts_with(b"urn:iso:std:iso:ts:21496:-1\0");
            if marker==0xE1 && payload.starts_with(b"Exif\0\0") {
                result.exif=true;
                result.orientation=zencodec::helpers::parse_exif_orientation(&payload[6..]).map(|value|value as u8).or(result.orientation);
            }
            if marker==0xE1 && (payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0") || payload.starts_with(b"http://ns.adobe.com/xmp/extension/\0")) {
                result.xmp=true;
                let namespace=b"http://ns.adobe.com/hdr-gain-map/1.0/";
                result.hdr|=payload.windows(namespace.len()).any(|part|part==namespace);
            }
        }
        at=end;
        if marker==0xDA {position=Position::Entropy;}
    }
}
