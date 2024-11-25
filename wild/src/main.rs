use anyhow::anyhow;
use libc::fork;
use std::env::args;
use std::ffi::c_int;
use std::io::Error;
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
        false => {
            let mut fds: [c_int; 2] = [0; 2];
            match make_pipe(&mut fds) {
                Ok(_) => {
                    match unsafe { fork() } {
                        0 => {
                            // Fork success in the parent - wait for the child to "signal" us it's done
                            let exit_status = wait_for_child_done(&fds)?;
                            process::exit(exit_status);
                        }
                        -1 => {
                            // Fork failure in the parent - Fallback to running linker in this process
                            // Err(anyhow!("Failed to fork"))
                            wild_lib::Linker::from_args(args.into_iter())?.run(None)
                        }
                        _ => {
                            // Fork success in child - Run linker in this process with remaining args
                            let done_closure =
                                move |exit_status: i32| inform_parent_done(&fds, exit_status);
                            wild_lib::Linker::from_args(args.into_iter())?
                                .run(Some(Box::new(done_closure)))
                        }
                    }
                }
                Err(_e) => {
                    // TODO do we want to log the error, or output a warning?
                    // Err(anyhow!("Could not create named pipe: '{e}'"))
                    // Failure to creat named pipe - Fallback to running linker in this process
                    wild_lib::Linker::from_args(args.into_iter())?.run(None)
                }
            }
        }
        true => {
            // Create a linker with remaining args and run it in this process
            wild_lib::Linker::from_args(args.into_iter())?.run(None)
        }
    }
}

/// Inform the parent process that work of linker is done, sending the exit status over the pipe
fn inform_parent_done(fds: &[c_int], _exit_status: i32) {
    // Runs in child process when linking work is done - inform parent
    unsafe {
        libc::close(fds[0]);
        let stream = libc::fdopen(fds[1], "w".as_ptr() as *const i8);

        //libc::fprintf(stream, 0i8.to_le_bytes().as_ptr() as *const i8);
        libc::fclose(stream);
    }
}

/// Wait for the child process to signal it is done, by returning an exit code on the pipe
/// or for its unexpected death by closure of the pipe before receiving anything back
fn wait_for_child_done(fds: &[c_int]) -> wild_lib::error::Result<i32> {
    unsafe {
        // close our sending end of the pipe
        libc::close(fds[1]);
        // open the other end of the pipe for reading
        let stream = libc::fdopen(fds[0], "r".as_ptr() as *const i8);

        // Wait for child to exit or pipe to be closed
        /*
        let mut count = 0;
        let mut response = [0i8; 1];
        loop {
            let c = libc::fgetc(stream);
            if c == libc::EOF {
                println!("Error retrieving exit status from child process. count ={count}");
                break;
            }
            response[count] = c as i8;
            if count == 1 {
                break;
            }
            count = count + 1;
        }
         */
        let _c = libc::fgetc(stream);

        Ok(0)
    }
}

/// Create a pipe for communication between parent and child processes.
/// If successful it will return Ok and `fds` will have file descriptors for reading and writing
/// If errors it will return an error message with the errno set, if it can be read or -1 if not
fn make_pipe(fds: &mut [c_int; 2]) -> wild_lib::error::Result<()> {
    match unsafe { libc::pipe(fds.as_mut_ptr()) } {
        0 => Ok(()),
        _ => Err(anyhow!(
            "Error creating pipe. Errno = {:?}",
            Error::last_os_error().raw_os_error().unwrap_or(-1)
        )),
    }
}
