use anyhow::anyhow;
use libc::fork;
use std::env::args;
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process;

fn main() -> wild_lib::error::Result {
    // skip the program name
    let mut args: Vec<String> = args().skip(1).collect();

    // See if there is a better way to do this, more canonical
    let mut no_fork_subprocess = false;
    args.retain(|a| {
        if a == "--no-fork" {
            no_fork_subprocess = true;
            false
        } else {
            true
        }
    });

    match no_fork_subprocess {
        false => match make_named_pipe() {
            Ok(path) => {
                match unsafe { fork() } {
                    0 => {
                        // Success in the parent
                        let mut f = File::open(&path)?;
                        let mut response = [0u8; 4];
                        // Wait for child to exit or pipe to be closed
                        let count = f.read(&mut response);
                        // Remove the file always - before checking other things
                        fs::remove_file(path)?;
                        match count {
                            Ok(4) => process::exit(i32::from_ne_bytes(response)),
                            _ => Err(anyhow!("Error retrieving exit status from child process")),
                        }
                    }
                    -1 => {
                        // Failure in the parent
                        // Create a linker with remaining args and run it
                        wild_lib::Linker::from_args(args.into_iter())?.run()
                    }
                    _ => {
                        // Success in the child
                        // Create a linker with remaining args and run it
                        wild_lib::Linker::from_args(args.into_iter())?.run()?;

                        // inform parent that we are done!
                        let mut f = File::open(path)?;
                        f.write_all(&0i32.to_ne_bytes())?;
                        f.flush()?;
                        Ok(())
                    }
                }
            }
            Err(_) => {
                // Create a linker with remaining args and run it
                wild_lib::Linker::from_args(args.into_iter())?.run()
            }
        },
        true => {
            // Create a linker with remaining args and run it
            wild_lib::Linker::from_args(args.into_iter())?.run()
        }
    }
}

/// Create a named pipe for communication between parent and child processes.
/// If successful it will return Ok with the name of the file
/// If errors it will return an error message with the errno set, if it can be read or -1 if not
fn make_named_pipe() -> wild_lib::error::Result<String> {
    let path = format!(
        "{}/{}",
        tempdir::TempDir::new("wild")?.path().display(),
        process::id()
    );
    if Path::new(&path).exists() {
        fs::remove_file(&path)?;
    }
    let filename = CString::new(path.as_str())?;
    unsafe {
        match libc::mkfifo(filename.as_ptr(), 0o660) {
            0 => Ok(path.to_owned()),
            _ => Err(anyhow!(
                "Error creating named pipe. Errno = {:?}",
                Error::last_os_error().raw_os_error().unwrap_or(-1)
            )),
        }
    }
}
