//! Structured input/output errors for retained zune_image library operations.
//!
//! [`ImgError`] wraps [`zune_image::errors::ImageErrors`] with a direction,
//! path, format and optional hint. [`ImgError::source`] exposes the stored
//! cause, and [`Display`] renders its chain.
//!
//! Output helpers can infer a format from the destination extension. Callers
//! that know the encoder use [`output_encode_error`] to supply its name.
//! [`output_io_error`] handles direct filesystem errors, and
//! [`ImgError::log`] prints the error with its optional hint.
//!
//! The still-image CLI uses its own failure and JSON reporting path. These
//! helpers remain available to library callers.
//!
//! [`Display`]: std::fmt::Display

use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use zune_image::errors::{ImageErrors, ImgEncodeErrors};

use crate::limits::{ImageFormatId, LimitViolation, ViolationKind};

/// Which side of the pipeline a failure happened on.
///
/// Kept separate from the specific error so callers can branch on direction
/// without matching every variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Reading, parsing or validating an existing file.
    Input,
    /// Materialising data, encoding, or writing a new file.
    Output,
}

impl Direction {
    /// Human-readable direction, used in messages.
    pub const fn as_str(self) -> &'static str {
        match self {
            Direction::Input => "input",
            Direction::Output => "output",
        }
    }

    /// The verb the pipeline was performing when this direction failed.
    pub const fn verb(self) -> &'static str {
        match self {
            Direction::Input => "reading",
            Direction::Output => "writing",
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What went wrong with a file we were reading.
#[derive(Debug)]
#[non_exhaustive]
pub enum InputError {
    /// The file could not be opened, or its bytes could not be read.
    Open {
        /// The file we were trying to open.
        path: PathBuf,
        /// Why the open failed.
        cause: std::io::Error,
    },
    /// The file existed and was readable, but its contents could not be decoded
    /// into pixels.
    Decode {
        /// The file we were trying to decode.
        path: PathBuf,
        /// The format we identified, from the extension.
        format: ImageFormatId,
        /// The decoder's own report.
        cause: ImageErrors,
    },
    /// The file is a recognised image format whose decoder was not compiled in,
    /// or which this program does not implement.
    UnsupportedFormat {
        /// The file we were asked to read.
        path: PathBuf,
        /// The format we identified.
        format: ImageFormatId,
        /// Whether the decoder is missing from the build or absent entirely.
        reason: UnsupportedReason,
    },
    /// The image is larger than this machine or this format can handle.
    ///
    /// Distinct from [`InputError::Decode`] because it is a property of the
    /// input's size, not of its contents, and is decided before any decoding.
    SizeLimit {
        /// The file we were asked to read.
        path: PathBuf,
        /// The format we identified.
        format: ImageFormatId,
        /// The dimensions, when the header could be read.
        ///
        /// `None` when the limit was derived from the file's byte length rather
        /// than from its header.
        dimensions: Option<(u64, u64)>,
        /// Which ceiling was exceeded, and by how much.
        violation: LimitViolation,
    },
    /// Requested dimensions cannot be represented by the resize operation.
    InvalidResize {
        /// The dimensions that were requested or computed.
        requested: (u64, u64),
        /// Why they are unusable.
        reason: &'static str,
    },
    /// The encoder for the requested output format could not be configured.
    ///
    /// Distinct from [`InputError::Decode`] because the file was read
    /// successfully; the failure is in matching the user's configuration
    /// (quality, colorspace, sampling) to the encoder's capabilities.
    Configuration {
        /// The file being processed.
        path: PathBuf,
        /// The output format that was being set up.
        format: ImageFormatId,
        /// The encoder's own report.
        cause: ImageErrors,
    },
}

/// Why a format could not be handed to a decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedReason {
    /// The required cargo feature was not enabled when this binary was built.
    FeatureNotEnabled,
    /// No decoder exists for this format in this program.
    NotImplemented,
}

