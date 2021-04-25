mod build;
mod commandline;
mod config;
mod context;
mod dir;
mod markdown;

#[macro_use]
extern crate lazy_static;
use crate::build::build;
use crate::config::read_config;
use log::info;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    if let Err(_) = std::env::var("LOG") {
        std::env::set_var("LOG", "info");
    }
    pretty_env_logger::init_custom_env("LOG");

    let matches = commandline::parse_command().get_matches();

    let base_dir = PathBuf::from(matches.value_of("base-directory").unwrap());
    if !base_dir.exists() {
        panic!(
            "Could not read base directory: {}",
            base_dir.to_str().unwrap()
        );
    }

    let config_path = matches.value_of("config").unwrap();
    let config_file = base_dir.join(config_path);
    let config = read_config(&config_file)
        .unwrap_or_else(|_| panic!("Could not read config: {}", config_file.to_str().unwrap()));
    info!(
        "Generating site {} using config {}",
        base_dir.to_str().unwrap(),
        config_file.to_str().unwrap()
    );

    match matches.subcommand() {
        ("build", _) => build(base_dir.into(), config.into()),
        _ => panic!(),
    }
    info!("Done in {:?}", start.elapsed());
}
