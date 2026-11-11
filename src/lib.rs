/*!
# Rimage

Rimage is a powerful Rust image optimization library extending `zune_image` crate. Designed to enhance your image optimization workflows.

## Features

- Modern codecs:
    - Rimage uses modern codecs optimized to produce tiny images
    - Under the hood uses `zune_image` crate that enhances performance
- Optimization operations:
    - Rimage provides several image optimization operation
    - Resize - uses `fast_image_resize` crate that has incredible performance
    - Quantization - allowing to reduce image palette

## Formats

| Image Format | Decoder | Encoder |
|--------------|---------|---------|
| jpeg         | -       | mozjpeg |
| png          | -       | oxipng  |
| avif         | dav1d   | ravif   |
| webp         | webp    | webp    |
| svg          | resvg   | -       |

> AVIF decoding requires a system-installed `dav1d` (>= 1.3.0) found through
> pkg-config and only handles still images: grid collages and animated
> sequences are rejected, 10/12-bit sources are converted to 8-bit output, and
> ICC profiles are not applied.

*/

#![warn(missing_docs)]

/// All additional operations for the zune_image
pub mod operations;

/// All additional codecs for the zune_image
pub mod codecs;

/// Runtime-derived image size limits.
///
/// Deciding whether an image is too large to process is a property of the
/// machine, not of the source code, so the ceiling is derived from the format's
/// own published limits intersected with the memory actually available. See
/// [`limits::SystemBudget`] and [`limits::LimitSet`].
pub mod limits;

/// Structured, side-tagged pipeline errors.
///
/// Wraps [`zune_image::errors::ImageErrors`] so a failure can say whether it
/// happened reading the input or writing the output, and which format was
/// involved. See [`error::RimageError`].
pub mod error;

/// Exit codes reported to the shell.
///
/// A caller can distinguish a clean success, a usage error, a failure reading
/// an input, a failure writing an output, and a run that only partly succeeded
/// without parsing stderr. See [`exit::ExitCode`].
pub mod exit;

#[cfg(test)]
mod test_utils;
