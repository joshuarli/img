//! Atomic no-clobber creation and explicitly authorized source replacement.

use std::{ffi::{CString, OsStr}, fs::{File, Metadata}, io::{self, Read, Write}, os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::MetadataExt}}, path::{Component, Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

#[cfg(target_os="macos")]
mod macos;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn cstring(value: &OsStr) -> io::Result<CString> {
    CString::new(value.as_bytes()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"))
}

fn open_at(parent: &File, name: &OsStr, flags: i32) -> io::Result<File> {
    let name = cstring(name)?;
    // SAFETY: parent is live, name is NUL-terminated, and the returned fd is uniquely owned.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags | libc::O_CLOEXEC | libc::O_NOFOLLOW, 0o600) };
    if fd < 0 { return Err(io::Error::last_os_error()); }
    // SAFETY: openat returned a new valid descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn directory(path: &Path) -> io::Result<File> {
    let mut dir = File::open(if path.is_absolute() { "/" } else { "." })?;
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {},
            Component::Normal(name) => { dir = open_at(&dir, name, libc::O_RDONLY | libc::O_DIRECTORY)?; },
            Component::ParentDir => { dir = open_at(&dir, OsStr::new(".."), libc::O_RDONLY | libc::O_DIRECTORY)?; },
            _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsupported path component")),
        }
    }
    Ok(dir)
}

fn split(path: &Path) -> io::Result<(File, CString)> {
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no filename"))?;
    Ok((directory(path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")))?, cstring(name)?))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Identity { device: u64, inode: u64, size: u64, modified: (i64, i64), changed: (i64, i64) }

impl Identity {
    fn from(metadata: Metadata) -> io::Result<Self> {
        if !metadata.is_file() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "input is not a regular file")); }
        Ok(Self { device: metadata.dev(), inode: metadata.ino(), size: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()), changed: (metadata.ctime(), metadata.ctime_nsec()) })
    }
}

/// An opened regular source. Symlinks in any component are rejected.
pub struct Source {
    file: File,
    dir: File,
    name: CString,
    identity: Identity,
}

impl Source {
    /// Open a regular source and retain its identity for replacement checks.
    pub fn open(path: &Path) -> io::Result<Self> {
        let (dir, name) = split(path)?;
        let file = open_at(&dir, OsStr::from_bytes(name.as_bytes()), libc::O_RDONLY | libc::O_NONBLOCK)?;
        let identity = Identity::from(file.metadata()?)?;
        Ok(Self { file, dir, name, identity })
    }

    /// Device and inode identity, without converting paths to strings.
    pub fn file_id(&self) -> (u64, u64) { (self.identity.device, self.identity.inode) }
    /// Metadata from the opened source.
    pub fn metadata(&self) -> io::Result<Metadata> { self.file.metadata() }

    fn verify(&self) -> io::Result<()> {
        let current = open_at(&self.dir, OsStr::from_bytes(self.name.as_bytes()), libc::O_RDONLY | libc::O_NONBLOCK)?;
        if Identity::from(current.metadata()?)? != self.identity || Identity::from(self.file.metadata()?)? != self.identity {
            return Err(io::Error::other("source changed before replacement"));
        }
        Ok(())
    }
}

impl Read for Source {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> { self.file.read(bytes) }
}

/// Publication authority. Replacement requires the originally opened source.
pub enum Destination<'a> {
    /// Atomically create a previously absent destination.
    New(&'a Path),
    /// Replace after a best-effort identity check; concurrent writers are not excluded.
    Replace(&'a Source),
}

/// A failed publication distinguishes committed output from unpublished work.
#[derive(Debug)]
pub struct PublishError {
    committed: bool,
    cause: io::Error,
}

impl PublishError {
    /// Whether the output was published before a later cleanup or durability error.
    pub fn committed(&self) -> bool { self.committed }
}
impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.committed { write!(f, "output committed; durability or cleanup failed: {}", self.cause) } else { self.cause.fmt(f) }
    }
}
impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(&self.cause) }
}

struct Pending {
    dir: File,
    name: CString,
    file: File,
}

impl Drop for Pending {
    fn drop(&mut self) {
        // SAFETY: both the directory fd and NUL-terminated temporary name remain live.
        unsafe { libc::unlinkat(self.dir.as_raw_fd(), self.name.as_ptr(), 0); }
    }
}

