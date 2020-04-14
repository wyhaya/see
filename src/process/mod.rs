use crate::util::{config_dir, config_path, pid_path};
use bright::Colorful;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
#[cfg(unix)]
mod libc;

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
pub enum ExitError {
    None,
    Failure,
}

#[cfg(unix)]
pub fn kill(pid: i32) -> Result<(), ExitError> {
    if unsafe { libc::kill(pid, 0) } != 0 {
        return Err(ExitError::None);
    }
    if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
        return Err(ExitError::Failure);
    }

    Ok(())
}

#[cfg(windows)]
pub fn kill(pid: i32) -> Result<(), ExitError> {
    use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};

    unsafe {
        let handle = OpenProcess(1, 0, pid as u32);
        TerminateProcess(handle, 0);
    }

    Ok(())
}

pub fn start_daemon(detach: &str) {
    let args = env::args().collect::<Vec<String>>();
    let cmd = args
        .iter()
        .filter(|item| item != &detach)
        .cloned()
        .collect::<Vec<String>>();

    let child = Command::new(&cmd[0]).args(&cmd[1..]).spawn();
    match child {
        Ok(child) => {
            let _ = fs::create_dir_all(config_dir());
            let mut pid = File::create(config_path()).unwrap();
            write!(pid, "{}", child.id()).unwrap();
        }
        Err(err) => exit!("Failed to run in the background\n{:?}", err),
    }
}

pub fn stop_daemon() {
    let pid_path = pid_path();
    match fs::read_to_string(&pid_path) {
        Ok(pid) => {
            if let Err(err) = fs::remove_file(&pid_path) {
                exit!("Cannot delete pid file\n{:?}", err)
            }

            let pid = match pid.parse::<i32>() {
                Ok(pid) => pid,
                Err(_) => exit!("Cannot parse pid '{}'", pid),
            };

            if let Err(err) = kill(pid) {
                match err {
                    ExitError::None => exit!("Process does not exist"),
                    ExitError::Failure => exit!("Can't kill the process"),
                }
            }
        }
        Err(e) => {
            exit!("Open {:?} failed\n{:?}", pid_path, e);
        }
    }
}