impl UnsupportedReason {
    /// Human-readable explanation, used in messages and hints.
    pub const fn as_str(self) -> &'static str {
        match self {
            UnsupportedReason::FeatureNotEnabled => "its decoder was not enabled at build time",
            UnsupportedReason::NotImplemented => "no decoder for this format is implemented",
        }
    }
}

/// What went wrong with a file we were writing.
#[derive(Debug)]
#[non_exhaustive]
pub enum OutputError {
    /// Creating, writing to, or renaming the output file failed.
    Io {
        /// The file we were trying to write.
        path: PathBuf,
        /// Why the write failed.
        cause: std::io::Error,
    },
    /// The encoder for the requested output format failed.
    Encode {
        /// The intended destination.
        path: PathBuf,
        /// The format we were asked to produce.
        format: ImageFormatId,
        /// The encoder's own report.
        cause: ImageErrors,
    },
    /// An image cannot be produced at the requested size.
    SizeLimit {
        /// The intended destination.
        path: PathBuf,
        /// The output format.
        format: ImageFormatId,
        /// Which ceiling was exceeded, and by how much.
        violation: LimitViolation,
    },
    /// There is not enough room on the destination volume.
    ///
    /// Reported separately from [`OutputError::Io`] because the fix is
    /// different: freeing space, not repairing the path.
    OutOfSpace {
        /// The intended destination.
        path: PathBuf,
        /// The format we were asked to produce.
        format: ImageFormatId,
        /// Bytes the encoded image is estimated to need.
        needed: u64,
        /// Bytes the volume has room for.
        available: u64,
    },
}

/// A failure anywhere in the pipeline, tagged with the side it happened on.
#[derive(Debug)]
pub enum ImgError {
    /// A file could not be read, parsed, or accepted.
    Input(InputError),
    /// A result could not be produced or written.
    Output(OutputError),
}

impl ImgError {
    /// Which side of the pipeline failed.
    pub const fn direction(&self) -> Direction {
        match self {
            ImgError::Input(_) => Direction::Input,
            ImgError::Output(_) => Direction::Output,
        }
    }

