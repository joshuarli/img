use std::{collections::{BTreeMap,BTreeSet},ffi::OsString,fs,io,os::unix::fs::MetadataExt,path::{Path,PathBuf},sync::{Condvar,Mutex,OnceLock,atomic::{AtomicBool,AtomicI32,AtomicUsize,Ordering}},time::{Duration,Instant}};
use img::{cancellation::Cancellation,optimizer::{self,FormatSelection,Operation,Request},publication::{self,Destination,ReplacementSource,Source},still::{self,Format}};
use serde_json::{Value,json};
use crate::cli::{Arguments,Command};
static CANCELLED:AtomicBool=AtomicBool::new(false);
static SIGNAL:AtomicI32=AtomicI32::new(0);
const MAX_FILES:usize=100_000;
static IN_PLACE_DIRECTORIES:OnceLock<(Mutex<BTreeSet<(u64,u64)>>,Condvar)>=OnceLock::new();
struct InPlaceDirectory((u64,u64));
impl InPlaceDirectory {
    fn acquire(path:&Path,cancelled:&AtomicBool)->io::Result<Self> {
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let id=publication::directory_file_id(parent)?;
        let (lock,changed)=IN_PLACE_DIRECTORIES.get_or_init(||(Mutex::new(BTreeSet::new()),Condvar::new()));
        let mut active=lock.lock().unwrap_or_else(|e|e.into_inner());
        loop {
            if cancelled.load(Ordering::Relaxed) {return Err(io::Error::new(io::ErrorKind::Interrupted,"cancelled while waiting for in-place directory"));}
            if !active.contains(&id) {active.insert(id);return Ok(Self(id));}
            active=changed.wait_timeout(active,Duration::from_millis(100)).unwrap_or_else(|e|e.into_inner()).0;
        }
    }
}
impl Drop for InPlaceDirectory {
    fn drop(&mut self) {
        let (lock,changed)=IN_PLACE_DIRECTORIES.get().unwrap();
        lock.lock().unwrap_or_else(|e|e.into_inner()).remove(&self.0);changed.notify_all();
    }
}
#[derive(Clone,Debug)]
struct Failure {input:PathBuf,output:Option<PathBuf>,kind:&'static str,message:String,code:u8}
impl Failure {
    fn new(input:&Path,output:Option<&Path>,kind:&'static str,message:impl ToString,code:u8)->Self {Self {input:input.to_owned(),output:output.map(Path::to_owned),kind,message:message.to_string(),code}}
    fn value(&self)->Value {json!({"input":self.input.to_str(),"output":self.output.as_ref().and_then(|p|p.to_str()),"kind":self.kind,"message":self.message,"exitCode":self.code})}
}
struct Job {input:PathBuf,relative:PathBuf,format:Option<Format>,id:Option<(u64,u64)>,failure:Option<Failure>,destinations:Vec<(Format,PathBuf)>}
struct Completed {value:Value,input:PathBuf,output:Option<PathBuf>,status:&'static str}
fn discover(path:&Path,relative:&Path,args:&Arguments,output_root:Option<(u64,u64)>,jobs:&mut Vec<Job>)->Result<bool,String> {
    if jobs.len()>=MAX_FILES {return Err("input file count exceeds 100000".into());}
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir()=>{
            if output_root.is_some() && publication::directory_file_id(path).ok()==output_root {return Ok(true);}

            match publication::read_directory(path) {
                Err(error)=>jobs.push(Job {input:path.to_owned(),relative:relative.to_owned(),format:None,id:None,failure:Some(Failure::new(path,None,"input.io",error,3)),destinations:vec![]}),
                Ok(entries)=>for entry in entries {
                    let child=path.join(&entry.name);let rel=relative.join(&entry.name);
                    if entry.directory && Some(entry.file_id)==output_root {continue;}
                    if entry.symlink {jobs.push(Job {input:child.clone(),relative:rel,format:None,id:None,failure:Some(Failure::new(&child,None,"input.symlink","symlink traversal entries are refused",3)),destinations:vec![]});}
                    else if entry.directory && args.recursive {discover(&child,&rel,args,output_root,jobs)?;}
                    else if entry.file && child.extension().and_then(Format::from_extension).is_some() {jobs.push(Job {input:child,relative:rel,format:None,id:None,failure:None,destinations:vec![]});}
                    if jobs.len()>MAX_FILES {return Err("input file count exceeds 100000".into());}
                },
            }
            Ok(true)
        },
        _=>{jobs.push(Job {input:path.to_owned(),relative:relative.to_owned(),format:None,id:None,failure:None,destinations:vec![]});Ok(false)},
    }
}
fn expand(args:&Arguments)->Result<(Vec<Job>,bool),String> {
    enum Selection { Argument(PathBuf), Literal(PathBuf) }
    let mut inputs:Vec<_>=args.inputs.iter().cloned().map(Selection::Argument).collect();
    let mut lists=args.lists.clone();
    if let Some(list)=args.inputs.iter().find(|p|p.file_name().is_some_and(|p|p=="file.list")).cloned() {inputs.clear();lists.push(list);}
    for list in lists {
        let source=Source::open(&list).map_err(|e|format!("file list {}: {e}",list.display()))?;
        let bytes=still::read_bounded(source).map_err(|e|e.to_string())?;
        let contents=std::str::from_utf8(&bytes).map_err(|_|"file list must be UTF-8")?;
        let parent=list.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        for line in contents.lines().filter(|s|!s.is_empty()) {
            let path=Path::new(line);let path=if path.is_absolute() {path.to_owned()} else {parent.join(path)};
            let path=if path==Path::new("-") {std::env::current_dir().map_err(|e|e.to_string())?.join(path)} else {path};
            inputs.push(Selection::Literal(path));
        }
    }
    let output_root=args.out_dir.as_deref().and_then(|path|publication::directory_file_id(path).ok());
    let mut jobs=vec![];let mut directory=false;
    for selection in inputs {
        let mut expanded=vec![];
        match selection {
            Selection::Literal(path)=>expanded.push(path),
            Selection::Argument(input)=>{
                if let Some(text)=input.to_str().filter(|s|s.contains(['*','?','[']) && fs::symlink_metadata(&input).is_err_and(|e|e.kind()==io::ErrorKind::NotFound)) {
                    let paths=glob::glob(text).map_err(|e|e.to_string())?;
                    for result in paths {expanded.push(result.map_err(|e|e.to_string())?);}
                    if expanded.is_empty() {expanded.push(input);}
                } else {expanded.push(input);}
            },
        }
        expanded.sort();
        for path in expanded {
            let name=path.file_name().unwrap_or(path.as_os_str());
            let relative=if fs::symlink_metadata(&path).is_ok_and(|m|m.is_dir()) {Path::new("")} else {Path::new(name)};
            directory|=discover(&path,relative,args,output_root,&mut jobs)?;
        }
    }
    if jobs.is_empty() {return Err("input set is empty".into());}
    Ok((jobs,directory))
}
fn output_name(job:&Job,format:Format,args:&Arguments,batch:bool)->PathBuf {
    if let Some(output)=&args.output {return output.clone();}
    if args.in_place {return job.input.clone();}
    let suffix=if args.command==Command::Optimize {"optimized"} else {"converted"};
    let original=if args.auto && args.out_dir.is_some() {job.relative.file_name()} else {job.relative.file_stem()}.unwrap_or(job.relative.as_os_str());
    let mut name=OsString::from(original);name.push(format!(".{suffix}.{}",format.extension()));
    if let Some(root)=&args.out_dir {
        let parent=if batch {job.relative.parent().unwrap_or(Path::new(""))} else {Path::new("")};
        root.join(parent).join(name)
    } else {job.input.with_file_name(name)}
}
fn plan(args:&Arguments)->Result<Vec<Job>,String> {
    if args.json && args.inputs.iter().chain(args.lists.iter()).chain(args.output.iter()).chain(args.out_dir.iter()).any(|p|p.to_str().is_none()) {return Err("JSON requires UTF-8 paths; no files were written".into());}
    let (mut jobs,directory)=if args.command==Command::Compare {
        (args.inputs.iter().map(|p|Job {input:p.clone(),relative:p.clone(),format:None,id:None,failure:None,destinations:vec![]}).collect(),false)
    } else {expand(args)?};
    if (jobs.len()>1||directory) && matches!(args.command,Command::Optimize|Command::Convert) && args.out_dir.is_none() && !args.in_place {return Err("multiple inputs and directories require out-dir or in-place".into());}
    if args.output.is_some() && jobs.len()!=1 {return Err("output requires exactly one input".into());}
    let batch=jobs.len()>1||directory;
    if args.json && jobs.iter().any(|j|j.input.to_str().is_none()) {return Err("JSON requires UTF-8 input paths; no files were written".into());}
    if args.json && args.output.iter().chain(args.out_dir.iter()).any(|p|p.to_str().is_none()) {return Err("JSON requires UTF-8 output paths; no files were written".into());}
    let mut ids:BTreeMap<(u64,u64),Vec<usize>>=BTreeMap::new();
    for (index,job) in jobs.iter_mut().enumerate() {
        if job.failure.is_some() {continue;}
        if job.input==Path::new("-") {continue;}
        let preflight=(|| {
            let mut source=Source::open(&job.input).map_err(|e|Failure::new(&job.input,None,"input.io",e,3))?;
            job.id=Some(source.file_id());ids.entry(source.file_id()).or_default().push(index);
            let bytes=still::read_bounded(&mut source).map_err(|e|Failure::new(&job.input,None,"input.limit",e,3))?;
            let header=still::describe(&bytes).map_err(|e|Failure::new(&job.input,None,"input.decode",e,3))?;
            job.format=Some(header.format);Ok(())
        })();
        if let Err(error)=preflight {job.failure=Some(error);}
    }
    for indexes in ids.values().filter(|ids|ids.len()>1 && matches!(args.command,Command::Optimize|Command::Convert)) {for &index in indexes {let job=&mut jobs[index];job.failure=Some(Failure::new(&job.input,None,"input.alias","duplicate or hard-link aliased input",3));}}
    if matches!(args.command,Command::Inspect|Command::Compare) {return Ok(jobs);}
    let inferred=args.output.as_ref().filter(|p|p.as_path()!=Path::new("-")).and_then(|p|p.extension()).and_then(Format::from_extension);
    if let Some(output)=&args.output && output!=Path::new("-") {
        let inferred=inferred.ok_or("output extension must name jpeg, png, webp or avif")?;
        if args.format.is_some_and(|f|f!=inferred) {return Err("explicit format disagrees with output extension".into());}
    }
    if args.command==Command::Convert && args.format.or(inferred).is_none() {return Err("convert requires format or a concrete output extension".into());}
    let mut outputs:BTreeMap<PathBuf,Vec<usize>>=BTreeMap::new();
    for (index,job) in jobs.iter_mut().enumerate() {
        if job.failure.is_some() {continue;}
        let formats=if args.auto {vec![Format::Jpeg,Format::Png,Format::Webp,Format::Avif]} else {vec![args.format.or(inferred).or(job.format).ok_or("stdin requires explicit format")?]};
        for format in formats {
            if args.in_place && job.format!=Some(format) {job.failure=Some(Failure::new(&job.input,None,"input.unsupported","in-place optimization must retain the source format",3));continue;}
            let path=output_name(job,format,args,batch);
            if path==Path::new("-") {job.destinations.push((format,path));continue;}
            if args.json && path.to_str().is_none() {return Err("JSON requires UTF-8 destination paths; no files were written".into());}
            let checked=(|| {
                let key=publication::destination_key(&path).map_err(|e|Failure::new(&job.input,Some(&path),"output.path",e,4))?;
                outputs.entry(key).or_default().push(index);
                match fs::symlink_metadata(&path) {
                    Ok(meta)=>{
                        let authorized=args.in_place && meta.is_file() && !meta.file_type().is_symlink() && Some((meta.dev(),meta.ino()))==job.id;
                        if !authorized {return Err(Failure::new(&job.input,Some(&path),"output.exists","destination exists or aliases an input",4));}
                    },
                    Err(e) if e.kind()==io::ErrorKind::NotFound=>{},
                    Err(e)=>return Err(Failure::new(&job.input,Some(&path),"output.io",e,4)),
                }
                Ok(())
            })();
            if let Err(error)=checked {job.failure=Some(error);}
            job.destinations.push((format,path));
        }
    }
    for indexes in outputs.values().filter(|ids|ids.len()>1) {for &index in indexes {let job=&mut jobs[index];job.failure=Some(Failure::new(&job.input,job.destinations.first().map(|p|p.1.as_path()),"output.duplicate","duplicate planned destination",4));}}
    Ok(jobs)
}
fn stdin_bytes()->io::Result<Vec<u8>> {
    let mut bytes=vec![];let mut buffer=[0;65536];
    loop {
        if CANCELLED.load(Ordering::Relaxed) {return Err(io::Error::new(io::ErrorKind::Interrupted,"cancelled stdin"));}
        let mut poll=libc::pollfd {fd:0,events:libc::POLLIN,revents:0};
        // SAFETY: poll points to one initialized descriptor and has no retained references.
        let ready=unsafe {libc::poll(&mut poll,1,100)};
        if ready<0 {let error=io::Error::last_os_error();if error.kind()==io::ErrorKind::Interrupted {continue;}return Err(error);}
        if ready==0 {continue;}
        // SAFETY: buffer is writable for its length; stdin is a borrowed descriptor.
        let read=unsafe {libc::read(0,buffer.as_mut_ptr().cast(),buffer.len())};
        if read<0 {let error=io::Error::last_os_error();if error.kind()==io::ErrorKind::Interrupted {continue;}return Err(error);}
        if read==0 {return Ok(bytes);}
        if bytes.len() as u64+read as u64>still::MAX_INPUT_BYTES {return Err(io::Error::new(io::ErrorKind::InvalidData,"stdin exceeds 128 MiB"));}
        bytes.extend_from_slice(&buffer[..read as usize]);
    }
}
fn stdout_bytes(bytes:&[u8],finish_after_cancel:bool)->io::Result<()> {
    let _lock=io::stdout().lock();
    let allow_one_cancelled_write=finish_after_cancel && CANCELLED.load(Ordering::Relaxed);
    // SAFETY: stdout is a borrowed descriptor and fcntl retains no pointers.
    let flags=unsafe {libc::fcntl(1,libc::F_GETFL)};
    if flags<0 {return Err(io::Error::last_os_error());}
    // SAFETY: F_SETFL changes only the file status flags for this descriptor.
    if unsafe {libc::fcntl(1,libc::F_SETFL,flags|libc::O_NONBLOCK)}<0 {return Err(io::Error::last_os_error());}
    let result=(|| {
        let mut remaining=bytes;
        while !remaining.is_empty() {
            if !allow_one_cancelled_write && CANCELLED.load(Ordering::Relaxed) {return Err(io::Error::new(io::ErrorKind::Interrupted,"cancelled stdout"));}
            let mut poll=libc::pollfd {fd:1,events:libc::POLLOUT,revents:0};
            // SAFETY: poll points to one initialized descriptor and retains no references.
            let ready=unsafe {libc::poll(&mut poll,1,100)};
            if ready<0 {let error=io::Error::last_os_error();if error.kind()==io::ErrorKind::Interrupted {continue;}return Err(error);}
            if ready==0 {if CANCELLED.load(Ordering::Relaxed) {return Err(io::Error::new(io::ErrorKind::Interrupted,"cancelled stdout"));}continue;}
            let chunk=&remaining[..remaining.len().min(65536)];
            // SAFETY: chunk is readable for its length and stdout is a borrowed descriptor.
            let written=unsafe {libc::write(1,chunk.as_ptr().cast(),chunk.len())};
            if written<0 {let error=io::Error::last_os_error();if matches!(error.kind(),io::ErrorKind::Interrupted|io::ErrorKind::WouldBlock) {if CANCELLED.load(Ordering::Relaxed) {return Err(io::Error::new(io::ErrorKind::Interrupted,"cancelled stdout"));}continue;}return Err(error);}
            if written==0 {return Err(io::Error::new(io::ErrorKind::WriteZero,"stdout write returned zero"));}
            remaining=&remaining[written as usize..];
            if !remaining.is_empty() && CANCELLED.load(Ordering::Relaxed) {return Err(io::Error::new(io::ErrorKind::Interrupted,"cancelled stdout"));}
        }
        Ok(())
    })();
    // SAFETY: the descriptor remains open and flags were obtained from F_GETFL.
    let restored=unsafe {libc::fcntl(1,libc::F_SETFL,flags)};
    if restored<0 && result.is_ok() {return Err(io::Error::last_os_error());}
    result
}
enum InputSource { Read(Source), Replacement(ReplacementSource) }
impl InputSource {
    fn source(&self)->&Source {match self {Self::Read(source)=>source,Self::Replacement(source)=>source}}
}
impl io::Read for InputSource {
    fn read(&mut self,bytes:&mut [u8])->io::Result<usize> {match self {Self::Read(source)=>source.read(bytes),Self::Replacement(source)=>source.read(bytes)}}
}
fn read(job:&Job,in_place:bool)->Result<(Vec<u8>,Option<InputSource>),Failure> {
    if job.input==Path::new("-") {return stdin_bytes().map(|b|(b,None)).map_err(|e|Failure::new(&job.input,None,"input.io",e,3));}
    let source=if in_place {ReplacementSource::open(&job.input).map(InputSource::Replacement)} else {Source::open(&job.input).map(InputSource::Read)};
    let mut source=source.map_err(|e|if in_place && e.kind()==io::ErrorKind::WouldBlock {
        Failure::new(&job.input,None,"input.conflict","another in-place writer owns the source directory",3)
    } else {Failure::new(&job.input,None,"input.io",e,3)})?;
    if job.id.is_some_and(|id|id!=source.source().file_id()) {return Err(Failure::new(&job.input,None,"input.changed","source identity changed after planning",3));}
    let bytes=still::read_bounded(&mut source).map_err(|e|Failure::new(&job.input,None,"input.limit",e,3))?;Ok((bytes,Some(source)))
}
fn details(header:&still::Description)->Value {json!({"width":header.width,"height":header.height,"bitDepth":header.bit_depth,"alpha":header.alpha,"grayscale":header.grayscale,"icc":header.icc,"cicp":header.cicp,"orientation":header.orientation,"animation":header.animation,"grid":header.grid,"rotationCcwDegrees":header.rotation_ccw_degrees,"mirrorAxis":header.mirror_axis,"pixelAspectRatio":header.pixel_aspect_ratio,"hdrMetadata":header.hdr_metadata,"exif":header.exif,"xmp":header.xmp})}
/// Working memory admitted for one image. Lossless work is bounded by the
/// decoded samples (measured peaks: 36-71 bytes per pixel for RGB8 and Gray8
/// PNG); scored and quality work keeps the 256 bytes per pixel that covers
/// SSIMULACRA2's float pyramids (measured 187-220 for JPEG, PNG palette and
/// AVIF). WebP and automatic format selection keep the original reservation.
fn reservation(header:&still::Description,input:usize,lossless:bool,legacy:bool)->u64 {
    let pixels=u64::from(header.width).saturating_mul(u64::from(header.height));
    let input=input as u64;
    if legacy {return pixels.saturating_mul(256).saturating_add(input).saturating_add(still::MAX_INPUT_BYTES*2);}
    let headroom=input.saturating_mul(3).saturating_add(64<<20);
    if lossless {
        let channels=u64::from(if header.grayscale==Some(true) {1u8} else {3})+u64::from(header.alpha);
        let sample=if header.bit_depth.unwrap_or(8)>8 {2} else {1};
        pixels.saturating_mul(48).saturating_add(pixels.saturating_mul(channels*sample*24)).saturating_add(headroom)
    } else {pixels.saturating_mul(256).saturating_add(headroom)}
}
/// Bytes of working memory shared by concurrently processed images.
struct MemoryBudget {total:u64,free:Mutex<u64>,released:Condvar}
struct Reserved<'a> {budget:&'a MemoryBudget,bytes:u64}
impl MemoryBudget {
    fn new(total:u64)->Self {Self {total,free:Mutex::new(total),released:Condvar::new()}}
    /// Wait until `bytes` are free; fails with the total when they never can be.
    fn reserve(&self,bytes:u64)->Result<Reserved<'_>,u64> {
        if bytes>self.total {return Err(self.total);}
        let mut free=self.free.lock().unwrap_or_else(|e|e.into_inner());
        while *free<bytes && !CANCELLED.load(Ordering::Relaxed) {
            free=self.released.wait_timeout(free,Duration::from_millis(50)).unwrap_or_else(|e|e.into_inner()).0;
        }
        *free=free.saturating_sub(bytes);
        Ok(Reserved {budget:self,bytes})
    }
}
impl Drop for Reserved<'_> {
    fn drop(&mut self) {
        let mut free=self.budget.free.lock().unwrap_or_else(|e|e.into_inner());
        *free=(*free+self.bytes).min(self.budget.total);
        self.budget.released.notify_all();
    }
}
fn execute(job:&Job,args:&Arguments,budget:&MemoryBudget)->Result<Completed,Failure> {
    if let Some(failure)=&job.failure {return Err(failure.clone());}
    if args.verbose {eprintln!("{}: admitted",job.input.display());}
    let start=Instant::now();
    let _directory=if args.in_place {Some(InPlaceDirectory::acquire(&job.input,&CANCELLED).map_err(|e|Failure::new(&job.input,None,if e.kind()==io::ErrorKind::Interrupted {"input.cancelled"} else {"input.io"},e,3))?)} else {None};
    let (bytes,source)=read(job,args.in_place)?;
    let header=still::describe(&bytes).map_err(|e|Failure::new(&job.input,None,"input.decode",e,3))?;
    if args.command==Command::Inspect {
        return Ok(Completed {input:job.input.clone(),output:None,status:"inspected",value:json!({"input":job.input.to_str(),"output":null,"format":header.format.extension(),"inputBytes":bytes.len(),"outputBytes":null,"score":null,"elapsedMs":start.elapsed().as_secs_f64()*1000.0,"status":"inspected","warnings":[],"image":details(&header)})});
    }
    let inferred=job.destinations.first().map(|d|d.0).or(job.format).ok_or_else(||Failure::new(&job.input,None,"input.format","input format unavailable",3))?;
    let reservation=reservation(&header,bytes.len(),matches!(args.fidelity,optimizer::Fidelity::Lossless),args.auto||inferred==Format::Webp);
    let _reserved=budget.reserve(reservation).map_err(|total|Failure::new(&job.input,None,"input.limit",format!("image working reservation {reservation} exceeds memory budget {total}"),3))?;
    let operation=if args.command==Command::Convert {Operation::Convert(inferred)} else {Operation::Optimize(if args.auto {FormatSelection::Auto} else if args.format.is_some()||args.output.is_some() {FormatSelection::Exact(inferred)} else {FormatSelection::Source})};
    let cancellation=Cancellation::from_static(&CANCELLED);
    let outcome=optimizer::process_cancellable(&bytes,Request {operation,fidelity:args.fidelity,effort:args.effort,metadata:args.metadata,max_dimension:args.max_dimension},&cancellation)
        .map_err(|e|Failure::new(&job.input,job.destinations.first().map(|p|p.1.as_path()),match e.kind() {still::ErrorKind::Encode=>"output.encode",still::ErrorKind::Score=>"input.score",still::ErrorKind::Unsupported=>"input.unsupported",still::ErrorKind::Limit=>"input.limit",still::ErrorKind::Invalid=>"input.decode"},&e,if e.kind()==still::ErrorKind::Encode {4} else {3}))?;
    let output=job.destinations.iter().find(|p|p.0==outcome.format).map(|p|p.1.clone()).ok_or_else(||Failure::new(&job.input,None,"output.plan","winner has no planned destination",4))?;
    if CANCELLED.load(Ordering::Relaxed) {return Err(Failure::new(&job.input,Some(&output),"input.cancelled","cancelled before publication",3));}
    if args.verbose {for evaluation in &outcome.evaluations {eprintln!("{}: {:?} quality={:?} sampling={:?} bytes={:?} score={:?} encodeMs={:.3} failure={:?}",job.input.display(),evaluation.format,evaluation.quality,evaluation.sampling,evaluation.bytes,evaluation.score,evaluation.encode_ms,evaluation.failure);}}
    if output==Path::new("-") {stdout_bytes(&outcome.bytes,false).map_err(|e|Failure::new(&job.input,Some(&output),"output.io",e,4))?;}
    else if !(args.in_place && outcome.unchanged) {
        publication::create_parent(&output).map_err(|e|Failure::new(&job.input,Some(&output),"output.path",e,4))?;
        let timestamp=if args.timestamps {source.as_ref().map(|s|s.source().metadata().and_then(|m|m.modified())).transpose().map_err(|e|Failure::new(&job.input,Some(&output),"input.io",e,3))?} else {None};
        let destination=if args.in_place {match &source {Some(InputSource::Replacement(source))=>Destination::Replace(source),_=>return Err(Failure::new(&job.input,Some(&output),"output.plan","stdin cannot be replaced",4))}} else {Destination::New(&output)};
        publication::publish_cancellable(&outcome.bytes,destination,timestamp,&cancellation).map_err(|e|Failure::new(&job.input,Some(&output),if e.committed() {"output.committed"} else {"output.publish"},e,4))?;
    }
    if args.verbose {eprintln!("{}: published {}",job.input.display(),output.display());}
    let status=if outcome.unchanged {"unchanged"} else {"written"};
    Ok(Completed {input:job.input.clone(),output:Some(output.clone()),status,value:json!({"input":job.input.to_str(),"output":output.to_str(),"format":outcome.format.extension(),"inputBytes":bytes.len(),"outputBytes":outcome.bytes.len(),"score":outcome.score,"elapsedMs":start.elapsed().as_secs_f64()*1000.0,"status":status,"warnings":outcome.warnings})})
}
fn compare(jobs:&[Job],budget:u64)->Result<Completed,Failure> {
    let start=Instant::now();let reference=&jobs[0];let candidate=&jobs[1];
    if let Some(failure)=jobs.iter().find_map(|j|j.failure.clone()) {return Err(failure);}
    let (original,_)=read(reference,false)?;let (encoded,_)=read(candidate,false)?;
    let reference_header=still::describe(&original).map_err(|e|Failure::new(&reference.input,None,"input.decode",e,3))?;
    let candidate_header=still::describe(&encoded).map_err(|e|Failure::new(&candidate.input,None,"input.decode",e,3))?;
    let area=|header:&still::Description|u64::from(header.width).saturating_mul(u64::from(header.height));
    let reservation=area(&reference_header).saturating_add(area(&candidate_header)).saturating_mul(256).saturating_add(original.len() as u64).saturating_add(encoded.len() as u64);
    if reservation>budget {return Err(Failure::new(&candidate.input,None,"input.limit","comparison exceeds memory budget",3));}
    let dimensions=|header:&still::Description|if header.orientation>=5 {(header.height,header.width)} else {(header.width,header.height)};
    if dimensions(&reference_header)!=dimensions(&candidate_header) {return Err(Failure::new(&candidate.input,None,"input.score","score dimensions differ",3));}
    let score=img::perceptual::compare(&original,&encoded).map_err(|e|Failure::new(&candidate.input,None,"input.score",e,3))?;
    let format=Format::sniff(&encoded).map_err(|e|Failure::new(&candidate.input,None,"input.format",e,3))?;
    Ok(Completed {input:candidate.input.clone(),output:None,status:"compared",value:json!({"input":candidate.input.to_str(),"output":null,"format":format.extension(),"inputBytes":encoded.len(),"outputBytes":null,"score":score,"elapsedMs":start.elapsed().as_secs_f64()*1000.0,"status":"compared","warnings":[],"reference":reference.input.to_str()})})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_budget_admits_within_total_and_waits_for_release() {
        let budget=MemoryBudget::new(100);
        assert_eq!(budget.reserve(101).err(),Some(100));
        let held=budget.reserve(70).unwrap();
        let (sent,received)=std::sync::mpsc::sync_channel(1);
        std::thread::scope(|scope| {
            scope.spawn(|| {let _second=budget.reserve(60).unwrap();sent.send(()).unwrap();});
            assert!(received.recv_timeout(Duration::from_millis(200)).is_err());
            drop(held);
            received.recv_timeout(Duration::from_secs(3)).unwrap();
        });
        assert_eq!(*budget.free.lock().unwrap(),100);
    }

    #[test]
    fn waiting_in_place_job_observes_cancellation() {
        let path=Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let cancelled=std::sync::Arc::new(AtomicBool::new(false));let owner=InPlaceDirectory::acquire(&path,&cancelled).unwrap();
        let (started,ready)=std::sync::mpsc::sync_channel(1);let (finished,result)=std::sync::mpsc::sync_channel(1);
        let flag=cancelled.clone();let worker=std::thread::spawn(move|| {
            started.send(()).unwrap();
            let kind=match InPlaceDirectory::acquire(&path,&flag) {Ok(_)=>None,Err(error)=>Some(error.kind())};
            finished.send(kind).unwrap();
        });
        ready.recv().unwrap();cancelled.store(true,Ordering::Relaxed);
        assert_eq!(result.recv_timeout(Duration::from_secs(3)).unwrap(),Some(io::ErrorKind::Interrupted));
        drop(owner);worker.join().unwrap();
    }

    #[test]
    fn compare_reserves_both_images_before_decoding() {
        let root=Path::new(env!("CARGO_MANIFEST_DIR")).join(".work/tmp").join(format!("compare-budget-{}",std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let make=|name:&str,size:u32| {
            let path=root.join(name);let mut encoder=png::Encoder::new(fs::File::create(&path).unwrap(),size,size);
            encoder.set_color(png::ColorType::Rgb);encoder.write_header().unwrap().write_image_data(&vec![128;size as usize*size as usize*3]).unwrap();
            Job {input:path.clone(),relative:path,format:Some(Format::Png),id:None,failure:None,destinations:vec![]}
        };
        let jobs=[make("reference.png",8),make("candidate.png",256)];
        let error=match compare(&jobs,262_144) {Err(error)=>error,Ok(_)=>panic!("oversized candidate was admitted")};
        assert_eq!(error.kind,"input.limit");fs::remove_dir_all(root).unwrap();
    }
}
fn report(args:&Arguments,completed:Vec<Result<Completed,Failure>>)->u8 {
    let mut results=vec![];let mut failures=vec![];let mut categories=BTreeSet::new();let mut successes=0;
    for result in completed {
        match result {
            Ok(result)=>{
                successes+=1;
                if !args.json {
                    if !args.quiet || matches!(args.command,Command::Inspect|Command::Compare) {
                    if result.status=="compared" {if stdout_bytes(format!("{}\n",result.value["score"]).as_bytes(),true).is_err() {return 4;}}
                    else if result.status=="inspected" {if stdout_bytes(format!("{}: {} {}\n",result.input.display(),result.value["format"],result.value["image"]).as_bytes(),true).is_err() {return 4;}}
                    else {eprintln!("{}: {} {}",result.input.display(),result.status,result.output.as_deref().unwrap_or(Path::new("-")).display());}
                    }
                    if let Some(warnings)=result.value["warnings"].as_array() {for warning in warnings {eprintln!("warning: {warning}");}}
                }
                results.push(result.value);
            },
            Err(failure)=>{categories.insert(failure.code);if !args.json {eprintln!("{}: {}: {}",failure.input.display(),failure.kind,failure.message);}failures.push(failure.value());},
        }
    }
    let code=if failures.is_empty() {0} else if successes>0||categories.len()>1 {5} else {*categories.first().unwrap_or(&3)};
    if args.json {
        let data=serde_json::to_vec(&json!({"schemaVersion":1,"results":results,"failures":failures})).map_err(io::Error::other).and_then(|mut data| {data.push(b'\n');stdout_bytes(&data,true)});
        if let Err(error)=data {eprintln!("output: {error}");return 4;}
    }
    code
}
pub(crate) fn run(args:Arguments)->u8 {
    let _signals=match Signals::install() {Ok(guard)=>guard,Err(e)=>{eprintln!("cannot install signal handlers: {e}");return 4;}};
    let result=(|| {
        let jobs=match plan(&args) {Ok(jobs)=>jobs,Err(error)=>{eprintln!("error: {error}");return 2;}};
        let cpu=std::thread::available_parallelism().map(usize::from).unwrap_or(1);
        let memory=img::limits::SystemBudget::probe(1).per_image_bytes();
        let workers=args.jobs.min(cpu).min(jobs.len()).max(1);
        let budget=MemoryBudget::new(memory);
        if args.verbose {eprintln!("workers={workers} requested={} memoryBudgetBytes={memory} decodedQueue=0",args.jobs);}
        if args.command==Command::Compare {return report(&args,vec![compare(&jobs,memory)]);}
        let next=AtomicUsize::new(0);let slots=Mutex::new((0..jobs.len()).map(|_|None).collect::<Vec<_>>());
        std::thread::scope(|scope| {
            let mut handles=vec![];
            for _ in 0..workers {
                let work=|| {
                    while !CANCELLED.load(Ordering::Relaxed) {
                        let index=next.fetch_add(1,Ordering::Relaxed);
                        let Some(job)=jobs.get(index) else {break;};
                        if CANCELLED.load(Ordering::Relaxed) {break;}
                        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||execute(job,&args,&budget)))
                            .unwrap_or_else(|_|Err(Failure::new(&job.input,None,"output.worker","worker panicked",4)));
                        slots.lock().unwrap_or_else(|e|e.into_inner())[index]=Some(result);
                    }
                };
                match std::thread::Builder::new().name("img-image".into()).spawn_scoped(scope,work) {
                    Ok(handle)=>handles.push(handle),Err(error)=>{eprintln!("cannot start image worker: {error}");break;},
                }
            }
            for handle in handles {if handle.join().is_err() {eprintln!("image worker failed to join cleanly");}}
        });
        let slots=slots.into_inner().unwrap_or_else(|e|e.into_inner());
        let completed=slots.into_iter().enumerate().map(|(i,r)|r.unwrap_or_else(||Err(Failure::new(&jobs[i].input,None,if CANCELLED.load(Ordering::Relaxed) {"input.cancelled"} else {"output.worker"},"work was not admitted",if CANCELLED.load(Ordering::Relaxed) {3} else {4})))).collect();
        report(&args,completed)
    })();
    let signal=SIGNAL.load(Ordering::Relaxed);if signal!=0 {reraise(signal);}
    result
}
#[cfg(target_os="macos")]
unsafe fn errno()->*mut libc::c_int { // SAFETY: this thread's errno accessor has no arguments or retained pointers.
    unsafe {libc::__error()}
}
#[cfg(target_os="linux")]
unsafe fn errno()->*mut libc::c_int { // SAFETY: this thread's errno accessor has no arguments or retained pointers.
    unsafe {libc::__errno_location()}
}
extern "C" fn graceful(signal:libc::c_int) {
    // SAFETY: errno is thread-local; the handler performs only lock-free atomic stores.
    unsafe {let pointer=errno();let saved=*pointer;let _=SIGNAL.compare_exchange(0,signal,Ordering::Relaxed,Ordering::Relaxed);CANCELLED.store(true,Ordering::Relaxed);*pointer=saved;}
}
extern "C" fn fatal(signal:libc::c_int) {
    // SAFETY: sigaction, sigemptyset, getpid and kill are async-signal-safe; all pointers refer to local C data.
    unsafe {let pointer=errno();let saved=*pointer;let mut action:libc::sigaction=std::mem::zeroed();libc::sigemptyset(&mut action.sa_mask);action.sa_sigaction=libc::SIG_DFL;libc::sigaction(signal,&action,std::ptr::null_mut());libc::kill(libc::getpid(),signal);*pointer=saved;}
}
fn reraise(signal:i32)->! {
    // SAFETY: restore default and unblock this signal after all workers have joined and temporary files dropped.
    unsafe {libc::signal(signal,libc::SIG_DFL);let mut set:libc::sigset_t=std::mem::zeroed();libc::sigemptyset(&mut set);libc::sigaddset(&mut set,signal);libc::sigprocmask(libc::SIG_UNBLOCK,&set,std::ptr::null_mut());libc::raise(signal);libc::_exit(128+signal);}
}
struct Signals {old:Vec<(i32,libc::sigaction)>,mask:libc::sigset_t}
impl Signals {
    fn install()->io::Result<Self> {
        // SAFETY: initialized C signal sets and actions are passed to checked OS calls, with no retained pointers.
        unsafe {
            let mut mask:libc::sigset_t=std::mem::zeroed();
            if libc::sigprocmask(libc::SIG_SETMASK,std::ptr::null(),&mut mask)<0 {return Err(io::Error::last_os_error());}
            let mut guard=Self {old:vec![],mask};let mut set:libc::sigset_t=std::mem::zeroed();libc::sigemptyset(&mut set);
            for signal in [libc::SIGINT,libc::SIGTERM,libc::SIGHUP,libc::SIGQUIT,libc::SIGABRT,libc::SIGILL,libc::SIGTRAP,libc::SIGSEGV,libc::SIGBUS,libc::SIGFPE] {
                let mut action:libc::sigaction=std::mem::zeroed();libc::sigemptyset(&mut action.sa_mask);
                let is_graceful=matches!(signal,libc::SIGINT|libc::SIGTERM|libc::SIGHUP|libc::SIGQUIT);
                action.sa_sigaction=if is_graceful {graceful as *const () as usize} else {fatal as *const () as usize};
                action.sa_flags=if is_graceful {0} else {libc::SA_RESETHAND};
                let mut old=std::mem::zeroed();if libc::sigaction(signal,&action,&mut old)<0 {return Err(io::Error::last_os_error());}
                guard.old.push((signal,old));libc::sigaddset(&mut set,signal);
            }
            if libc::sigprocmask(libc::SIG_UNBLOCK,&set,std::ptr::null_mut())<0 {return Err(io::Error::last_os_error());}
            Ok(guard)
        }
    }
}
impl Drop for Signals {
    fn drop(&mut self) { // SAFETY: saved actions and mask came from this process; all worker owners have ended.
        unsafe {for (signal,action) in &self.old {libc::sigaction(*signal,action,std::ptr::null_mut());}libc::sigprocmask(libc::SIG_SETMASK,&self.mask,std::ptr::null_mut());}
    }
}
