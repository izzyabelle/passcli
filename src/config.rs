use log::warn;
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
    Print(bool),
}

impl FromStr for Ops {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "a" | "add" => Ok(Self::Add),
            "r" | "remove" => Ok(Self::Remove),
            "p" | "print" => Ok(Self::Print(false)),
            "pa" | "printall" => Ok(Self::Print(true)),
            "e" | "edit" => Ok(Self::Edit),
            _ => Err(format!("{} is not a valid operation", s)),
        }
    }
}

const DEFAULT_PATH: &str = "test/passwords";
const DEFAULT_DISALLOW: &str = "";
const DEFAULT_GEN_LEN: usize = 16;
const DEFAULT_MAIN_FIELD: &str = "pass";

#[derive(Parser, Debug, SmartDefault)]
#[command(version, about, long_about = None)]
pub struct Args {
    // add, remove, list, copy
    #[arg(index = 1, value_enum)]
    pub operation: Option<Ops>,
    #[arg(index = 2)]
    pub account: Option<String>,
    #[arg(short, long, default_value = DEFAULT_MAIN_FIELD)]
    pub field: String,
    #[arg(index = 3)]
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
    // indicates that list operation should hide actual passwords
    #[arg(long)]
    pub hide: bool,
    // optional path to use instead of config.default_path
    #[arg(long, default_value = DEFAULT_PATH)]
    pub path: PathBuf,
}

#[derive(SmartDefault, Debug, Deserialize)]
pub struct LocalConfig {
    #[default(PathBuf::from(DEFAULT_PATH))]
    pub default_path: PathBuf,
    #[default(DEFAULT_GEN_LEN)]
    pub default_pwd_len: usize,
    #[default(String::from(DEFAULT_DISALLOW))]
    pub pwd_disallow_char: String,
    #[default(String::from(DEFAULT_MAIN_FIELD))]
    pub default_main_field: String,
}

impl LocalConfig {
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
    /// Set required values if not supplied by command arguments.
    /// If no argument has been passed then the field will match
    /// the default and therefore the value from the config file
    /// should be read.
    pub fn configure(mut self, config: LocalConfig) -> Result<Self> {
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

        if self.field == DEFAULT_MAIN_FIELD {
            warn!(
                "Default field ('{}') is being used",
                config.default_main_field
            );
            self.field = config.default_main_field;
        }

        Ok(self)
    }
}
