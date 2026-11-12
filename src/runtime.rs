use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use crate::cli::{
    Arguments,
    pipeline::{decode, operations},
    utils::{
        jpeg::{insert_jpeg_exif_app1, read_jpeg_source_metadata},
        paths::{expand_file_lists, get_paths, paths_equivalent},
        threads,
    },
};
use filetime::FileTime;
use indicatif::{DecimalBytes, MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use little_exif::metadata::Metadata as ExifMetadata;
use rimage::operations::icc::ApplySRGB;
use zune_core::{bit_depth::BitDepth, colorspace::ColorSpace};
use zune_image::{
    core_filters::{colorspace::ColorspaceConv, depth::Depth},
    traits::OperationsTrait,
};
// Only `check_output_limits` names the type, and that whole check is compiled
// out with the feature: without the gate this import is unused in every
// `build-binary` build that leaves `limits` off.
#[cfg(feature = "limits")]
use zune_image::image::Image;
use zune_imageprocs::auto_orient::AutoOrient;

use crate::cli::pipeline::encoder;


/// Report a structured pipeline failure and abandon the current file.
///
/// The side comes from the error itself (`RimageError::direction`), and the
/// message carries the hint the `error` module derived for it.
macro_rules! fail_pipeline {
    ( $state:expr, $e:expr ) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                record_structured_failure(&$state, &e);
                return;
            }
        }
    };
}

/// Refuse the current file for a reason discovered here, and abandon it.
///
/// The bare `return`s that used to sit on these paths logged a message but left
/// no trace on the run, so the process still reported success. Every early exit
/// from the worker must go through this or one of the `fail_*` macros, or the
/// exit code stops describing what happened.
macro_rules! refuse_file {
    ( input, $state:expr, $path:expr, $($arg:tt)* ) => {{
        record_failure(&$state, rimage::exit::ExitCode::Input);
        log::error!($($arg)*);
        let _ = &$path;
        return;
    }};
    ( output, $state:expr, $path:expr, $($arg:tt)* ) => {{
        record_failure(&$state, rimage::exit::ExitCode::Output);
        log::error!($($arg)*);
        let _ = &$path;
        return;
    }};
}

const SUPPORTS_EXIF: &[&str; 7] = &[
    "mozjpeg", "oxipng", "png", "jpeg", "jpeg_xl", "tiff", "webp",
];
const SUPPORTS_ICC: &[&str; 2] = &["mozjpeg", "oxipng"];

struct Result {
    output: PathBuf,
    input_size: u64,
    output_size: u64,
}

struct ProcessingState {
    results: Vec<Result>,
    /// The verdict for the run, accumulated as each file finishes.
    ///
    /// Kept here rather than counted from `results.len()` at the end, because
    /// the number of successes cannot distinguish a failure to read an input
    /// from a failure to write an output — and those exit with different codes.
    run: rimage::exit::RunState,
    /// The side each failure happened on, for the end-of-run summary.
    failures: Vec<rimage::exit::ExitCode>,
}

impl ProcessingState {
    fn new() -> Self {
        Self {
            results: vec![],
            run: rimage::exit::RunState::start(),
            failures: vec![],
        }
    }

    /// Fold the outcome of one file into the run verdict.
    fn record(&mut self, file: rimage::exit::ExitCode) {
        self.run = self.run.record(file);
    }

    /// Record a file that was written.
    ///
    /// Pushing the result and recording the success happen together because
    /// they are the same event: a run that wrote half its files is
    /// [`ExitCode::Partial`](rimage::exit::ExitCode::Partial), and it can only
    /// be recognised as such if every write leaves a trace. Recording the
    /// result alone let the verdict stay at whatever the failures had set, so
    /// a partial run was reported as a clean input or output failure — which
    /// tells a wrapper nothing was written and invites a retry over files that
    /// were already replaced.
    fn record_success(&mut self, result: Result) {
        self.results.push(result);
        self.record(rimage::exit::ExitCode::Success);
    }
}

/// Record a failure on the shared state.
///
/// Called from the worker threads, so it takes the state by reference and locks
/// internally. Only the count and the side are kept: each failure's path was
/// already logged at the point it happened, and repeating the list at the end
/// would bury the summary line the reader is looking for.
fn record_failure(state: &Arc<Mutex<ProcessingState>>, side: rimage::exit::ExitCode) {
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    state.record(side);
    state.failures.push(side);
}

/// Record a failure that already carries its own structured message.
///
/// Used for the pipeline's own [`RimageError`](rimage::error::RimageError)
/// values, which know their side and would otherwise have to be taken apart and
/// reassembled to be logged.
fn record_structured_failure(
    state: &Arc<Mutex<ProcessingState>>,
    error: &rimage::error::RimageError,
) {
    let side = match error.direction() {
        rimage::error::Direction::Input => rimage::exit::ExitCode::Input,
        rimage::error::Direction::Output => rimage::exit::ExitCode::Output,
    };

    {
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        state.record(side);
        state.failures.push(side);
    }

    error.log();
}

