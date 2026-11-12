mod cli;
mod runtime;

fn main() -> std::process::ExitCode {
    cli::run(std::env::args_os().skip(1))
}
