use serde::Deserialize;
use smart_default::SmartDefault;
use std::{fs, path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::Parser;

#[derive(Debug, SmartDefault, Clone)]
pub enum Ops {
    Add,
    Remove,
    Edit,
    #[default]
    Print,
    Interactive,
}

impl FromStr for Ops {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "a" | "add" => Ok(Self::Add),
            "r" | "remove" => Ok(Self::Remove),
            "p" | "print" => Ok(Self::Print),
            "e" | "edit" => Ok(Self::Edit),
            "i" | "interactive" => Ok(Self::Interactive),
            _ => Err(format!("{} is not a valid operation", s)),
        }
    }
}

#[derive(Parser, Debug, SmartDefault)]
#[command(version, about, long_about = None)]
pub struct Args {
    // add, remove, list, copy
    #[arg(index = 1, value_enum)]
    pub operation: Option<Ops>,
    #[arg(index = 2)]
    pub account: Option<String>,
    #[arg(index = 3)]
    pub field: Option<String>,
    #[arg(short, long)]
    pub value: Option<Option<String>>,
    // disallowed characters for password generator
    #[arg(short, long)]
    pub disallow: Option<String>,
    // password length for password generator, Some() indicates to use generated password for add
    #[arg(short, long)]
    pub gen: Option<Option<usize>>,
    // indicates that list operation should hide actual passwords
    #[arg(long)]
    pub hide: bool,
    // optional path to use instead of config.default_path
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub pass: Option<Option<String>>,
    #[arg(short, long)]
    pub all_fields: bool,
    #[arg(short, long)]
    pub force: bool,
    #[arg(long)]
    pub new_password: Option<Option<String>>,
}

#[derive(SmartDefault, Debug, Deserialize)]
pub struct PassConfig {
    #[default(PathBuf::from("test/passwords"))]
    pub default_path: PathBuf,
    #[default(16)]
    pub default_gen: usize,
    #[default(String::new())]
    pub default_disallow: String,
    #[default(String::from("pass"))]
    pub default_field: String,
}

impl PassConfig {
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
