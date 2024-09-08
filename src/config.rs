use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::{default, fs, path::PathBuf};
use toml;

use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pass: Option<String>,
    #[arg(short, long)]
    list: Option<String>,
    #[arg(short, long)]
    gen: Option<Option<usize>>,
    #[arg(num_args = 2, short = 'N', long)]
    new: Vec<String>,
    #[arg(short = 'L', long)]
    lock: bool,
    #[arg(short = 'U', long)]
    unlock: bool,
    #[arg(short, long)]
    append: Option<String>,
}

#[derive(SmartDefault, Debug, Deserialize)]
pub struct Config {
    #[default = "passwords"]
    default_filename: String,
    unlock_warning: bool,
}

impl Config {
    pub fn new() -> Result<Self> {
        let path = dirs::config_dir().ok_or_else(|| anyhow!("failed to find config directory"))?;
        let path = path.join("passgen\\passgen.toml");

        Ok(toml::de::from_str(&fs::read_to_string(path)?)?)
    }
}
