#[derive(Debug)]
pub enum ExitError {
    None,
    Failure,
}

#[cfg(unix)]
pub fn exit(pid: i32) -> Result<(), ExitError> {
    if unsafe { libc::kill(pid, 0) } != 0 {
        return Err(ExitError::None);
    }
    if unsafe { libc::kill(pid, 1) } != 0 {
        return Err(ExitError::Failure);
    }

    Ok(())
}

#[cfg(windows)]
pub fn exit(pid: i32) -> Result<(), ExitError> {
    use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};

    unsafe {
        let handle = OpenProcess(1, 0, pid as u32);
        TerminateProcess(handle, 0);
    }

    Ok(())
}
