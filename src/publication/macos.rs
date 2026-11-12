use std::{ffi::{OsStr,OsString,c_void},fs::File,io,os::{fd::AsRawFd,unix::ffi::{OsStrExt,OsStringExt}}};
#[link(name="CoreFoundation",kind="framework")]
unsafe extern "C" {
    fn CFStringCreateWithBytes(allocator:*const c_void,bytes:*const u8,length:isize,encoding:u32,external:u8)->*const c_void;
    fn CFStringCreateMutableCopy(allocator:*const c_void,maximum:isize,string:*const c_void)->*mut c_void;
    fn CFLocaleCreate(allocator:*const c_void,identifier:*const c_void)->*const c_void;
    fn CFStringFold(string:*mut c_void,flags:usize,locale:*const c_void);
    fn CFStringGetMaximumSizeOfFileSystemRepresentation(string:*const c_void)->isize;
    fn CFStringGetFileSystemRepresentation(string:*const c_void,buffer:*mut i8,capacity:isize)->u8;
    fn CFRelease(value:*const c_void);
}
struct Owned(*const c_void);
impl Drop for Owned {fn drop(&mut self) { // SAFETY: each non-null create result has one ownership reference held by this wrapper.
    unsafe {CFRelease(self.0);}
}}
pub(super) fn name(directory:&File,name:&OsStr)->io::Result<OsString> {
    // SAFETY: live directory descriptor and a constant pathconf selector.
    let sensitive=unsafe {libc::fpathconf(directory.as_raw_fd(),libc::_PC_CASE_SENSITIVE)};
    if sensitive==1 {return Ok(name.to_owned());}
    if sensitive!=0 {return Err(io::Error::last_os_error());}
    let length=isize::try_from(name.as_bytes().len()).map_err(|_|io::Error::other("path component too long"))?;
    // SAFETY: bytes are valid for length. CoreFoundation validates UTF-8 and copies them; null allocator uses the default.
    let original=unsafe {CFStringCreateWithBytes(std::ptr::null(),name.as_bytes().as_ptr(),length,0x08000100,0)};
    if original.is_null() {return Err(io::Error::new(io::ErrorKind::InvalidInput,"invalid UTF-8 filesystem name"));}
    let original=Owned(original);
    // SAFETY: original is a live CFString, copied into a separately owned mutable string.
    let mutable=unsafe {CFStringCreateMutableCopy(std::ptr::null(),0,original.0)};
    if mutable.is_null() {return Err(io::Error::other("cannot allocate filesystem name"));}
    let owned=Owned(mutable);
    // SAFETY: UTF-8 identifier bytes are copied, then used to create an immutable locale independent of user settings.
    let identifier=unsafe {CFStringCreateWithBytes(std::ptr::null(),b"en_US_POSIX".as_ptr(),11,0x08000100,0)};
    if identifier.is_null() {return Err(io::Error::other("cannot allocate filesystem locale identifier"));}
    let identifier=Owned(identifier);
    // SAFETY: identifier is a live CFString; the result owns a separate immutable locale reference.
    let locale=unsafe {CFLocaleCreate(std::ptr::null(),identifier.0)};
    if locale.is_null() {return Err(io::Error::other("cannot allocate filesystem locale"));}
    let locale=Owned(locale);
    // SAFETY: owned uniquely holds a mutable CFString; flag 1 is kCFCompareCaseInsensitive and locale is live.
    unsafe {CFStringFold(mutable,1,locale.0);}
    // SAFETY: live CFString; the returned capacity includes the terminator for the filesystem representation.
    let capacity=unsafe {CFStringGetMaximumSizeOfFileSystemRepresentation(owned.0)};
    if capacity<=0 {return Err(io::Error::other("cannot size filesystem name"));}
    let mut bytes=vec![0u8;capacity as usize];
    // SAFETY: writable buffer has capacity bytes; the string remains live and the call retains no pointers.
    if unsafe {CFStringGetFileSystemRepresentation(owned.0,bytes.as_mut_ptr().cast(),capacity)}==0 {return Err(io::Error::other("cannot normalize filesystem name"));}
    let end=bytes.iter().position(|&b|b==0).ok_or_else(||io::Error::other("unterminated filesystem name"))?;bytes.truncate(end);
    Ok(OsString::from_vec(bytes))
}
