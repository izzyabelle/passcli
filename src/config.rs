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
    pub add: Option<String>,
    #[arg(short, long)]
    pub copy: bool,
    #[arg(short, long)]
    pub disallow: Option<Option<String>>,
    #[arg(short, long)]
    pub gen: Option<Option<usize>>,
    #[arg(short, long)]
    pub interactive: bool,
    #[arg(short, long)]
    pub list: Option<Option<String>>,
    #[arg(short, long)]
    pub paste: bool,
    #[arg(index = 1)]
    pub path: Option<PathBuf>,
    #[arg(short, long)]
    pub remove: Option<String>,
}

#[derive(SmartDefault, Debug, Deserialize)]
pub struct Config {
    #[default = "passwords"]
    pub default_path: PathBuf,
    #[default = 12]
    pub default_pssw_len: usize,
    #[default = ""]
    pub pwd_disallow_char: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        let path = dirs::config_dir().ok_or_else(|| anyhow!("failed to find config directory"))?;
        let path = path.join("passgen\\passgen.toml");

        Ok(toml::de::from_str(&fs::read_to_string(path)?)?)
    }
}
