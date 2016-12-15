#![recursion_limit="100"]

use std::env;

extern crate sha2;
extern crate libc;
extern crate nix;
extern crate rand;
extern crate rustc_serialize;
extern crate env_logger;
extern crate argparse;
extern crate quire;
extern crate signal;
extern crate regex;
extern crate scan_dir;
extern crate zip;
extern crate tar;
extern crate flate2;
extern crate xz2;
extern crate bzip2;
extern crate net2;
extern crate docopt;
extern crate humantime;
extern crate digest_writer;
#[macro_use] extern crate matches;
#[macro_use] extern crate mopa;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate lazy_static;

#[cfg(feature="containers")]
extern crate unshare;
#[cfg(feature="containers")]
extern crate libmount;

#[macro_use] mod macros;
mod config;
mod container;
mod file_util;
mod path_util;
mod process_util;
mod tty_util;
mod options;
mod digest;
mod build_step;

#[cfg(not(feature="containers"))]
mod unshare;
#[cfg(not(feature="containers"))]
mod libmount;

// Commands
mod launcher;
mod network;
mod setup_netns;
mod version;
mod wrapper;
mod builder;
mod runner;

fn init_logging() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init().unwrap();
}

#[cfg(feature="containers")]
fn main() {
    init_logging();
    match env::args().next().as_ref().map(|x| &x[..]) {
        Some("vagga") => launcher::main(),
        Some("vagga_launcher") => launcher::main(),
        Some("vagga_network") => network::main(),
        Some("vagga_setup_netns") => setup_netns::main(),
        Some("vagga_version") => version::main(),
        Some("vagga_wrapper") => wrapper::main(),
        Some("vagga_build") => builder::main(),
        Some("vagga_runner") => runner::main(),
        _ => launcher::main(),
    }
}

#[cfg(feature="docker_runner")]
fn main() {
    init_logging();
    launcher::main();
}