impl Pending {
    fn create(dir: File) -> io::Result<Self> {
        for _ in 0..100 {
            let name = CString::new(format!(".rimage-tmp-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))).unwrap();
            match open_at(&dir, OsStr::from_bytes(name.as_bytes()), libc::O_RDWR | libc::O_CREAT | libc::O_EXCL) {
                Ok(file) => return Ok(Self { dir, name, file }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {},
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(io::ErrorKind::AlreadyExists, "temporary-name budget exhausted"))
    }
}

/// Flush and sync a destination-directory temporary file, then publish atomically.
/// A new destination never overwrites any existing directory entry.
pub fn publish(bytes: &[u8], destination: Destination<'_>, modified: Option<std::time::SystemTime>) -> Result<(), PublishError> {
    publish_cancellable(bytes,destination,modified,&crate::cancellation::Cancellation::default())
}

/// Check cancellation before staging and before starting the final publication step.
pub fn publish_cancellable(bytes: &[u8], destination: Destination<'_>, modified: Option<std::time::SystemTime>, cancellation: &crate::cancellation::Cancellation) -> Result<(), PublishError> {
    let mut committed = false;
    let result = (|| {
        if cancellation.is_cancelled() {return Err(io::Error::new(io::ErrorKind::Interrupted,"publication cancelled"));}
        let (dir, target) = match &destination {
            Destination::New(path) => split(path)?,
            Destination::Replace(source) => (source.dir.try_clone()?, source.name.clone()),
        };
        let mut pending = Pending::create(dir)?;
        pending.file.write_all(bytes)?;
        if let Some(modified) = modified { pending.file.set_modified(modified)?; }
        if let Destination::Replace(source) = &destination { pending.file.set_permissions(source.metadata()?.permissions())?; }
        pending.file.flush()?;
        pending.file.sync_all()?;
        if cancellation.is_cancelled() {return Err(io::Error::new(io::ErrorKind::Interrupted,"publication cancelled"));}
        let result = match &destination {
            Destination::New(_) => {
                // SAFETY: directory descriptors and both NUL-terminated names remain live. linkat fails atomically if target exists.
                unsafe { libc::linkat(pending.dir.as_raw_fd(), pending.name.as_ptr(), pending.dir.as_raw_fd(), target.as_ptr(), 0) }
            },
            Destination::Replace(source) => {
                source.verify()?;
                // SAFETY: source authority was revalidated and both names are relative to the same live directory.
                unsafe { libc::renameat(pending.dir.as_raw_fd(), pending.name.as_ptr(), pending.dir.as_raw_fd(), target.as_ptr()) }
            },
        };
        if result < 0 { return Err(io::Error::last_os_error()); }
        committed = true;
        // SAFETY: this name is owned by pending. It may already be absent after renameat.
        let removed = unsafe { libc::unlinkat(pending.dir.as_raw_fd(), pending.name.as_ptr(), 0) };
        if removed < 0 && !matches!(destination, Destination::Replace(_)) { return Err(io::Error::last_os_error()); }
        pending.dir.sync_all()?;
        Ok(())
    })();
    result.map_err(|cause| PublishError { committed, cause })
}

/// Resolve a destination parent without following symlinks and return a native path key.
pub fn destination_key(path: &Path) -> io::Result<PathBuf> {
    let name=path.file_name().ok_or_else(||io::Error::other("missing filename"))?;
    let mut ancestor=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut missing=Vec::new();
    let dir=loop {
        match directory(ancestor) {
            Ok(dir)=>break dir,
            Err(error) if error.kind()==io::ErrorKind::NotFound=>{
                missing.push(ancestor.file_name().ok_or_else(||io::Error::other("invalid destination ancestor"))?);
                ancestor=ancestor.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
            },
            Err(error)=>return Err(error),
        }
    };
    let mut key=ancestor.canonicalize()?;
    for part in missing.into_iter().rev() { key.push(collision_name(&dir,part)?); }
    Ok(key.join(collision_name(&dir,name)?))
}

/// Create destination ancestors without traversing symlinks.
pub fn create_parent(path: &Path) -> io::Result<()> {
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut dir=File::open(if parent.is_absolute() { "/" } else { "." })?;
    for component in parent.components() {
        match component {
            Component::RootDir|Component::CurDir=>{},
            Component::Normal(name)=>{
                let encoded=cstring(name)?;
                // SAFETY: directory fd and NUL-terminated name are live; no pointers are retained.
                let created=unsafe { libc::mkdirat(dir.as_raw_fd(),encoded.as_ptr(),0o755) };
                if created<0 { let error=io::Error::last_os_error(); if error.kind()!=io::ErrorKind::AlreadyExists { return Err(error); } }
                dir=open_at(&dir,name,libc::O_RDONLY|libc::O_DIRECTORY)?;
            },
            Component::ParentDir=>{ dir=open_at(&dir,OsStr::new(".."),libc::O_RDONLY|libc::O_DIRECTORY)?; },
            _=>return Err(io::Error::new(io::ErrorKind::InvalidInput,"unsupported path component")),
        }
    }
    Ok(())
}

/// Native directory entry inspected without following links.
pub struct DirectoryEntry {
    /// Entry name relative to its directory.
    pub name: std::ffi::OsString,
    /// Device and inode from the no-follow directory entry.
    pub file_id: (u64,u64),
    /// Entry is a directory.
    pub directory: bool,
    /// Entry is a regular file.
    pub file: bool,
    /// Entry is a symlink.
    pub symlink: bool,
}
/// Read a directory through a no-follow handle. Returned names are sorted natively.
pub fn read_directory(path:&Path)->io::Result<Vec<DirectoryEntry>> {
    use std::os::fd::IntoRawFd;
    struct Entries(*mut libc::DIR);
    impl Drop for Entries { fn drop(&mut self) { // SAFETY: fdopendir owns this live stream exclusively.
        unsafe {libc::closedir(self.0);}
    } }
    let fd=directory(path)?.into_raw_fd();
    // SAFETY: fd is an owned directory descriptor, transferred only on success.
    let raw=unsafe {libc::fdopendir(fd)};
    if raw.is_null() {let error=io::Error::last_os_error(); // SAFETY: failed fdopendir did not consume fd.
        unsafe {libc::close(fd);} return Err(error);}
    let entries=Entries(raw);let mut output=vec![];
    loop {
        // SAFETY: platform errno pointer is thread-local and valid for this thread.
        unsafe { *errno_pointer()=0; }
        // SAFETY: stream is live, uniquely owned, and not accessed by another thread.
        let entry=unsafe {libc::readdir(entries.0)};
        if entry.is_null() {let error=io::Error::last_os_error();if error.raw_os_error()!=Some(0) {return Err(error);}break;}
        // SAFETY: readdir supplies a NUL-terminated name valid until its next call.
        let name=unsafe {std::ffi::CStr::from_ptr((*entry).d_name.as_ptr())};
        if name.to_bytes()==b"."||name.to_bytes()==b".." {continue;}
        // SAFETY: stat is plain C data; fstatat initializes it on success.
        let mut stat:libc::stat=unsafe {std::mem::zeroed()};
        // SAFETY: live directory descriptor, entry name and output pointer; no link following.
        if unsafe {libc::fstatat(fd,name.as_ptr(),&mut stat,libc::AT_SYMLINK_NOFOLLOW)}<0 {return Err(io::Error::last_os_error());}
        let kind=stat.st_mode & libc::S_IFMT;
        output.push(DirectoryEntry {file_id:(stat.st_dev as u64,stat.st_ino as u64),name:OsStr::from_bytes(name.to_bytes()).to_owned(),directory:kind==libc::S_IFDIR,file:kind==libc::S_IFREG,symlink:kind==libc::S_IFLNK});
    }
    output.sort_by(|a,b|a.name.cmp(&b.name));Ok(output)
}
#[cfg(target_os="macos")]
fn errno_pointer()->*mut libc::c_int { // SAFETY: OS accessor returns this thread's errno location.
    unsafe {libc::__error()}
}
#[cfg(target_os="linux")]
fn errno_pointer()->*mut libc::c_int { // SAFETY: OS accessor returns this thread's errno location.
    unsafe {libc::__errno_location()}
}

#[cfg(target_os="macos")]
fn collision_name(directory:&File,name:&OsStr)->io::Result<std::ffi::OsString> {macos::name(directory,name)}
#[cfg(not(target_os="macos"))]
fn collision_name(_directory:&File,name:&OsStr)->io::Result<std::ffi::OsString> {Ok(name.to_owned())}

/// Resolve a regular directory's device and inode without following any symlinks.
pub fn directory_file_id(path:&Path)->io::Result<(u64,u64)> {
    let metadata=directory(path)?.metadata()?;Ok((metadata.dev(),metadata.ino()))
}
