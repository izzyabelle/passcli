#![allow(unused_variables)]
use anyhow::{anyhow, Result};
use clap::Parser;
use config::{Args, LocalConfig, Ops};
use crypt::{read_encrypted_file, write_encrypted_file};
use dialoguer::{Confirm, Input};
use log::{debug, error, info, warn, LevelFilter};
use rand::prelude::{thread_rng, Rng};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use std::{collections::HashMap, iter::repeat_with, process::exit};

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
    /// Initializes the application by parsing arguments and the config
    /// file if present, configuring them and handling the password file.
    fn new() -> Result<Self> {
        let args = Args::parse().configure(LocalConfig::new()?)?;

        if args.path.exists() {
            debug!("File found at target path");
            for attempt in 0..3 {
                let prompt = if attempt == 0 {
                    "Enter master password: ".to_string()
                } else {
                    format!("Attempt {}/3: ", attempt + 1)
                };

                let master_pass = rpassword::prompt_password(&prompt)?;
                if let Ok(passwords) = read_encrypted_file(&master_pass, &args.path) {
                    debug!("File read successfully");
                    return Ok(Self {
                        args,
                        master_pass,
                        passwords,
                    });
                } else {
                    warn!("Incorrect password\n");
                }
            }
            Err(anyhow!("Password attempts exceeded"))
        } else {
            info!("File not found, new file will be created");
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
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();
    match run() {
        Ok(ret) => exit(ret),
        Err(err) => {
            error!("Exiting with error:\n{}", err);
            exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let mut app = App::new()?;

    handle_cmd(&mut app)?;

    if app.args.interactive {
        info!("Interactive mode initialised");
        loop {
            let cmd: String = Input::new().with_prompt("cmd").interact_text()?;
            if cmd == "quit" {
                break;
            }
            let mut args = vec!["passcli "];
            args.append(&mut cmd.split_whitespace().collect());
            app.args = Args::parse_from(args).configure(LocalConfig::new()?)?;
            handle_cmd(&mut app)?;
        }
    }

    write_encrypted_file(&app)?;
    Ok(0)
}

fn handle_cmd(app: &mut App) -> Result<()> {
    match app.args.operation {
        Some(Ops::Add) => handle_add(app)?,
        Some(Ops::Remove) => handle_remove(app)?,
        Some(Ops::Edit) => handle_edit(app)?,
        Some(Ops::Print) => handle_print(app)?,
        None => {}
    }
    Ok(())
}

/// Add or edit account fields, an empty account can also be added
fn handle_add(app: &mut App) -> Result<()> {
    // create references for relevant fields
    let (account, field, gen, disallow) = (
        &app.args.account,
        &app.args.field,
        &app.args.gen,
        &app.args.disallow,
    );

    // early return if there is no account specified and allows reduced nesting
    if account.is_none() {
        return Err(anyhow!("Insufficient arguments supplied"));
    }
    let account = account.as_ref().unwrap();

    // check if password gen argument was specified and if so override the value
    let value = if let Some(gen) = gen {
        info!("Password generated");
        &Some(gen_passwd(&gen.unwrap(), disallow))
    } else {
        &app.args.value
    };

    if let Some(value) = value {
        if let Some(account_map) = app.passwords.get_mut(account) {
            match account_map.entry(field.clone()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    // confirm edit if field is already extant
                    if Confirm::new()
                        .with_prompt("This field already has a value, would you like to change it?")
                        .default(true)
                        .interact()?
                    {
                        entry.insert(value.clone());
                        info!("Field edited");
                    } else {
                        info!("Nothing was changed");
                    }
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(value.clone());
                    info!("Field created");
                }
            }
        } else {
            app.passwords.insert(
                account.clone(),
                HashMap::from([(field.clone(), value.clone())]),
            );
            info!("Account and field created");
        }
    } else {
        app.passwords.insert(account.clone(), HashMap::new());
        info!("Empty account initialised");
    }

    Ok(())
}

fn handle_print(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_edit(app: &mut App) -> Result<()> {
    todo!()
}

fn handle_remove(app: &mut App) -> Result<()> {
    todo!()
}

/// generates a password with ascii values between 33-126
/// barring any characters that are disallowed
fn gen_passwd(len: &usize, disallow: &str) -> String {
    let mut rng = thread_rng();
    let allowed_chars: Vec<u8> = (33..=126)
        .filter(|&c| !disallow.contains(c as char))
        .collect();

    repeat_with(|| allowed_chars[rng.gen_range(0..allowed_chars.len())])
        .take(*len)
        .map(|c| c as char)
        .collect()
}
