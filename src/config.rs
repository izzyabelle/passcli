use serde::Deserialize;
use smart_default::SmartDefault;
use std::{borrow::BorrowMut, fs, path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::Parser;

#[derive(Debug, SmartDefault, Clone)]
pub enum Ops {
    Add,
    Remove,
    Edit,
    #[default]
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
            "e" | "edit" => Ok(Self::Edit),
            _ => Err(format!("{} is not a valid operation", s)),
        }
    }
}

const DEFAULT_PATH: &str = "passwords";
const DEFAULT_DISALLOW: &str = "";

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
    pub value: Option<String>,
    // disallowed characters for password generator
    #[arg(short, long, default_value = DEFAULT_DISALLOW)]
    pub disallow: String,
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
    #[arg(long, default_value = DEFAULT_PATH)]
    pub path: PathBuf,
}

#[derive(SmartDefault, Debug, Deserialize)]
pub struct Config {
    #[default = "passwords"]
    pub default_path: PathBuf,
    #[default = 16]
    pub default_pwd_len: usize,
    #[default = ""]
    pub pwd_disallow_char: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        if let Some(path) = dirs::config_dir() {
            let path = path.join("passgen\\passgen.toml");
            if path.exists() {
                Ok(toml::de::from_str(&fs::read_to_string(&path)?)?)
            } else {
                Ok(Self::default())
            }
        } else {
            Ok(Self::default())
        }
    }
}

impl Args {
    /// set required values if not supplied by command arguments
    pub fn configure(mut self, config: Config) -> Result<Self> {
        if self.path == PathBuf::from(DEFAULT_PATH) {
            self.path = config.default_path;
        }

        if let Some(len) = self.gen.borrow_mut() {
            if len.is_none() {
                *len = Some(config.default_pwd_len);
            }
        }

        if self.disallow == DEFAULT_DISALLOW {
            self.disallow = config.pwd_disallow_char;
        }

        Ok(self)
    }
}