    /// A stable, machine-readable slug for this failure.
    ///
    /// Intended for logs and tests; the human-facing text comes from
    /// [`Display`](std::fmt::Display).
    pub const fn kind(&self) -> &'static str {
        match self {
            ImgError::Input(InputError::Open { .. }) => "input.open",
            ImgError::Input(InputError::Decode { .. }) => "input.decode",
            ImgError::Input(InputError::UnsupportedFormat { .. }) => "input.unsupported-format",
            ImgError::Input(InputError::SizeLimit { .. }) => "input.size-limit",
            ImgError::Input(InputError::InvalidResize { .. }) => "input.invalid-resize",
            ImgError::Input(InputError::Configuration { .. }) => "input.configuration",
            ImgError::Output(OutputError::Io { .. }) => "output.io",
            ImgError::Output(OutputError::Encode { .. }) => "output.encode",
            ImgError::Output(OutputError::SizeLimit { .. }) => "output.size-limit",
            ImgError::Output(OutputError::OutOfSpace { .. }) => "output.out-of-space",
        }
    }

    /// The file this failure concerns.
    pub fn path(&self) -> &Path {
        match self {
            ImgError::Input(InputError::Open { path, .. })
            | ImgError::Input(InputError::Decode { path, .. })
            | ImgError::Input(InputError::UnsupportedFormat { path, .. })
            | ImgError::Input(InputError::SizeLimit { path, .. })
            | ImgError::Input(InputError::Configuration { path, .. }) => path,
            ImgError::Input(InputError::InvalidResize { .. }) => Path::new(""),
            ImgError::Output(OutputError::Io { path, .. })
            | ImgError::Output(OutputError::Encode { path, .. })
            | ImgError::Output(OutputError::SizeLimit { path, .. })
            | ImgError::Output(OutputError::OutOfSpace { path, .. }) => path,
        }
    }

    /// A concrete next step, when one exists.
    ///
    /// `None` when the failure has no actionable advice beyond fixing the file,
    /// so callers can omit an empty "hint:" line.
    pub fn hint(&self) -> Option<String> {
        match self {
            ImgError::Input(InputError::UnsupportedFormat { format, reason, .. }) => {
                Some(match reason {
                    UnsupportedReason::FeatureNotEnabled => format!(
                        "rebuild with the feature that provides the {} decoder",
                        format.name()
                    ),
                    UnsupportedReason::NotImplemented => {
                        "convert the file to a supported format (jpeg, png, webp or avif) before optimizing it"
                            .to_string()
                    }
                })
            }
            ImgError::Input(InputError::SizeLimit {
                dimensions,
                violation,
                ..
            }) => Some(size_limit_hint(*dimensions, violation, "shrink")),
            ImgError::Input(InputError::InvalidResize { requested, .. }) => Some(format!(
                "choose a --resize value that fits in {}x{} pixels",
                requested.0, requested.1
            )),
            ImgError::Output(OutputError::SizeLimit { violation, .. }) => {
                Some(size_limit_hint(None, violation, "shrink"))
            }
            ImgError::Output(OutputError::OutOfSpace {
                needed, available, ..
            }) => Some(format!(
                // `-t` is a concurrency, so fewer images at once means a
                // *lower* value: the claim used to read "raise -t", which
                // pointed the user at the change that makes it worse.
                "free at least {} on the destination volume, lower -t to process fewer \
                     images at once, or lower --speed",
                human_bytes(needed.saturating_sub(*available))
            )),
            ImgError::Output(OutputError::Io { cause, .. }) => match cause.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    Some("check write permissions on the output directory".to_string())
                }
                std::io::ErrorKind::StorageFull => {
                    Some("free up space on the destination volume or write elsewhere".to_string())
                }
                _ => None,
            },
            ImgError::Input(InputError::Open { cause, .. }) => match cause.kind() {
                std::io::ErrorKind::NotFound => {
                    Some("check that the path exists and the spelling is correct".to_string())
                }
                std::io::ErrorKind::PermissionDenied => {
                    Some("check read permissions on the file".to_string())
                }
                _ => None,
            },
            ImgError::Input(InputError::Decode { .. }) => {
                Some("check that the file is not truncated or corrupt".to_string())
            }
            ImgError::Input(InputError::Configuration { cause, .. }) => {
                // The cause text already names the invalid value and the
                // constraint (e.g. "Unsupported JPEG colorspace: foo"),
                // so there is nothing actionable to add beyond restating it.
                // Returning None avoids a generic "check the docs" hint that
                // is less useful than the error text itself.
                let _ = cause;
                None
            }
            ImgError::Output(OutputError::Encode { cause, .. }) => match cause {
                ImageErrors::EncodeErrors(ImgEncodeErrors::UnsupportedColorspace(..)) => {
                    Some("choose an output format that accepts this image's colorspace".to_string())
                }
                // An encode failure is a property of the output side — the
                // input decoded fine — so the input-side "truncated or
                // corrupt" advice would send the user looking at the wrong
                // file.
                _ => Some(
                    "check that this encoder supports the image's dimensions and bit depth, \
                     or choose another output format"
                        .to_string(),
                ),
            },
        }
    }

    /// Report this failure through the `log` crate.
    ///
    /// The message line is the [`Display`](std::fmt::Display) text — which
    /// already names the file — so it stays a single grep-able line without
    /// printing the path twice. (A leading `path: ` prefix also produced a
    /// bare `": "` for the path-less `InvalidResize`.) The hint follows on its
    /// own `hint: ` line because it names a different thing to do rather than
    /// more context about what broke. The [`kind`](ImgError::kind) slug is
    /// included so a log capture can be filtered by failure class without
    /// matching prose.
    ///
    /// Kept here rather than in `main.rs` so every call site formats failures
    /// identically, including the ones inside the worker threads.
    pub fn log(&self) {
        log::error!("{self} [{}]", self.kind());

        // A failure with no actionable advice prints one line, not two with an
        // empty second one.
        if let Some(hint) = self.hint() {
            log::error!("  hint: {hint}");
        }
    }
}

