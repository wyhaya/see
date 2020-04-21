use crate::kill::{kill, KillError};
use crate::util::{config_dir, pid_path};
use crate::*;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;

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
            let mut pid = File::create(pid_path()).unwrap();
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
                    KillError::NotExist => exit!("Process does not exist"),
                    KillError::Failure => exit!("Can't kill the daemon"),
                }
            }
        }
        Err(e) => {
            exit!("Open {:?} failed\n{:?}", pid_path, e);
        }
    }
}
