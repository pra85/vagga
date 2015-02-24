extern crate quire;
extern crate argparse;
extern crate serialize;
extern crate libc;
#[macro_use] extern crate log;

extern crate config;
#[macro_use] extern crate container;

use std::old_io::stderr;
use std::env::{set_exit_status};
use std::os::{getcwd};

use argparse::{ArgumentParser, Store, List};

use config::{find_config, Config, Settings};
use config::command::MainCommand::{Command, Supervise};
use container::signal;
use settings::{read_settings, MergedSettings};


mod settings;
mod debug;
mod build;
mod run;
mod supervise;
mod commandline;
mod setup;
mod util;
mod clean;


struct Wrapper<'a> {
    config: &'a Config,
    settings: &'a Settings,
    project_root: &'a Path,
    ext_settings: &'a MergedSettings,
}


pub fn run() -> i32 {
    let mut err = stderr();
    let mut cmd: String = "".to_string();
    let mut args: Vec<String> = Vec::new();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("
            Internal vagga tool to setup basic system sandbox
            ");
        ap.refer(&mut cmd)
          .add_argument("command", Store,
                "A vagga command to run")
          .required();
        ap.refer(&mut args)
          .add_argument("args", List,
                "Arguments for the command");
        ap.stop_on_first_argument(true);
        match ap.parse_args() {
            Ok(()) => {}
            Err(0) => return 0,
            Err(_) => {
                return 122;
            }
        }
    }

    let workdir = getcwd().unwrap();

    let (config, project_root) = match find_config(&workdir) {
        Ok(tup) => tup,
        Err(e) => {
            err.write_line(e.as_slice()).ok();
            return 126;
        }
    };
    let (ext_settings, int_settings) = match read_settings(&project_root)
    {
        Ok(tup) => tup,
        Err(e) => {
            err.write_line(e.as_slice()).ok();
            return 126;
        }
    };

    let wrapper = Wrapper {
        config: &config,
        settings: &int_settings,
        project_root: &project_root,
        ext_settings: &ext_settings,
    };

    args.insert(0, format!("vagga {}", cmd));

    let result = match cmd.as_slice() {
        "_build_shell" => Ok(debug::run_interactive_build_shell(&wrapper)),
        "_build" => build::build_container_cmd(&wrapper, args),
        "_version_hash" => build::print_version_hash_cmd(&wrapper, args),
        "_run" => run::run_command_cmd(&wrapper, args, true),
        "_run_in_netns" => run::run_command_cmd(&wrapper, args, false),
        "_clean" => clean::clean_cmd(&wrapper, args),
        _ => {
            match config.commands.get(&cmd) {
                Some(&Command(ref cmd_info)) => {
                    commandline::commandline_cmd(cmd_info, &wrapper, args)
                }
                Some(&Supervise(ref svc_info)) => {
                    supervise::supervise_cmd(&cmd, svc_info, &wrapper, args)
                }
                None => {
                    error!("Unknown command {}", cmd);
                    return 127;
                }
            }
        }
    };
    match result {
        Ok(x) => return x,
        Err(e) => {
            error!("Error executing {}: {}", cmd, e);
            return 124;
        }
    };
}

fn main() {
    signal::block_all();
    let val = run();
    set_exit_status(val);
}