/// Build an "the input is too large" failure from a resolved limit check.
///
/// Convenience for the pre-decode size check, which knows the path, the format
/// and the dimensions but has no `ImageErrors` to classify.
pub fn input_size_limit(
    path: &Path,
    format: ImageFormatId,
    dimensions: Option<(u64, u64)>,
    violation: LimitViolation,
) -> ImgError {
    ImgError::Input(InputError::SizeLimit {
        path: path.to_path_buf(),
        format,
        dimensions,
        violation,
    })
}

/// Build an "the output cannot be produced" failure from a resolved limit check.
pub fn output_size_limit(
    path: &Path,
    format: ImageFormatId,
    violation: LimitViolation,
) -> ImgError {
    ImgError::Output(OutputError::SizeLimit {
        path: path.to_path_buf(),
        format,
        violation,
    })
}

/// Build an input IO failure (cannot open or read the file).
///
/// Used at call sites that produce `io::Error` while reading the input file,
/// such as `Path::metadata()`, so they are reported with the same structured
/// form as decode failures.
pub fn input_open_error(path: &Path, error: &std::io::Error) -> ImgError {
    ImgError::Input(InputError::Open {
        path: path.to_path_buf(),
        cause: std::io::Error::new(error.kind(), error.to_string()),
    })
}

/// Build an encoder-configuration failure.
///
/// Used when `encoder()` rejects a CLI argument (e.g. an unsupported
/// `--colorspace`). The file was read successfully — the failure is in
/// matching the user's configuration to the encoder's capabilities, which is
/// why it has its own variant rather than being labelled a decode error.
pub fn input_config_error(path: &Path, encoder_name: &str, error: &ImageErrors) -> ImgError {
    ImgError::Input(InputError::Configuration {
        path: path.to_path_buf(),
        format: ImageFormatId::from_encoder_name(encoder_name),
        cause: clone_image_errors(error),
    })
}

/// Build an operation-execution failure.
///
/// Wraps `op.execute()` errors through [`classify_input`] so they get a
/// format tag and hint. Operations like resize and quantize fail when the
/// image's bit depth or colorspace does not match the operation's
/// requirements; `classify_input` already handles the resize-specific case.
pub fn input_operation_error(path: &Path, error: &ImageErrors) -> ImgError {
    classify_input(path, error, None)
}

/// Build a hint naming the violated ceiling and a size that would fit.
fn size_limit_hint(
    dimensions: Option<(u64, u64)>,
    violation: &LimitViolation,
    verb: &str,
) -> String {
    // A volume running out of space is not something resizing the image fixes
    // in general: the destination is the thing that has to change. Naming the
    // free space to aim for is more useful than restating the pixel ceiling.
    if violation.kind == ViolationKind::Bytes {
        return format!(
            "free up space on the destination volume, or write elsewhere; \
             writing this image needs about {} but only {} is allowed (from {})",
            human_bytes(violation.actual),
            human_bytes(violation.allowed),
            violation.binding.describe()
        );
    }

    let measured = match violation.kind {
        ViolationKind::Width => "width",
        ViolationKind::Height => "height",
        ViolationKind::Pixels => "total pixel count",
        ViolationKind::Bytes => unreachable!("handled above"),
    };

    format!(
        "{verb} the image so its {measured} is at most {}; this limit comes from {}",
        human_count(violation.allowed),
        violation.binding.describe()
    ) + &match dimensions {
        Some((w, h)) => format!(" (this image is {w}x{h})"),
        None => String::new(),
    }
}

/// Render a byte count with a binary unit suffix.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Render a large count with a billion/million suffix.
pub fn human_count(count: u64) -> String {
    if count >= 1_000_000_000 {
        format!("{:.1} billion", count as f64 / 1e9)
    } else if count >= 1_000_000 {
        format!("{:.1} million", count as f64 / 1e6)
    } else {
        count.to_string()
    }
}

