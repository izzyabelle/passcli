use anyhow::Result;
use clap::Parser;
use config::{Args, Config};
use crypt::read_encrypted_file;
use rand::prelude::{thread_rng, Rng};
use rpassword::read_password;
use std::{collections::HashMap, iter::repeat_with, path::PathBuf, process::exit};

mod config;
mod crypt;

type Passwords = HashMap<String, HashMap<String, String>>;

fn main() {
    let ret = run();
    exit(ret);
}

fn run() -> i32 {
    let args = Args::parse();
    let config = match Config::new() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Config not loaded, using default\nerror: {}", err);
            Config::default()
        }
    };

    // initialise app
    let mut app = match App::new(args, config) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Failed to initialise program\nerror: {}", err);
            return 1;
        }
    };

    println!("{:?}", app.args);

    0
}

struct App {
    args: Args,
    config: Config,
    path: PathBuf,
    master_pass: String,
    passwords: Passwords,
}

impl App {
    fn new(args: Args, config: Config) -> Result<Self> {
        // use supplied path else default
        let path = if let Some(path) = &args.path {
            path.clone()
        } else {
            config.default_path.clone()
        };

        // get master password from user
        print!("Enter master password: ");
        let master_pass = read_password()?;

        // read target file
        let passwords = if path.exists() {
            read_encrypted_file(&master_pass, &path)?
        } else {
            println!("File not found, new file will be created");
            HashMap::new()
        };

        Ok(Self {
            args,
            config,
            path,
            master_pass,
            passwords,
        })
    }
}

// generates a password with ascii values between 33-126
// fn gen_passwd(len: usize, args: Args) -> String {
//     let mut rng = thread_rng();
//     String::from_utf8(repeat_with(|| rng.gen_range(33..=126)).take(len).collect())
//         // cannot possibly have err value
//         .unwrap()
// }