/// Limits concurrent image processing to prevent OOM with large images.
///
/// A `Mutex<isize>` + `Condvar` permit counter. Each worker task calls
/// [`acquire`](ConcurrencyLimiter::acquire) at the start of its work; if the
/// counter is at 0 the call blocks. When the returned [`PermitGuard`] is
/// dropped, the permit is returned and a blocked waiter is woken.
#[derive(Clone)]
struct ConcurrencyLimiter {
    inner: Arc<(Mutex<isize>, Condvar)>,
}

impl ConcurrencyLimiter {
    fn new(max: usize) -> Self {
        Self {
            inner: Arc::new((Mutex::new(max as isize), Condvar::new())),
        }
    }

    fn acquire(&self) -> PermitGuard {
        let (ref lock, ref cvar) = *self.inner;
        let mut count = lock.lock().unwrap();
        while *count <= 0 {
            count = cvar.wait(count).unwrap();
        }
        *count -= 1;
        PermitGuard {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct PermitGuard {
    inner: Arc<(Mutex<isize>, Condvar)>,
}

impl Drop for PermitGuard {
    fn drop(&mut self) {
        let (ref lock, ref cvar) = *self.inner;
        let mut count = lock.lock().unwrap();
        *count += 1;
        cvar.notify_one();
    }
}

/// RAII guard that updates progress bars on drop, ensuring they advance
/// even when a worker returns early due to an error.
struct FinishGuard {
    pb: ProgressBar,
    pb_main: ProgressBar,
}

impl Drop for FinishGuard {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
        self.pb_main.inc(1);
    }
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TemporaryOutput {
    path: PathBuf,
    published: bool,
}

impl TemporaryOutput {
    fn new(target: &Path) -> std::io::Result<(Self, File)> {
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let stem = target
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("image");
        let extension = target.extension().and_then(|value| value.to_str());

        for _ in 0..100 {
            let id = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let mut name = format!(".{stem}.rimage-{}-{id}", std::process::id());
            if let Some(extension) = extension {
                name.push('.');
                name.push_str(extension);
            }
            let path = parent.join(name);
            match File::create_new(&path) {
                Ok(file) => {
                    return Ok((
                        Self {
                            path,
                            published: false,
                        },
                        file,
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique temporary output file",
        ))
    }

    fn publish(mut self, target: &Path) -> std::io::Result<()> {
        fs::rename(&self.path, target)?;
        self.published = true;
        Ok(())
    }
}

impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn append_output_extension(path: &mut PathBuf, extension: &str) {
    if let Some(current) = path.extension() {
        let mut combined = current.to_os_string();
        combined.push(".");
        combined.push(extension);
        path.set_extension(combined);
    } else {
        path.set_extension(extension);
    }
}

fn output_path_key(path: &Path) -> String {
    // Fold case in keys on every platform: path comparisons are treated as
    // case-insensitive by default (fail-safe), so collisions between e.g.
    // `img@Backup.png` and `img@backup.png` are detected before publishing
    // over the backup. See `cli::utils::paths::component_eq`.
    path.to_string_lossy().into_owned().to_lowercase()
}

fn size_ratio(output_size: u64, input_size: u64) -> f64 {
    if input_size == 0 {
        0.0
    } else {
        output_size as f64 / input_size as f64
    }
}

/// Refuse to encode `img` when the conversion, or the destination, cannot hold
/// the result.
///
/// Returns the failure wrapped for the caller's side — an unwritable output —
/// because both ceilings describe the write, not the read.
///
/// Two independent ceilings are applied, and they answer different questions:
///
/// 1. *Memory.* The pre-decode screen in `cli::pipeline` runs before the image
///    exists, so it has to assume the widest layout the input format allows
///    and can only guess what the encoder will need. Here the real bit depth,
///    colour space and dimensions are known, and the encoder's own buffers —
///    which dominate for `avif` and `oxipng` — have not been allocated yet.
///    Checking now is what turns an allocation abort inside a codec into a
///    message naming the ceiling, and it is also the only place the *output*
///    format's dimension caps are enforced.
/// 2. *Disk.* The volume has to hold what the encoder writes.
///
/// `concurrency` divides the memory budget for the same reason it does in the
/// pre-decode screen: every image in flight holds its own buffers.
#[cfg(feature = "limits")]
fn check_output_limits(
    output: &Path,
    img: &Image,
    format: rimage::limits::ImageFormatId,
    concurrency: usize,
) -> std::result::Result<(), rimage::error::RimageError> {
    use rimage::limits::{LimitSet, PipelineCost, SystemBudget, bytes_per_pixel};

    let budget = SystemBudget::probe(concurrency);
    let limits = LimitSet::for_output(
        format,
        img.depth(),
        img.colorspace(),
        &budget,
        PipelineCost::for_encoder(format),
        output,
    );

    let (width, height) = img.dimensions();
    let width = width as u64;
    let height = height as u64;

    limits
        .check(width, height)
        .map_err(|violation| rimage::error::output_size_limit(output, format, violation))?;

    // The byte ceiling only describes *this volume* when free space was the
    // binding constraint. Otherwise it is the memory budget in disguise, and
    // rejecting on it would duplicate the check above under a message about
    // disk space.
    if limits.bytes_binding != rimage::limits::Binding::Disk {
        return Ok(());
    }

    // The estimate deliberately uses the *decoded* footprint rather than a
    // compressed-size guess: it is the one number this code can know without
    // encoding first, and it is an upper bound for lossy formats and the right
    // order of magnitude for lossless ones, which is what a pre-flight check
    // needs. A volume with room for the decoded pixels will hold any sane
    // encoding of them.
    let footprint = width
        .saturating_mul(height)
        .saturating_mul(bytes_per_pixel(img.depth(), img.colorspace()));

    limits
        .check_bytes(footprint)
        .map_err(|violation| rimage::error::output_size_limit(output, format, violation))
}

#[cfg(feature = "limits")]
fn print_limits(subcommand: &str, threads: usize) -> std::process::ExitCode {
    use rimage::error::human_bytes;
    use rimage::limits::{ImageFormatId, LimitSet, PipelineCost, SystemBudget, bytes_per_pixel};

    let format = ImageFormatId::from_encoder_name(subcommand);
    let budget = SystemBudget::probe(threads);
    let cost = PipelineCost::for_encoder(format);
    let depth = zune_core::bit_depth::BitDepth::Eight;
    let colorspace = zune_core::colorspace::ColorSpace::RGBA;
    let bpp = bytes_per_pixel(depth, colorspace);

    println!("rimage runtime limits");
    println!("─────────────────────");
    println!();
    println!("Encoder:    {subcommand} ({})", format.name());
    println!("Concurrency: {threads} image(s) at once");
    println!();
    println!("System budget");
    println!(
        "  Available memory:    {}",
        human_bytes(budget.available_memory)
    );
    println!("  Address space cap:   {}", human_bytes(budget.address_cap));
    println!(
        "  Probe status:        {}",
        if budget.is_probed() {
            "ok"
        } else {
            "fallback (512 MiB)"
        }
    );
    println!(
        "  Per-image bytes:     {}",
        human_bytes(budget.per_image_bytes())
    );
    println!();

    let caps = rimage::limits::format_caps(format);
    println!("Format caps ({})", caps.source);
    println!("  Max side:   {}", caps.max_side);
    let max_pixels_str = if caps.max_pixels == u64::MAX {
        "unbounded".to_string()
    } else {
        caps.max_pixels.to_string()
    };
    println!("  Max pixels: {max_pixels_str}");
    println!();

    println!("Pipeline cost");
    println!("  Decode:   {}×", cost.decode);
    println!("  Resize:   {}×", cost.resize);
    println!("  Quantize: {}×", cost.quantize);
    println!("  Encode:   {}×", cost.encode);
    println!(
        "  Total:    {}× (× {} B/px = {} B/px)",
        cost.total(),
        bpp,
        cost.total() * bpp
    );
    println!();

    let limits = LimitSet::for_input(format, depth, colorspace, &budget, cost);
    println!("Effective input limits");
    println!("  Max width:   {}", limits.max_width);
    println!("  Max height:  {}", limits.max_height);
    println!(
        "  Max pixels:  {} (≈ {}²)",
        limits.max_pixels,
        (limits.max_pixels as f64).sqrt() as u64
    );
    println!("  Max bytes:   {}", human_bytes(limits.max_bytes));
    println!("  Binding:     {}", binding_str(limits.binding));
    println!("  Byte binding: {}", binding_str(limits.bytes_binding));
    println!();
    println!("  Suggested --resize side: {}", limits.suggested_side());

    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "limits"))]
fn print_limits(_subcommand: &str, _threads: usize) -> std::process::ExitCode {
    eprintln!("--print-limits requires the 'limits' feature to be enabled at build time.");
    eprintln!("Rebuild with: cargo b -r --features limits");
    std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8())
}

#[cfg(feature = "limits")]
fn binding_str(binding: rimage::limits::Binding) -> &'static str {
    use rimage::limits::Binding;
    match binding {
        Binding::Format => "format limit",
        Binding::Memory => "memory budget",
        Binding::Disk => "disk free space",
        Binding::None => "none",
    }
}

/// Normalize a user-provided file path into its canonical absolute form.
///
/// Handles:
/// - Expanding `~` to the home directory
/// - Resolving relative paths (`.`, `..`) against the current directory
///   using component-level joining — avoids `Path::join` which leaves
///   `./` and `../` literals in the joined result
/// - Canonicalizing existing paths (resolves symlinks, case, remaining `..`)
///
/// This is used for output directories. Input files are
/// normalized separately without canonicalization so symlinked directory
/// layouts and the user-visible path are preserved.
fn normalize_path(path: &Path, current_dir: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return path.to_path_buf();
    }

    // Expand ~ in the path string
    let path = expand_tilde_in_path(path);

    let is_absolute = path.is_absolute();

    let path = if is_absolute {
        path
    } else {
        join_normalized(current_dir, &path)
    };

    // Canonicalize if the path exists (resolves symlinks, remaining .., case).
    // Non-existent paths (output dirs) keep the component-normalized form.
    path.canonicalize().unwrap_or(path)
}

/// Join a base path with a relative path, normalizing `.` and `..` components.
///
/// Unlike `Path::join`, this resolves `./foo` to `base/foo` instead of
/// `base/./foo`, and correctly handles `../` by popping the parent.
fn join_normalized(base: &Path, relative: &Path) -> PathBuf {
    let mut result = base.to_path_buf();
    for c in relative.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => result.push(other),
        }
    }
    result
}

