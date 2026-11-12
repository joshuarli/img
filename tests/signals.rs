#![cfg(feature="build-binary")]
mod common;
use std::{io::{BufRead,BufReader},process::{Command,Stdio},os::unix::process::{CommandExt,ExitStatusExt},time::Duration};
fn wait_line(stderr:&mut BufReader<std::process::ChildStderr>,needle:&str) {
    loop {let mut line=String::new();assert!(stderr.read_line(&mut line).unwrap()>0,"child exited before {needle}");if line.contains(needle) {break;}}
}
#[test]
fn graceful_signals_unblock_inherited_dispositions_join_and_reraise() {
    for signal in [libc::SIGINT,libc::SIGTERM,libc::SIGHUP,libc::SIGQUIT,libc::SIGILL] {
        let mut command=Command::new(env!("CARGO_BIN_EXE_rimage"));command.args(["convert","-","--format","png","--lossless","-o","-","--verbose"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
        // SAFETY: pre_exec calls only async-signal-safe OS functions with initialized C data.
        unsafe {command.pre_exec(move|| {let limit=libc::rlimit {rlim_cur:0,rlim_max:0};if libc::setrlimit(libc::RLIMIT_CORE,&limit)<0 {return Err(std::io::Error::last_os_error());}libc::signal(signal,libc::SIG_IGN);let mut set:libc::sigset_t=std::mem::zeroed();libc::sigemptyset(&mut set);libc::sigaddset(&mut set,signal);if libc::sigprocmask(libc::SIG_BLOCK,&set,std::ptr::null_mut())<0 {return Err(std::io::Error::last_os_error());}Ok(())});}
        let mut child=command.spawn().unwrap();let mut stderr=BufReader::new(child.stderr.take().unwrap());wait_line(&mut stderr,"admitted");
        // SAFETY: child id is a live process id; the signal is selected from OS constants.
        assert_eq!(unsafe {libc::kill(child.id() as i32,signal)},0);
        let (sender,receiver)=std::sync::mpsc::sync_channel(1);let pid=child.id();std::thread::spawn(move||sender.send(child.wait().unwrap()).unwrap());
        match receiver.recv_timeout(Duration::from_secs(30)) {Ok(status)=>assert_eq!(status.signal(),Some(signal)),Err(e)=>{unsafe {libc::kill(pid as i32,libc::SIGKILL);}panic!("signal shutdown exceeded deadline: {e}");}}
    }
}
#[test]
fn cancellation_keeps_committed_files_and_removes_unpublished_temps() {
    let scratch=common::Scratch::new("cancel-batch");let first=scratch.0.join("first.png");let second=scratch.0.join("second.png");common::write_png(&first,16,16);common::write_png(&second,512,512);let out=scratch.0.join("out");
    let mut child=Command::new(env!("CARGO_BIN_EXE_rimage")).arg("convert").arg(&first).arg(&second).arg("--out-dir").arg(&out).args(["--format","png","--lossless","--jobs","1","--verbose","--json"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut stderr=BufReader::new(child.stderr.take().unwrap());wait_line(&mut stderr,"published");
    // SAFETY: child is live and SIGTERM requests the installed graceful shutdown path.
    assert_eq!(unsafe {libc::kill(child.id() as i32,libc::SIGTERM)},0);
    let output=child.wait_with_output().unwrap();assert_eq!(output.status.signal(),Some(libc::SIGTERM));
    let first_out=out.join("first.converted.png");assert!(first_out.exists());common::check_png(&first_out,16,16,&common::pixels(16,16));
    assert!(std::fs::read_dir(&out).unwrap().all(|entry|!entry.unwrap().file_name().as_encoded_bytes().starts_with(b".rimage-tmp-")));
    let report:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();assert!(report["results"].as_array().unwrap().iter().any(|r|r["input"].as_str()==first.to_str()));
}

#[test]
fn termination_interrupts_a_full_open_stdout_pipe() {
    use std::os::fd::{AsRawFd,FromRawFd};
    let scratch=common::Scratch::new("cancel-full-pipe");let input=scratch.0.join("noisy.png");
    let mut state=1u32;let pixels:Vec<u8>=(0..512*512*3).map(|_|{state^=state<<13;state^=state>>17;state^=state<<5;state as u8}).collect();
    let mut encoder=png::Encoder::new(std::fs::File::create(&input).unwrap(),512,512);encoder.set_color(png::ColorType::Rgb);encoder.write_header().unwrap().write_image_data(&pixels).unwrap();
    let mut fds=[0;2];
    // SAFETY: fds has space for the two file descriptors returned by pipe.
    assert_eq!(unsafe {libc::pipe(fds.as_mut_ptr())},0);
    // SAFETY: pipe returned two newly owned descriptors.
    let reader=unsafe {std::fs::File::from_raw_fd(fds[0])};let writer=unsafe {std::fs::File::from_raw_fd(fds[1])};
    let mut child=Command::new(env!("CARGO_BIN_EXE_rimage")).arg("convert").arg(&input).args(["--format","png","--lossless","-o","-","--quiet"]).stdout(Stdio::from(writer.try_clone().unwrap())).stderr(Stdio::null()).spawn().unwrap();
    let deadline=std::time::Instant::now()+Duration::from_secs(30);
    loop {
        let mut available:libc::c_int=0;
        // SAFETY: reader is a live pipe and available points to writable integer storage.
        assert_eq!(unsafe {libc::ioctl(reader.as_raw_fd(),libc::FIONREAD,&mut available)},0);
        let mut poll=libc::pollfd {fd:writer.as_raw_fd(),events:libc::POLLOUT,revents:0};
        // SAFETY: poll points to one initialized descriptor.
        let ready=unsafe {libc::poll(&mut poll,1,0)};assert!(ready>=0);
        if available>0 && ready==0 {break;}
        if std::time::Instant::now()>deadline {let _=child.kill();let _=child.wait();panic!("stdout pipe did not fill");}
        assert!(child.try_wait().unwrap().is_none(),"child exited before filling stdout");std::thread::yield_now();
    }
    // SAFETY: child is live and SIGTERM requests graceful shutdown.
    assert_eq!(unsafe {libc::kill(child.id() as i32,libc::SIGTERM)},0);
    let pid=child.id();let (sender,receiver)=std::sync::mpsc::sync_channel(1);std::thread::spawn(move||sender.send(child.wait().unwrap()).unwrap());
    match receiver.recv_timeout(Duration::from_secs(3)) {Ok(status)=>assert_eq!(status.signal(),Some(libc::SIGTERM)),Err(error)=>{unsafe {libc::kill(pid as i32,libc::SIGKILL);}panic!("blocked stdout prevented termination: {error}");}}
    drop((reader,writer));
}
