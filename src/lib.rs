/*!
Bounded JPEG, PNG, WebP and AVIF conversion and verified optimization.

[`still`] decodes bounded still images and preserves explicit sample layouts.
[`optimizer`] selects verified actual encoded bytes under one global encode cap.
[`perceptual`] compares orientation-correct SDR pixels with exact alpha.
[`publication`] creates outputs atomically without clobbering existing files.

Scoring supports eight-bit SDR profiles. Exact PNG paths preserve sixteen-bit
samples. The repository's docs describe supported representations, independent
validation evidence and known validation gaps.
*/

#![warn(missing_docs)]
#[path = "../codecs/png/src/lib.rs"]
pub mod png_stream;
#[cfg(not(any(all(target_os="macos",target_arch="aarch64"),all(target_os="linux",any(target_arch="x86_64",target_arch="aarch64")))))]
compile_error!("img supports macOS ARM64 and Linux x86-64/ARM64");

/// All additional operations for the zune_image
#[cfg(feature = "native-codecs")]
pub mod operations;

/// Resource-derived limits for retained library operations and CLI admission.
///
/// [`limits::LimitSet`] combines format, memory and disk constraints for library
/// callers. The CLI uses [`limits::SystemBudget`] alongside the fixed ceilings
/// enforced by [`still`].
#[cfg(feature = "native-codecs")]
pub mod limits;

/// Structured, side-tagged pipeline errors.
///
/// Wraps [`zune_image::errors::ImageErrors`] so a failure can say whether it
/// happened reading the input or writing the output, and which format was
/// involved. See [`error::ImgError`].
#[cfg(feature = "native-codecs")]
pub mod error;

/// Exit-code categories and an outcome accumulator for library callers.
///
/// [`exit::ExitCode`] distinguishes success, usage, input, output, and partial
/// or mixed failures. The CLI reports the same numeric categories separately.
#[cfg(feature = "native-codecs")]
pub mod exit;

/// Bounded still-image operations.
#[cfg(feature = "native-codecs")]
pub mod still;

/// Atomic per-file output publication.
#[cfg(feature = "native-codecs")]
pub mod publication;

/// Verified perceptual comparison of encoded still images.
#[cfg(feature = "native-codecs")]
pub mod perceptual;

#[cfg(all(test, feature = "native-codecs"))]
mod test_utils;

/// Deterministic, bounded optimization and conversion capabilities.
#[cfg(feature = "native-codecs")]
pub mod optimizer;
#[cfg(feature = "native-codecs")]
pub mod allocator;

/// Cooperative cancellation of image work.
#[cfg(feature = "native-codecs")]
pub mod cancellation;