/// Expand a leading `~` in a path to the user's home directory.
///
/// Falls back to the original path if the env var is unset.
fn expand_tilde_in_path(path: &Path) -> PathBuf {
    let s = match path.to_str() {
        Some(s) => s,
        None => return path.to_path_buf(),
    };

    if !s.starts_with('~') {
        return path.to_path_buf();
    }

    let home_dir = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return path.to_path_buf(),
    };

    if s == "~" {
        return home_dir;
    }

    // "~/..." → home_dir/...
    let after_tilde = &s[1..]; // skip '~'
    let trimmed = after_tilde.trim_start_matches('/');
    if trimmed.len() < after_tilde.len() {
        join_normalized(&home_dir, Path::new(trimmed))
    } else {
        // "~something" is not a home directory reference
        path.to_path_buf()
    }
}

/// Creates the target directory and verifies that existing symlinks/reparse
/// points have not redirected it outside the requested output root.
fn prepare_output_parent(output: &Path, output_root: Option<&Path>) -> std::io::Result<()> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let Some(root) = output_root else {
        return Ok(());
    };

    fs::create_dir_all(root)?;
    let canonical_root = root.canonicalize()?;
    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "output directory {} resolves outside {}",
                parent.display(),
                root.display()
            ),
        ));
    }
    Ok(())
}

