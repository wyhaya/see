use crate::{default, exit, util};
use clap::{App, AppSettings, Arg, SubCommand};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug)]
pub enum RunType {
    Start(SocketAddr, PathBuf),
    Config(String, bool),
}

pub fn run() -> RunType {
    let app = App::new(default::SERVER_NAME)
        .version(default::VERSION)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("start")
                .about("Quick start in the current directory")
                .arg(
                    Arg::with_name("bind")
                        .short("b")
                        .long("bind")
                        .takes_value(true)
                        .value_name("IP|Port|SocketAddr")
                        .help("Change the 'start' binding address"),
                )
                .arg(
                    Arg::with_name("path")
                        .short("p")
                        .long("path")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Change the 'start' root path"),
                ),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("FILE")
                .help("Set configuration file"),
        )
        .arg(
            Arg::with_name("test")
                .short("t")
                .long("test")
                .help("Test the config file for error"),
        )
        .get_matches();

    match app.subcommand_matches("start") {
        Some(start) => {
            let addr = match start.value_of("bind") {
                Some(val) => util::to_socket_addr(val).unwrap_or_else(|err| exit!("{}", err)),
                None => default::bind_addr(),
            };

            let path = match start.value_of("path") {
                Some(val) => util::absolute_path(val, util::current_dir()),
                None => util::current_dir(),
            };

            RunType::Start(addr, path)
        }
        None => {
            let file = match app.value_of("config") {
                Some(val) => val.to_string(),
                None => default::config_path(),
            };

            let test = app.is_present("test");

            RunType::Config(file, test)
        }
    }
}
