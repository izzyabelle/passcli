use anyhow::{anyhow, Result};
use clap::{builder::OsStringValueParser, Parser};
use config::{Args, Config, Ops};
use crypt::read_encrypted_file;
use rand::prelude::{thread_rng, Rng};
use rpassword::read_password;
use std::{
    collections::HashMap,
    io::{self, Write},
    iter::repeat_with,
    path::PathBuf,
    process::exit,
};

mod config;
mod crypt;

type Account = HashMap<String, String>;
type Accounts = HashMap<String, Account>;

fn main() {
    let ret = run();
    exit(ret);
}

fn input_prompt(prompt: &str) -> String {
    // Print a prompt to the user
    print!("{}", prompt);
    // Flush stdout to ensure the prompt is displayed before user input
    io::stdout().flush().unwrap();

    // Create a new String to store the user input
    let mut input = String::new();

    // Read the user input from stdin
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input
}

fn run() -> i32 {
    // initialise app
    let args = Args::parse();
    let config = match Config::new() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Config not loaded, using default\nerror: {}", err);
            Config::default()
        }
    };
    let mut app = match App::new(args, config) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Failed to initialise program\nerror: {}", err);
            return 1;
        }
    };

    handle_cmd(&mut app);

    if app.args.interactive {
        let mut cmd = String::new();
        while cmd != "quit" {
            cmd = input_prompt("Command: ");
            app.args = Args::parse_from(vec![cmd.clone()]);
            handle_cmd(&mut app);
        }
    }

    0
}

fn handle_cmd(app: &mut App) -> Result<()> {
    match app.args.operation {
        Some(Ops::Add) => handle_add(app)?,
        Some(Ops::Remove) => handle_remove(app)?,
        Some(Ops::Edit) => handle_edit(app)?,
        Some(Ops::Copy) => handle_copy(app)?,
        Some(Ops::List) => handle_list(app)?,
        None => {}
    }
    Ok(())
}

enum PasswordData {
    Account(Account),
    Field(String),
}

enum EditCmd {
    Key(String),
    Value(String),
}

fn handle_edit(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_list(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_copy(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_remove(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_add(app: &mut App) -> Result<()> {
    todo!()
}

struct App {
    args: Args,
    config: Config,
    path: PathBuf,
    master_pass: String,
    passwords: Accounts,
}

impl App {
    fn new(args: Args, config: Config) -> Result<Self> {
        // use supplied path else default
        let path = if let Some(path) = &args.path {
            path.clone()
        } else {
            config.default_path.clone()
        };

        let mut master_pass = String::new();
        // read target file
        let passwords = if path.exists() {
            println!();
            master_pass = rpassword::prompt_password("Enter master password: ")?;
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