impl Display for ImgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ImgError::Input(InputError::Open { path, cause }) => {
                write!(f, "input: cannot open {}: {cause}", path.display())
            }
            ImgError::Input(InputError::Decode {
                path,
                format,
                cause,
            }) => write!(
                f,
                "input: cannot decode {} as {}: {}",
                path.display(),
                format.name(),
                flatten(cause)
            ),
            ImgError::Input(InputError::UnsupportedFormat {
                path,
                format,
                reason,
            }) => write!(
                f,
                "input: {} is a {} file, but {}",
                path.display(),
                format.name(),
                reason.as_str()
            ),
            ImgError::Input(InputError::SizeLimit {
                path,
                format,
                dimensions,
                violation,
            }) => {
                write!(
                    f,
                    "input: {} is too large to process as {}: ",
                    path.display(),
                    format.name()
                )?;
                write_violation(f, violation)?;
                if let Some((w, h)) = dimensions {
                    write!(f, " (image is {w}x{h})")?;
                }
                Ok(())
            }
            ImgError::Input(InputError::InvalidResize { requested, reason }) => write!(
                f,
                "input: cannot resize to {}x{}: {reason}",
                requested.0, requested.1
            ),
            ImgError::Input(InputError::Configuration {
                path,
                format,
                cause,
            }) => write!(
                f,
                "input: cannot configure {} encoder for {}: {}",
                format.name(),
                path.display(),
                flatten(cause)
            ),
            ImgError::Output(OutputError::Io { path, cause }) => {
                write!(f, "output: cannot write {}: {cause}", path.display())
            }
            ImgError::Output(OutputError::Encode {
                path,
                format,
                cause,
            }) => write!(
                f,
                "output: cannot encode {} as {}: {}",
                path.display(),
                format.name(),
                flatten(cause)
            ),
            ImgError::Output(OutputError::SizeLimit {
                path,
                format,
                violation,
            }) => {
                write!(
                    f,
                    "output: cannot produce {} as {}: ",
                    path.display(),
                    format.name()
                )?;
                write_violation(f, violation)
            }
            ImgError::Output(OutputError::OutOfSpace {
                path,
                format,
                needed,
                available,
            }) => {
                write!(
                    f,
                    "output: not enough space for {} as {}: ",
                    path.display(),
                    format.name()
                )?;
                write!(
                    f,
                    "needs about {} but the destination volume has {} free",
                    human_bytes(*needed),
                    human_bytes(*available)
                )
            }
        }
    }
}

/// Render the shared "exceeded N (limit L from BINDING)" phrasing.
fn write_violation(f: &mut Formatter<'_>, violation: &LimitViolation) -> fmt::Result {
    let measured = match violation.kind {
        ViolationKind::Width => "width",
        ViolationKind::Height => "height",
        ViolationKind::Pixels => "pixel count",
        ViolationKind::Bytes => "estimated size",
    };

    let (actual, allowed) = if violation.kind == ViolationKind::Bytes {
        (
            human_bytes(violation.actual),
            human_bytes(violation.allowed),
        )
    } else {
        (
            human_count(violation.actual),
            human_count(violation.allowed),
        )
    };

    write!(
        f,
        "{measured} {actual} exceeds the limit of {allowed} (from {})",
        violation.binding.describe()
    )
}

/// Collapse the trailing newline `ImageErrors` bakes into every `Debug`/`Display`.
///
/// `ImageErrors`' `Debug` writes `writeln!` at every level, so the nested
/// variants (encode errors, operation errors) end up with more than one
/// trailing newline. Trimming all trailing whitespace is what keeps a log line
/// a single line.
fn flatten(error: &ImageErrors) -> String {
    error.to_string().trim_end().to_string()
}

impl Display for InputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&ImgError::Input(clone_input(self)), f)
    }
}

