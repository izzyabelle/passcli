use anyhow::{anyhow, Result};
use clap::{builder::OsStringValueParser, Parser};
use config::{Args, Config, Ops};
use crypt::{read_encrypted_file, write_encrypted_file};
use dialoguer::{Confirm, Input};
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
    /// Initializes the application by parsing arguments, configuring them,
    /// and handling the password file.
    fn new() -> Result<Self> {
        let args = Args::parse().configure(Config::new()?)?;
        println!();

        if args.path.exists() {
            for attempt in 0..3 {
                let prompt = if attempt == 0 {
                    "Enter master password: ".to_string()
                } else {
                    format!("Attempt {}/3: ", attempt + 1)
                };

                let master_pass = rpassword::prompt_password(&prompt)?;
                if let Ok(passwords) = read_encrypted_file(&master_pass, &args.path) {
                    return Ok(Self {
                        args,
                        master_pass,
                        passwords,
                    });
                } else {
                    println!("Incorrect password\n");
                }
            }
            Err(anyhow!("Password attempts exceeded"))
        } else {
            println!("File not found, new file will be created");
            let master_pass = rpassword::prompt_password("Create master password: ")?;
            Ok(Self {
                args,
                master_pass,
                passwords: HashMap::new(),
            })
        }
    }
}

fn main() {
    match run() {
        Ok(ret) => exit(ret),
        Err(err) => {
            eprintln!("Exiting with error:\n{}", err);
            exit(1);
        }
    }
}

fn run() -> Result<i32> {
    // initialise app, parses arguments and then use
    let mut app = App::new()?;

    // println!("{:#?}", app);
    handle_cmd(&mut app)?;
    // println!("{:#?}", app);

    // if app.args.interactive {
    //     let mut cmd = String::new();
    //     while cmd != "quit" {
    //         cmd = input_prompt("Command: ");
    //         app.args = Args::parse_from(vec![cmd.clone()]);
    //         handle_cmd(&mut app);
    //     }
    // }

    write_encrypted_file(&app)?;
    Ok(0)
}

fn handle_cmd(app: &mut App) -> Result<()> {
    match app.args.operation {
        Some(Ops::Add) => handle_add(app)?,
        Some(Ops::Remove) => handle_remove(app)?,
        Some(Ops::Edit) => handle_edit(app)?,
        Some(Ops::List) => handle_list(app)?,
        None => {}
    }
    Ok(())
}

// enum PasswordData {
//     Account(Account),
//     Field(String),
// }

fn handle_add(app: &mut App) -> Result<()> {
    match (app.args.account.clone(), app.args.value.clone()) {
        (Some(account), Some(value)) => {
            if let Some(account) = app.passwords.get_mut(&account) {
                match account.entry(app.args.field.clone()) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        if Confirm::new()
                            .with_prompt(
                                "This field already has a value, would you like to change it?",
                            )
                            .default(true)
                            .interact()?
                        {
                            entry.insert(value);
                            println!("Field edited");
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                        println!("Field created")
                    }
                }
            } else {
                app.passwords
                    .insert(account, HashMap::from([(app.args.field.clone(), value)]));
                println!("Account and field created");
            }
            Ok(())
        }

        _ => Err(anyhow!("Insufficient arguments supplied")),
    }
}

fn handle_list(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_edit(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_copy(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_remove(app: &mut App) -> Result<()> {
    todo!()
}

// generates a password with ascii values between 33-126
// fn gen_passwd(len: usize, args: Args) -> String {
//     let mut rng = thread_rng();
//     String::from_utf8(repeat_with(|| rng.gen_range(33..=126)).take(len).collect())
//         // cannot possibly have err value
//         .unwrap()
// }