/// Computes the `--backup` destination for an input file: `<stem>@backup.<ext>`
/// next to the input.
fn compute_backup_path(input: &Path) -> PathBuf {
    let mut backup_name = input
        .file_stem()
        .unwrap_or_else(|| OsStr::new("backup"))
        .to_os_string();
    backup_name.push("@backup.");
    backup_name.push(input.extension().unwrap_or_else(|| OsStr::new("bak")));
    input.with_file_name(&backup_name)
}

/// Closes and removes an unfinished backup destination on drop unless the backup was fully written (see [`create_backup`]).
/// Without this, a failed copy or flush would strand a partial `<stem>@backup.<ext>` file, and every later `--backup` run would refuse to process the input because the destination already exists.
struct BackupFileGuard {
    path: PathBuf,
    file: Option<File>,
    committed: bool,
}

impl Drop for BackupFileGuard {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(file) = self.file.take() {
            drop(file);
        }
        let _ = fs::remove_file(&self.path);
    }
}

/// Creates a backup of `input` at `backup_path` without overwriting an existing destination.
/// Prefers a hard link; falls back to copying the bytes for filesystems without hard link support.
/// `fs::copy` is deliberately not used because it truncates an existing destination, which could destroy the original image preserved by an earlier run.
fn create_backup(input: &Path, backup_path: &Path) -> std::io::Result<()> {
    match fs::hard_link(input, backup_path) {
        Ok(()) => return Ok(()),
        Err(error) => {
            log::debug!(
                "{}: hard link backup failed, falling back to copy: {error}",
                input.display()
            );
        }
    }

    let mut source = fs::File::open(input)?;
    let mut guard = BackupFileGuard {
        path: backup_path.to_path_buf(),
        file: Some(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(backup_path)?,
        ),
        committed: false,
    };
    let destination = guard.file.as_mut().expect("backup file is present");
    std::io::copy(&mut source, destination)?;
    destination.flush()?;
    guard.committed = true;
    Ok(())
}

