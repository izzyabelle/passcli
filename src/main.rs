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

#[derive(Debug)]
struct App {
    args: Args,
    master_pass: String,
    passwords: Accounts,
}

impl App {
    fn new(args: Args) -> Result<Self> {
        // read target passwords file or create map for new file
        let mut master_pass = String::new();
        let passwords = if args.path.exists() {
            println!();
            master_pass = rpassword::prompt_password("Enter master password: ")?;
            read_encrypted_file(&master_pass, &args.path)?
        } else {
            println!("File not found, new file will be created");
            HashMap::new()
        };

        Ok(Self {
            args,
            master_pass,
            passwords,
        })
    }
}

fn main() {
    match run() {
        Ok(ret) => exit(ret),
        Err(err) => {
            eprintln!("Exiting with error\nError: {}", err);
            exit(1);
        }
    }
}

fn run() -> Result<i32> {
    // initialise app
    let mut app = App::new(Args::parse().configure(Config::new()?)?)?;

    println!("{:#?}", app);
    // handle_cmd(&mut app)?;

    // if app.args.interactive {
    //     let mut cmd = String::new();
    //     while cmd != "quit" {
    //         cmd = input_prompt("Command: ");
    //         app.args = Args::parse_from(vec![cmd.clone()]);
    //         handle_cmd(&mut app);
    //     }
    // }

    Ok(0)
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

// generates a password with ascii values between 33-126
// fn gen_passwd(len: usize, args: Args) -> String {
//     let mut rng = thread_rng();
//     String::from_utf8(repeat_with(|| rng.gen_range(33..=126)).take(len).collect())
//         // cannot possibly have err value
//         .unwrap()
// }
