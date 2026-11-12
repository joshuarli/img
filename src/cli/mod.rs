use std::{ffi::OsString, process::ExitCode};

pub mod pipeline;
pub mod preprocessors;
pub mod utils;
mod command;
mod help;

pub(crate) use command::{Arguments, Step};

pub fn run<I, T>(args: I) -> ExitCode
where I: IntoIterator<Item = T>, T: Into<OsString> {
    match command::parse(args) {
        Ok(command::Outcome::Run(args)) => crate::runtime::run(args),
        Ok(command::Outcome::Help(command)) => { print!("{}", help::render(command)); ExitCode::SUCCESS }
        Ok(command::Outcome::Version) => { println!("rimage {}", env!("CARGO_PKG_VERSION")); ExitCode::SUCCESS }
        Err(error) => { eprintln!("error: {error}"); ExitCode::from(2) }
    }
}