pub(crate) fn run(arguments: Arguments) -> std::process::ExitCode {
    let logger = env_logger::Builder::new()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .filter_module("little_exif", log::LevelFilter::Off)
        .format_timestamp(None)
        .build();
    let level = logger.filter();

    let multi = MultiProgress::new();
    let sty_main = ProgressStyle::with_template("{bar:40} {pos:>4}/{len:4}")
        .unwrap()
        .progress_chars("▬▬▬");
    let sty_aux_decode = ProgressStyle::with_template("{spinner} {msg}").unwrap();
    let sty_aux_operations = ProgressStyle::with_template("{spinner} {msg}").unwrap();
    let sty_aux_encode = ProgressStyle::with_template("{spinner} {msg}").unwrap();

    // try_init may fail if a logger is already installed (e.g. in test
    // harnesses). Discard the error rather than panicking: the worst case
    // is that our log filter is not applied, which is non-fatal.
    let _ = LogWrapper::new(multi.clone(), logger).try_init();
    log::set_max_level(level);

    let current_dir = std::env::current_dir().unwrap_or_else(|error| {
        // current_dir fails when the CWD was deleted or is inaccessible.
        // An empty PathBuf causes subsequent path joins to produce relative
        // paths, which at least does not crash; log the cause so the user
        // can diagnose the real problem.
        log::error!("Cannot determine current directory: {error}; using relative paths");
        PathBuf::new()
    });
    let subcommand = arguments.command;
    let matches = &arguments;

    let state: Arc<Mutex<ProcessingState>> = Arc::new(Mutex::new(ProcessingState::new()));

    {
            // Clamped here, and reported once, so the number the pool is built
            // with is the same one the memory budget divides by.
            let requested_threads = threads::requested(matches.threads);
            let threads = threads::clamp(requested_threads);
            if threads != requested_threads {
                log::warn!(
                    "--threads {requested_threads} is more than this machine can run in parallel; \
                     using {threads} instead"
                );
            }
            // What every input in this run is being turned into. The memory
            // budget depends on it, because the encoder's scratch buffers are
            // the largest term in it and their number is a property of the
            // encoder, not of the file being read.
            let target_format = rimage::limits::ImageFormatId::from_encoder_name(subcommand);

            // Hidden diagnostic: print the runtime-derived limits and exit
            // before touching any files. Used to understand why an image was
            // rejected and to calibrate the pipeline cost estimates.
            if matches.print_limits {
                return print_limits(subcommand, threads);
            }

            let thread_pool = match rayon::ThreadPoolBuilder::new().num_threads(threads).build() {
                Ok(pool) => pool,
                Err(error) => {
                    log::error!("Failed to create image worker pool: {error}");
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
            };

            // Keep normalized absolute paths but do not canonicalize existing
            // inputs here: validation follows symlinks while preserving their
            // user-visible path and recursive directory layout.
            let normalize_input = |path: &Path| {
                let expanded = expand_tilde_in_path(path);
                if expanded.is_absolute() {
                    join_normalized(Path::new(""), &expanded)
                } else {
                    join_normalized(&current_dir, &expanded)
                }
            };
            let inputs: Vec<PathBuf> = matches.files.iter()
                .map(|path| normalize_input(path))
                .collect();
            let files = match expand_file_lists(&inputs, normalize_input) {
                Ok(files) => files,
                Err(error) => {
                    log::error!("{error}");
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
            };
            log::debug!("Resolved files: {files:#?}");

            let out_dir = matches.directory.as_ref()
                .map(|p| normalize_path(p, &current_dir));
            let output_root = out_dir.clone();

            let recursive = matches.recursive;
            let backup = matches.backup;
            let strip_metadata = matches.strip;
            let preserve_timestamps = matches.preserve_timestamps;
            let quiet = matches.quiet;
            let no_progress = matches.no_progress;
            let suffix = matches.suffix.clone();

            if quiet || no_progress {
                multi.set_draw_target(ProgressDrawTarget::hidden());
            }

            let output_extension = match encoder(subcommand, matches) {
                Ok(encoder) => encoder.to_extension(),
                Err(error) => {
                    log::error!("Failed to initialize encoder: {error}");
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
            };
            let paths = match get_paths(files, out_dir, suffix, recursive) {
                Ok(paths) if !paths.is_empty() => paths
                    .into_iter()
                    .map(|(input, mut output)| {
                        append_output_extension(&mut output, output_extension);
                        (input, output)
                    })
                    .collect::<Vec<_>>(),
                Ok(_) => {
                    log::error!("No input files found. Check the file paths.");
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
                Err(error) => {
                    log::error!("{error}");
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
            };
            let file_count = paths.len() as u64;
            let in_flight = threads::in_flight(requested_threads, paths.len());

            let pb_main = multi.add(ProgressBar::new(file_count));
            pb_main.set_style(sty_main);
            if file_count <= 1 {
                pb_main.set_draw_target(ProgressDrawTarget::hidden());
            }

            let input_paths = paths
                .iter()
                .map(|(input, _)| output_path_key(input))
                .collect::<HashSet<_>>();
            let mut output_paths = HashSet::with_capacity(paths.len());
            for (input, output) in &paths {
                let input_key = output_path_key(input);
                let output_key = output_path_key(output);
                if !output_paths.insert(output_key.clone()) {
                    log::error!(
                        "Multiple input files resolve to the same output path: {}",
                        output.display()
                    );
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
                if input_key != output_key && input_paths.contains(&output_key) {
                    log::error!(
                        "Output path would overwrite another input file: {}",
                        output.display()
                    );
                    return std::process::ExitCode::from(rimage::exit::ExitCode::Usage.as_u8());
                }
            }
            let limiter = ConcurrencyLimiter::new(threads);
            thread_pool.install(|| {
                rayon::scope(|s| {
                    for (input, output) in paths {
                    let limiter = limiter.clone();
                    let pb_main = pb_main.clone();
                    let multi = multi.clone();
                    let sty_aux_decode = sty_aux_decode.clone();
                    let sty_aux_operations = sty_aux_operations.clone();
                    let sty_aux_encode = sty_aux_encode.clone();
                    let state = Arc::clone(&state);
                    let output_root = output_root.clone();
                    s.spawn(move |_| {
                        // Acquire the permit on the worker itself: rayon schedules the
                        // scope closure onto a pool worker, so blocking the caller would
                        // starve the already-spawned tasks in a single-threaded pool
                        // (deadlock). Acquiring before adding the progress bar also keeps
                        // the number of on-screen spinners bounded by `threads`.
                        let _permit = limiter.acquire();

                        let pb = multi.add(ProgressBar::new_spinner());
                        pb.set_style(sty_aux_decode.clone());
                        pb.set_message(format!("{}", input.display()));
                        pb.enable_steady_tick(Duration::from_millis(100));

                        // Advance progress bars on all exit paths (including the
                        // early returns from the `fail_pipeline!` calls below).
                        let _finish = FinishGuard {
                            pb: pb.clone(),
                            pb_main: pb_main.clone(),
                        };

                        // A same-format, in-place conversion can produce an
                        // output path identical to the --backup path (e.g.
                        // `-b -s @backup` on `img.png`). Publishing the temp
                        // output would then overwrite the backup holding the
                        // original image, silently destroying it. Detect the
                        // collision before any decoding or rename happens.
                        let backup_path = backup.then(|| compute_backup_path(&input));
                        if let Some(backup_path) = &backup_path
                            && paths_equivalent(&output, backup_path)
                        {
                            refuse_file!(
                                input,
                                state,
                                input,
                                "{}: output path {} is the same as the --backup path {}; \
                                 use a different --suffix or drop --backup",
                                input.display(),
                                output.display(),
                                backup_path.display()
                            );
                        }
                        // A backup left by an earlier run preserves the
                        // original image; refuse to overwrite or delete it.
                        // Fail fast instead of encoding the image first.
                        if let Some(backup_path) = &backup_path {
                            match fs::symlink_metadata(backup_path) {
                                Ok(_) => {
                                    refuse_file!(
                                        input,
                                        state,
                                        input,
                                        "{}: --backup destination already exists: {}; \
                                         refusing to overwrite it",
                                        input.display(),
                                        backup_path.display()
                                    );
                                }
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                                Err(error) => {
                                    refuse_file!(
                                        input,
                                        state,
                                        input,
                                        "{}: cannot inspect --backup destination {}: {error}",
                                        input.display(),
                                        backup_path.display()
                                    );
                                }
                            }
                        }

                        let mut ops: Vec<Box<dyn OperationsTrait>> = Vec::new();

                        let input_metadata = fail_pipeline!(state, input
                            .metadata()
                            .map_err(|e| rimage::error::input_open_error(&input, &e)));
                        let input_size = input_metadata.len();
                        let preserved_mtime = preserve_timestamps
                            .then(|| FileTime::from_last_modification_time(&input_metadata));

                        let mut img = fail_pipeline!(state, decode(&input, matches, target_format, in_flight));

                        // Preserve JPEG metadata directly from the source file.
                        // EXIF is copied as a raw APP1 segment instead of being
                        // decoded and re-encoded: little_exif rejects valid JPEGs
                        // that store ExifVersion as STRING rather than UNDEF.
                        let jpeg_source_metadata = if strip_metadata {
                            None
                        } else {
                            match read_jpeg_source_metadata(&input) {
                                Ok(metadata) => metadata,
                                Err(error) => {
                                    log::warn!(
                                        "{}: failed to read source JPEG metadata: {error}",
                                        input.display()
                                    );
                                    None
                                }
                            }
                        };

                        let (jfif_density, raw_exif_app1, source_is_jpeg) =
                            match jpeg_source_metadata {
                                Some(metadata) => (
                                    metadata.jfif_density,
                                    metadata.exif_app1,
                                    true,
                                ),
                                None => (None, None, false),
                            };

                        let exif_metadata: Option<ExifMetadata> = if source_is_jpeg
                            || strip_metadata
                            || !SUPPORTS_EXIF.contains(&subcommand)
                        {
                            None
                        } else {
                            match ExifMetadata::new_from_path(&input) {
                                Ok(metadata) => Some(metadata),
                                Err(error)
                                    if error
                                        .to_string()
                                        .contains("No EXIF data found") =>
                                {
                                    None
                                }
                                Err(error) => {
                                    log::warn!(
                                        "{}: failed to read EXIF metadata: {error}",
                                        input.display()
                                    );
                                    None
                                }
                            }
                        };

                        pb.set_style(sty_aux_operations.clone());

                        let mut available_encoder =
                            fail_pipeline!(state, encoder(subcommand, matches)
                                .map_err(|e| rimage::error::input_config_error(&input, subcommand, &e)));
                        let output_format = available_encoder.to_extension().to_string();

                        available_encoder.set_jfif_density(jfif_density);

                        if strip_metadata || !SUPPORTS_ICC.contains(&subcommand) {
                            ops.push(Box::new(ApplySRGB));
                        }

                        if strip_metadata || !SUPPORTS_EXIF.contains(&subcommand) {
                            ops.push(Box::new(AutoOrient));
                        }

                        operations(matches, &img)
                            .into_iter()
                            .for_each(|(_, operations)| match operations.name() {
                                "quantize" => {
                                    ops.push(Box::new(Depth::new(BitDepth::Eight)));
                                    ops.push(Box::new(ColorspaceConv::new(ColorSpace::RGBA)));
                                    ops.push(operations);
                                }
                                _ => {
                                    ops.push(operations);
                                }
                            });

                        for op in ops {
                            fail_pipeline!(state, op
                                .execute(&mut img)
                                .map_err(|e| rimage::error::input_operation_error(&input, &e)));
                        }

                        pb.set_style(sty_aux_encode.clone());

                        // Reject an output the machine or the destination
                        // volume cannot hold *before* encoding into a
                        // temporary file. The encode itself is the expensive
                        // part: it is where the codec allocates its scratch
                        // buffers, and a volume that fills up mid-write leaves
                        // a truncated temporary behind.
                        #[cfg(feature = "limits")]
                        fail_pipeline!(state, check_output_limits(
                            &output, &img, target_format, in_flight
                        ));

                        fail_pipeline!(state, prepare_output_parent(&output, output_root.as_deref())
                            .map_err(|e| rimage::error::output_io_error(&output, &e)));
                        let (temporary, output_file) =
                            fail_pipeline!(state, TemporaryOutput::new(&output)
                                .map_err(|e| rimage::error::output_io_error(&output, &e)));

                        fail_pipeline!(state, available_encoder
                            .encode(&img, output_file)
                            .map_err(|e| rimage::error::output_encode_error(&output, subcommand, &e)));

                        if output_format == "jpg"
                            && let Some(raw_exif) = raw_exif_app1
                        {
                            fail_pipeline!(state, insert_jpeg_exif_app1(&temporary.path, &raw_exif)
                                .map_err(|e| rimage::error::output_io_error(&temporary.path, &e)));
                        }

                        if let Some(actual_metadata) = exif_metadata {
                            fail_pipeline!(state, actual_metadata
                                .write_to_file(&temporary.path)
                                .map_err(|e| rimage::error::output_io_error(&temporary.path, &e)));
                        }

                        // Stamp the temporary file rather than the published one: the
                        // rename that publishes it carries the timestamps along, so the
                        // output is never observable with the wrong time on it. This
                        // has to happen after the EXIF and APP1 rewrites above, which
                        // each touch the file and would push the time back to now.
                        //
                        // A volume that refuses the timestamp abandons the file, the
                        // same way a failed EXIF write does: publishing anyway would
                        // hand back an output the user did not ask for and report it as
                        // a success, while abandoning it leaves their original untouched.
                        if let Some(mtime) = preserved_mtime {
                            fail_pipeline!(state, filetime::set_file_mtime(&temporary.path, mtime)
                                .map_err(|e| rimage::error::output_io_error(&output, &e)));
                        }

                        if let Some(backup_path) = backup_path.as_deref() {
                            if let Err(error) = create_backup(&input, backup_path) {
                                refuse_file!(output, state, output, "{}: {error}", input.display());
                            }
                            if let Err(error) = temporary.publish(&output) {
                                if let Err(cleanup_error) = fs::remove_file(backup_path) {
                                    refuse_file!(
                                        output,
                                        state,
                                        output,
                                        "{}: publish failed ({error}); removing the new backup also failed: {cleanup_error}",
                                        output.display()
                                    );
                                } else {
                                    refuse_file!(output, state, output, "{}: {error}", output.display());
                                }
                            }
                            if output_path_key(&input) != output_path_key(&output)
                                && let Err(error) = fs::remove_file(&input)
                            {
                                refuse_file!(
                                    output,
                                    state,
                                    output,
                                    "{}: output was published and backup created, but the original input could not be removed: {error}",
                                    input.display()
                                );
                            }
                        } else {
                            fail_pipeline!(state, temporary
                                .publish(&output)
                                .map_err(|e| rimage::error::output_io_error(&output, &e)));
                        }

                        let output_size = fail_pipeline!(state, output
                            .metadata()
                            .map_err(|e| rimage::error::output_io_error(&output, &e))).len();
                        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());

                        state.record_success(Result {
                            output: output.to_path_buf(),
                            input_size,
                            output_size,
                        });
                    });

                }
            });
            });

            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());

            state
                .results
                .sort_by_key(|b| std::cmp::Reverse(b.output_size));

            let path_width = state
                .results
                .iter()
                .map(|r| r.output.display().to_string().len())
                .max()
                .unwrap_or(0);

            if !quiet {
                let mut stdout = std::io::stdout().lock();

                if state.results.len() > 1 {
                    let _ = writeln!(stdout,
                        "{:<path_width$} {}",
                        "File",
                        "Size",
                    );

                    for result in state.results.iter() {
                        let difference = size_ratio(result.output_size, result.input_size) * 100.0;

                        let _ = writeln!(stdout,
                            "{:<path_width$} {} > {} {}",
                            result.output.display(),
                            DecimalBytes(result.input_size),
                            DecimalBytes(result.output_size),
                            format_args!("{:.2}%", difference - 100.0),
                        );
                    }
                }

                let total_input_size = state.results.iter().fold(0u64, |total, result| {
                    total.saturating_add(result.input_size)
                });
                let total_output_size = state.results.iter().fold(0u64, |total, result| {
                    total.saturating_add(result.output_size)
                });

                if !state.results.is_empty() {
                    let difference = size_ratio(total_output_size, total_input_size) * 100.0;

                    if let Err(error) = writeln!(stdout,
                        "Total: {} > {} {}",
                        DecimalBytes(total_input_size),
                        DecimalBytes(total_output_size),
                        format_args!("{:.2}%", difference - 100.0),
                    ) {
                        log::error!("Failed to write output summary: {error}");
                    }
                }
            }

            let failed = state.failures.len() as u64;
            if failed > 0 {
                // Split the count by side so the summary says which half of the
                // pipeline the failures were in, matching the exit code.
                let input_failures = state
                    .failures
                    .iter()
                    .filter(|side| **side == rimage::exit::ExitCode::Input)
                    .count() as u64;
                let output_failures = failed - input_failures;

                log::error!(
                    "{failed}/{file_count} file(s) failed ({input_failures} reading, \
                     {output_failures} writing). Run with `RUST_LOG=debug` for details."
                );
            }

            std::process::ExitCode::from(state.run.exit_code().as_u8())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_base() -> PathBuf {
        // Use the crate root (CARGO_MANIFEST_DIR) instead of a hardcoded
        // path that only works on one developer's machine.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn test_base_src() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    fn written(input_size: u64, output_size: u64) -> Result {
        Result {
            output: PathBuf::from("a.jpg"),
            input_size,
            output_size,
        }
    }

    /// A written file must leave a trace on the verdict.
    ///
    /// This is the whole reason the verdict is tracked separately from the
    /// result list: a run that wrote some files and lost others is
    /// `Partial`, and it can only be recognised if every write is recorded.
    #[test]
    fn a_written_file_is_recorded_as_a_success() {
        let mut state = ProcessingState::new();
        state.record_success(written(100, 50));

        assert_eq!(state.run, rimage::exit::RunState::Succeeded);
        assert_eq!(state.results.len(), 1);
    }

    /// A success next to a failure is the case [`rimage::exit::ExitCode::Partial`]
    /// exists for: the user's files were already modified, so a wrapper must
    /// not treat the run as a clean no-op and retry it.
    #[test]
    fn a_run_that_wrote_one_file_and_lost_another_is_partial() {
        for failure in [
            rimage::exit::ExitCode::Input,
            rimage::exit::ExitCode::Output,
        ] {
            let mut state = ProcessingState::new();
            state.record_success(written(100, 50));
            state.record(failure);

            assert_eq!(
                state.run.exit_code(),
                rimage::exit::ExitCode::Partial,
                "a success plus {failure} must not be reported as a clean failure"
            );
        }
    }

    #[test]
    fn join_normalized_strips_curdir() {
        let base = test_base();
        let rel = Path::new("./1.jpg");
        let result = join_normalized(&base, rel);
        assert_eq!(result, base.join("1.jpg"));
    }

    #[test]
    fn join_normalized_resolves_parentdir() {
        let base = test_base_src();
        let rel = Path::new("../tests/1.jpg");
        let result = join_normalized(&base, rel);
        let expected = base.parent().unwrap().join("tests/1.jpg");
        assert_eq!(result, expected);
    }

    #[test]
    fn join_normalized_handles_deep_path() {
        let base = test_base();
        let rel = Path::new("subdir/./other/../1.jpg");
        let result = join_normalized(&base, rel);
        let expected = base.join("subdir").join("1.jpg");
        assert_eq!(result, expected);
    }

    #[test]
    fn output_path_key_folds_case_on_all_platforms() {
        let upper = output_path_key(Path::new("/Img@Backup.PNG"));
        let lower = output_path_key(Path::new("/img@backup.png"));
        assert_eq!(upper, lower);
    }

    #[cfg(unix)]
    #[test]
    fn failed_backup_copy_removes_partial_destination() {
        let root = std::env::temp_dir().join(format!(
            "rimage-backup-cleanup-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();

        // A directory opens successfully on Unix but fails during `io::copy`,
        // which exercises the failure path between `create_new` and commit.
        let input = root.join("not-a-file");
        fs::create_dir_all(&input).unwrap();
        let backup = root.join("backup.png");

        let result = create_backup(&input, &backup);

        assert!(result.is_err(), "copying a directory as a file must fail");
        assert!(!backup.exists(), "partial backup must be cleaned up");
        fs::remove_dir_all(&root).unwrap();
    }
}
