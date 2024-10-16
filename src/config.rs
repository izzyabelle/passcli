use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::{default, fs, path::PathBuf};
use toml;

use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Parser, Debug, SmartDefault)]
#[command(version, about, long_about = None)]
pub struct Args {
    // add, remove, list, copy
    #[arg(index = 1)]
    pub operation: String,
    // disallowed characters for password generator
    #[arg(short, long)]
    pub disallow: Option<Option<String>>,
    // password length for password generator, Some() indicates to use generated password for add
    #[arg(short, long)]
    pub gen: Option<Option<usize>>,
    // initiates interactive session
    #[arg(short, long)]
    pub interactive: bool,
    // indicates to paste from clipboard for add
    #[arg(short, long)]
    pub paste: bool,
    // optional path to use instead of config.default_path
    #[arg(index = 2)]
    pub path: Option<PathBuf>,
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
