mod config;

use anyhow::Result;
use clap::Parser;
use config::{Args, Config};

pub fn run() -> Result<i32> {
    let args = Args::parse();
    let config = match Config::new() {
        Ok(config) => config,
        Err(err) => {
            eprint!("Config not loaded, using default\n error: {}", err);
            Config::default()
        }
    };

    println!("{:?}", args);

    Ok(0)
}
