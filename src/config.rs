use log::{debug, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};

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
    pub pass: Option<String>,
    #[arg(short, long)]
    pub all_fields: bool,
    #[arg(short, long)]
    pub force: bool,
    #[arg(long)]
    pub new_password: Option<Option<String>>,
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum LevelFilterConf {
    Off,
    Info,
    Warn,
    Debug,
    Trace,
}

impl From<LevelFilterConf> for LevelFilter {
    fn from(val: LevelFilterConf) -> Self {
        match val {
            LevelFilterConf::Off => LevelFilter::Off,
            LevelFilterConf::Info => LevelFilter::Info,
            LevelFilterConf::Warn => LevelFilter::Warn,
            LevelFilterConf::Debug => LevelFilter::Debug,
            LevelFilterConf::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(SmartDefault, Debug, Deserialize, Serialize)]
pub struct PassConfig {
    #[default(None)]
    pub default_pass: Option<String>,
    #[default(None)]
    pub default_path: Option<PathBuf>,
    #[default(16)]
    pub default_gen: usize,
    #[default(String::new())]
    pub default_disallow: String,
    #[default(String::from("pass"))]
    pub default_field: String,
    #[default(LevelFilterConf::Info)]
    pub log_level: LevelFilterConf,
    #[default(false)]
    pub default_hide: bool,
    #[default(false)]
    pub default_force: bool,
    #[default(3)]
    pub kdf_iterations: u32,
}

impl PassConfig {
    pub fn new() -> Result<Self> {
        if let Some(path) = dirs::config_dir() {
            let path = path.join("passcli/passcli.toml");
            if path.exists() {
                Ok(toml::de::from_str(&fs::read_to_string(&path)?)?)
            } else {
                debug!("Config file not found, generating new file");
                let config = Self::default();
                let file_contents = toml::ser::to_string(&Self::default())?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = File::create(path)?;
                file.write_all(file_contents.as_bytes())?;
                Ok(config)
            }
        } else {
            warn!("Config directory not found, using default config");
            Ok(Self::default())
        }
    }
}
