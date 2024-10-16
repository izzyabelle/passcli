use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::{default, fs, path::PathBuf, str::FromStr};
use toml;

use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Parser, Debug, SmartDefault)]
#[command(version, about, long_about = None)]
pub struct Args {
    // add, remove, list, copy
    #[arg(index = 1, value_enum)]
    pub operation: Option<Ops>,
    #[arg(short, long)]
    pub account: Option<String>,
    #[arg(short, long)]
    pub field: Option<String>,
    #[arg(short, long)]
    pub key: Option<String>,
    #[arg(short, long)]
    pub value: Option<String>,

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
    // indicates that list operation should print passwords in plaintext
    #[arg(long)]
    pub print: bool,
    // optional path to use instead of config.default_path
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, SmartDefault, Clone)]
pub enum Ops {
    #[default]
    Add,
    Remove,
    Edit,
    List,
    Copy,
}

impl FromStr for Ops {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "a" | "add" => Ok(Self::Add),
            "r" | "remove" => Ok(Self::Remove),
            "l" | "list" => Ok(Self::List),
            "c" | "copy" => Ok(Self::Copy),
            _ => Err(format!("{} is not a valid operation", s)),
        }
    }
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