impl Display for OutputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&ImgError::Output(clone_output(self)), f)
    }
}

impl std::error::Error for ImgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ImgError::Input(InputError::Open { cause, .. }) => Some(cause),
            ImgError::Input(InputError::Decode { cause, .. }) => Some(cause),
            ImgError::Input(
                InputError::UnsupportedFormat { .. }
                | InputError::SizeLimit { .. }
                | InputError::InvalidResize { .. },
            ) => None,
            ImgError::Input(InputError::Configuration { cause, .. }) => Some(cause),
            ImgError::Output(OutputError::Io { cause, .. }) => Some(cause),
            ImgError::Output(OutputError::Encode { cause, .. }) => Some(cause),
            ImgError::Output(OutputError::SizeLimit { .. } | OutputError::OutOfSpace { .. }) => {
                None
            }
        }
    }
}

/// Classify something that went wrong while reading `path`.
///
/// `format` is resolved from the file extension, so the message can name it
/// even when the decoder could not get far enough to report one. Every
/// [`ImageErrors`] variant maps onto a structured input failure — unknown
/// ones become [`InputError::Decode`] with the original text preserved — so
/// the result is the error itself, not an `Option`.
pub fn classify_input(
    path: &Path,
    error: &ImageErrors,
    resize: Option<(&'static str, (u64, u64))>,
) -> ImgError {
    let format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(ImageFormatId::from_extension)
        .unwrap_or(ImageFormatId::Other);

    match error {
        ImageErrors::ImageDecoderNotImplemented(_) => {
            ImgError::Input(InputError::UnsupportedFormat {
                path: path.to_path_buf(),
                format,
                reason: UnsupportedReason::NotImplemented,
            })
        }
        ImageErrors::ImageDecoderNotIncluded(_) => {
            ImgError::Input(InputError::UnsupportedFormat {
                path: path.to_path_buf(),
                format,
                reason: UnsupportedReason::FeatureNotEnabled,
            })
        }
        ImageErrors::ImageOperationNotImplemented(operation, _) if *operation == "resize" => {
            // Without the caller's context there is nothing to say beyond "the
            // resize failed", so fall through to a decode error rather than
            // dropping the failure's format tag and hint.
            match resize {
                Some((reason, requested)) => {
                    ImgError::Input(InputError::InvalidResize { requested, reason })
                }
                None => ImgError::Input(InputError::Decode {
                    path: path.to_path_buf(),
                    format,
                    cause: clone_image_errors(error),
                }),
            }
        }
        other => ImgError::Input(InputError::Decode {
            path: path.to_path_buf(),
            format,
            cause: clone_image_errors(other),
        }),
    }
}

/// Classify something that went wrong while writing `path`.
///
/// The format is inferred from the output path's extension, which is
/// authoritative at output: the encoder selects the extension, so `.jpg` is
/// jpeg, `.png` is png, etc. The underlying `ImageErrors` value does not carry
/// the format, but the path does.
///
/// `ImageErrors::IoError` is routed to [`OutputError::Io`] — directory
/// creation, temp-file allocation, or publish/rename failures that happened
/// during encoding. Everything else is an encoder failure →
/// [`OutputError::Encode`].
pub fn classify_output(path: &Path, error: &ImageErrors) -> Option<ImgError> {
    let format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(ImageFormatId::from_extension)
        .unwrap_or(ImageFormatId::Other);

    let error_val = match error {
        ImageErrors::IoError(io) => ImgError::Output(OutputError::Io {
            path: path.to_path_buf(),
            cause: std::io::Error::new(io.kind(), io.to_string()),
        }),
        _ => ImgError::Output(OutputError::Encode {
            path: path.to_path_buf(),
            format,
            cause: clone_image_errors(error),
        }),
    };

    Some(error_val)
}

/// Build an encode failure from a known encoder name.
///
/// Used at call sites where the encoder name is known (the CLI subcommand),
/// so the format tag is authoritative rather than inferred from the path
/// extension. The path extension and the encoder name usually agree, but
/// Supported encoder names identify the format independently of the path and
/// is the more direct source.
pub fn output_encode_error(path: &Path, encoder_name: &str, error: &ImageErrors) -> ImgError {
    ImgError::Output(OutputError::Encode {
        path: path.to_path_buf(),
        format: ImageFormatId::from_encoder_name(encoder_name),
        cause: clone_image_errors(error),
    })
}

/// Build an IO failure on the output side.
///
/// Used at call sites that produce `io::Error` directly (directory creation,
/// temp-file allocation, file publishing, metadata writing), so they are
/// reported with the same structured form as encoder failures. The path is the
/// file that was being written when the IO failed.
pub fn output_io_error(path: &Path, error: &std::io::Error) -> ImgError {
    ImgError::Output(OutputError::Io {
        path: path.to_path_buf(),
        cause: std::io::Error::new(error.kind(), error.to_string()),
    })
}

/// Clone an [`InputError`] so it can be re-wrapped for `Display`.
///
/// `ImageErrors` has no `Clone`, so the variants holding one are rebuilt from
/// their rendered text; that is exactly what the message is going to print
/// anyway.
fn clone_input(error: &InputError) -> InputError {
    match error {
        InputError::Open { path, cause } => InputError::Open {
            path: path.clone(),
            cause: std::io::Error::new(cause.kind(), cause.to_string()),
        },
        InputError::Decode {
            path,
            format,
            cause,
        } => InputError::Decode {
            path: path.clone(),
            format: *format,
            cause: clone_image_errors(cause),
        },
        InputError::UnsupportedFormat {
            path,
            format,
            reason,
        } => InputError::UnsupportedFormat {
            path: path.clone(),
            format: *format,
            reason: *reason,
        },
        InputError::SizeLimit {
            path,
            format,
            dimensions,
            violation,
        } => InputError::SizeLimit {
            path: path.clone(),
            format: *format,
            dimensions: *dimensions,
            violation: *violation,
        },
        InputError::InvalidResize { requested, reason } => InputError::InvalidResize {
            requested: *requested,
            reason,
        },
        InputError::Configuration {
            path,
            format,
            cause,
        } => InputError::Configuration {
            path: path.clone(),
            format: *format,
            cause: clone_image_errors(cause),
        },
    }
}

/// Clone an [`OutputError`] so it can be re-wrapped for `Display`.
fn clone_output(error: &OutputError) -> OutputError {
    match error {
        OutputError::Io { path, cause } => OutputError::Io {
            path: path.clone(),
            cause: std::io::Error::new(cause.kind(), cause.to_string()),
        },
        OutputError::Encode {
            path,
            format,
            cause,
        } => OutputError::Encode {
            path: path.clone(),
            format: *format,
            cause: clone_image_errors(cause),
        },
        OutputError::SizeLimit {
            path,
            format,
            violation,
        } => OutputError::SizeLimit {
            path: path.clone(),
            format: *format,
            violation: *violation,
        },
        OutputError::OutOfSpace {
            path,
            format,
            needed,
            available,
        } => OutputError::OutOfSpace {
            path: path.clone(),
            format: *format,
            needed: *needed,
            available: *available,
        },
    }
}

/// Rebuild an [`ImageErrors`] from its rendered text.
fn clone_image_errors(error: &ImageErrors) -> ImageErrors {
    match error {
        ImageErrors::GenericStr(s) => ImageErrors::GenericStr(s),
        other => ImageErrors::GenericString(other.to_string().trim_end().to_string()),
    }
}

/// Bridge for the format-agnostic wrappers that only carry a message.
impl From<InputError> for ImgError {
    fn from(error: InputError) -> Self {
        ImgError::Input(error)
    }
}

impl From<OutputError> for ImgError {
    fn from(error: OutputError) -> Self {
        ImgError::Output(error)
    }
}

#[cfg(test)]
mod tests;
