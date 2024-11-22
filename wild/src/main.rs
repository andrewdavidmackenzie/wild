use std::env::args;

fn main() -> wild_lib::error::Result {
    // Supply args to wildlib, skipping the program name
    let linker = wild_lib::Linker::from_args(args().skip(1))?;
    linker.run()
}
