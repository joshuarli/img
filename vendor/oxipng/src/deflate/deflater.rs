use libdeflater::*;

use crate::{PngError, PngResult};

/// At levels 5 and above, inputs of at least two chunks are compressed as
/// fixed-size chunks, in parallel where available, each primed with the
/// preceding 32 KiB window and joined by sync flushes. The chunk size and input
/// length, not the thread count, determine the output.
const CHUNK: usize = 1 << 20;
const WINDOW: usize = 1 << 15;

pub fn deflate(data: &[u8], level: u8, max_size: Option<usize>) -> PngResult<Vec<u8>> {
    if level >= 5 && data.len() >= 2 * CHUNK {
        return deflate_chunked(data, level, max_size);
    }
    let mut compressor = Compressor::new(CompressionLvl::new(level.into()).unwrap());
    let capacity = max_size.unwrap_or_else(|| compressor.zlib_compress_bound(data.len()));
    let mut dest = vec![0; capacity];
    let len = compressor
        .zlib_compress(data, &mut dest)
        .map_err(|err| match err {
            CompressionError::InsufficientSpace => PngError::DeflatedDataTooLong(capacity),
        })?;
    dest.truncate(len);
    Ok(dest)
}

fn deflate_chunked(data: &[u8], level: u8, max_size: Option<usize>) -> PngResult<Vec<u8>> {
    // libdeflate_deflate_compress_chunk rejects inputs of at most
    // 55 - 4 * level bytes, so such a final remainder joins the preceding chunk.
    let tail = data.len() % CHUNK;
    let count = if tail != 0 && tail <= 55usize.saturating_sub(4 * usize::from(level)) {
        data.len() / CHUNK
    } else {
        data.len().div_ceil(CHUNK)
    };
    let compress = |index: usize| -> PngResult<Vec<u8>> {
        let start = index * CHUNK;
        let prefix = start.min(WINDOW);
        let end = if index + 1 == count { data.len() } else { start + CHUNK };
        let chunk = &data[start - prefix..end];
        let mut compressor = Compressor::new(CompressionLvl::new(level.into()).unwrap());
        let capacity = compressor.deflate_compress_bound(chunk.len() - prefix) + 6;
        let mut dest = vec![0; capacity];
        let len = compressor
            .deflate_compress_chunk(chunk, prefix, index + 1 == count, &mut dest)
            .map_err(|_| PngError::DeflatedDataTooLong(capacity))?;
        dest.truncate(len);
        Ok(dest)
    };
    #[cfg(feature = "parallel")]
    let parts: Vec<PngResult<Vec<u8>>> = {
        use rayon::prelude::*;
        (0..count).into_par_iter().map(compress).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let parts: Vec<PngResult<Vec<u8>>> = (0..count).map(compress).collect();
    let hint: u16 = match level {
        0..=1 => 0,
        2..=5 => 1,
        6..=7 => 2,
        _ => 3,
    };
    let mut header: u16 = 0x7800 | (hint << 6);
    header |= 31 - header % 31;
    let parts: Vec<Vec<u8>> = parts.into_iter().collect::<PngResult<_>>()?;
    let mut out = Vec::with_capacity(6 + parts.iter().map(Vec::len).sum::<usize>());
    out.extend_from_slice(&header.to_be_bytes());
    for part in parts {
        out.extend_from_slice(&part);
        if let Some(max) = max_size
            && out.len() + 4 > max
        {
            return Err(PngError::DeflatedDataTooLong(max));
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    Ok(out)
}

pub fn inflate(data: &[u8], out_size: usize) -> PngResult<Vec<u8>> {
    let mut decompressor = Decompressor::new();
    let mut dest = vec![0; out_size];
    let len = decompressor
        .zlib_decompress(data, &mut dest)
        .map_err(|err| match err {
            DecompressionError::BadData => PngError::InvalidData,
            DecompressionError::InsufficientSpace => PngError::InflatedDataTooLong(out_size),
        })?;
    dest.truncate(len);
    Ok(dest)
}

#[must_use]
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = Crc::new();
    crc.update(data);
    crc.sum()
}
