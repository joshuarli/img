/*!
Bounded JPEG, PNG, WebP and AVIF conversion and verified optimization.

[`still`] decodes bounded still images and preserves explicit sample layouts.
[`optimizer`] selects verified actual encoded bytes under one global encode cap.
[`perceptual`] compares orientation-correct SDR pixels with exact alpha.
[`publication`] creates outputs atomically without clobbering existing files.

Scoring supports validated eight-bit SDR profiles. Exact PNG paths preserve
sixteen-bit samples. Unsupported alpha, precision, HDR, animation and color
representations fail explicitly. See the repository's docs for the supported
representation matrix and independent validation evidence.
*/

#![warn(missing_docs)]
#[cfg(not(any(all(target_os="macos",target_arch="aarch64"),all(target_os="linux",any(target_arch="x86_64",target_arch="aarch64")))))]
compile_error!("rimage supports macOS ARM64 and Linux x86-64/ARM64");

/// All additional operations for the zune_image
pub mod operations;

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

/// Bounded still-image operations.
pub mod still;

/// Atomic per-file output publication.
pub mod publication;

/// Verified perceptual comparison of encoded still images.
pub mod perceptual;

#[cfg(test)]
mod test_utils;

/// Deterministic, bounded optimization and conversion capabilities.
pub mod optimizer;

/// Cooperative cancellation of image work.
pub mod cancellation;
