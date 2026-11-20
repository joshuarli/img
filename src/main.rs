mod cli;
mod runtime;

#[global_allocator]
static ALLOCATOR: img::allocator::PageReturningAllocator = img::allocator::PageReturningAllocator;

fn main() -> std::process::ExitCode {
    cli::run(std::env::args_os().skip(1))
}
