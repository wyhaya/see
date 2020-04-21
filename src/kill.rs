#[macro_export]
macro_rules! exit {
    ($($arg:tt)*) => {
       {
            eprint!("{}", "error: ".red().bold());
            eprintln!($($arg)*);
            std::process::exit(1)
       }
    };
}

#[derive(Debug)]
pub enum KillError {
    NotExist,
    Failure,
}

#[cfg(unix)]
pub fn kill(pid: i32) -> Result<(), KillError> {
    if unsafe { libc::kill(pid, 0) } != 0 {
        return Err(KillError::NotExist);
    }
    if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
        return Err(KillError::Failure);
    }

    Ok(())
}

#[cfg(windows)]
pub fn kill(pid: i32) -> Result<(), ExitError> {
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};

    unsafe {
        let handle = OpenProcess(1, 0, pid as u32);
        TerminateProcess(handle, 0);
        CloseHandle(handle);
    }

    Ok(())
}
