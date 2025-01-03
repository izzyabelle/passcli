#![allow(unused_variables)]
use anyhow::{anyhow, Result};
use clap::Parser;
use colored::*;
use config::{Args, LocalConfig, Ops, DEFAULT_MAIN_FIELD};
use crypt::{read_encrypted_file, write_encrypted_file};
use dialoguer::{Confirm, Input, Password};
use log::{debug, error, info, warn, LevelFilter};
use rand::prelude::{thread_rng, Rng};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter::repeat_with,
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
    /// Initializes the application by parsing arguments and the config
    /// file if present, configuring them and handling the password file.
    fn new() -> Result<Self> {
        let args = Args::parse().configure(LocalConfig::new()?)?;

        if args.path.exists() {
            debug!("File found at target path");
            for attempt in 0..3 {
                let prompt = if attempt == 0 {
                    String::from("Enter master password: ")
                } else {
                    format!("Attempt {}/3: ", attempt + 1)
                };

                let master_pass = Password::new().with_prompt(&prompt).interact()?;
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
            let master_pass = prompt_password_confirm("Create master password:")?;
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
        LevelFilter::Error,
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
        Some(Ops::Print(all)) => handle_print(app, all)?,
        None => {}
    }
    Ok(())
}

/// Add or edit account fields, an empty account can also be added
fn handle_add(app: &mut App) -> Result<()> {
    // create references for relevant fields
    let (account, field, gen, disallow, passwords) = (
        &app.args.account,
        &app.args.field,
        &app.args.gen,
        &app.args.disallow,
        &mut app.passwords,
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
        if let Some(account_map) = passwords.get_mut(account) {
            match account_map.entry(field.clone()) {
                Entry::Occupied(mut entry) => {
                    // confirm edit if field is already extant
                    if confirm(
                        "This field already has a value, would you like to change it?",
                        true,
                    )? {
                        entry.insert(value.clone());
                        info!("Field edited");
                    } else {
                        info!("Nothing was changed");
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(value.clone());
                    info!("Field created");
                }
            }
        } else {
            passwords.insert(
                account.clone(),
                HashMap::from([(field.clone(), value.clone())]),
            );
            info!("Account and field created");
        }
    } else {
        passwords.insert(account.clone(), HashMap::new());
        info!("Empty account initialised");
    }

    Ok(())
}

fn handle_print(app: &App, all: bool) -> Result<()> {
    let (account, field, hide, passwords) = (
        &app.args.account,
        &app.args.field,
        &app.args.hide,
        &app.passwords,
    );

    if account.is_none() {
        for (k, v) in passwords.iter() {
            print_account(k, v, hide)?;
        }
        return Ok(());
    }

    let account = account.as_ref().unwrap();

    if let Some(account_map) = passwords.get(account) {
        if all {
            print_account(account, account_map, hide)
        } else if let Some(password) = account_map.get(field) {
            if app.args.interactive {
                println!("{}", password);
            } else {
                print!("{}", password);
            }
            Ok(())
        } else {
            Err(anyhow!("Field doesn't exist"))
        }
    } else {
        Err(anyhow!("Account doesn't exist"))
    }
}

fn print_account(name: &str, account: &Account, hide: &bool) -> Result<()> {
    println!("{}:", name.magenta());
    for (k, v) in account.iter() {
        if *hide {
            println!("    {}", k.green());
        } else {
            println!("    {}: {}", k.green(), v);
        }
    }
    Ok(())
}

// needs refactoring, commenting and logging
fn handle_edit(app: &mut App) -> Result<()> {
    let (account, field, hide, value, passwords, master_pass) = (
        &app.args.account,
        &app.args.field,
        &app.args.hide,
        &app.args.value,
        &mut app.passwords,
        &mut app.master_pass,
    );

    // edit master pass if no account arg passed
    if account.is_none() {
        info!("Editing master password");
        if let Some(value) = value {
            if confirm("Confirm editing master password", false)? {
                *master_pass = value.clone();
            }
        } else {
            *master_pass = prompt_password_confirm("Enter new master password")?;
        }
    }

    let account = account.as_ref().unwrap();

    let mut account_edit: Option<(String, Account)> = None;

    if let Some(account_map) = passwords.get_mut(account) {
        if field == DEFAULT_MAIN_FIELD && !confirm("Edit account name", false)? {
            if !confirm("Edit field name", false)? {
                match account_map.entry(field.clone()) {
                    Entry::Occupied(mut entry) => {
                        if !*hide {
                            info!("Previous value: {}", *entry.get());
                        }
                        entry.insert(if let Some(value) = value {
                            value.clone()
                        } else {
                            prompt_password_confirm("New value")?
                        });
                    }
                    Entry::Vacant(entry) => return Err(anyhow!("No value to edit")),
                }
            } else {
                // needs refactoring for occupied keys
                if !account_map.contains_key(field) {
                    return Err(anyhow!("Field doesn't exist"));
                }
                let field_value = account_map[field].clone();
                account_map.insert(
                    if let Some(value) = value {
                        value.clone()
                    } else {
                        Input::new()
                            .with_prompt("New account name")
                            .interact_text()?
                    },
                    field_value,
                );
                account_map.remove(field);
            }
        } else {
            account_edit = Some((
                if let Some(value) = value {
                    value.clone()
                } else {
                    Input::new()
                        .with_prompt("New account name")
                        .interact_text()?
                },
                account_map.clone(),
            ));
        }
    } else {
        return Err(anyhow!("No value to edit"));
    }

    if let Some(new_account) = account_edit {
        // needs refactoring for occupied keys
        passwords.insert(new_account.0, new_account.1);
        passwords.remove(account);
    }

    Ok(())
}

fn handle_remove(app: &mut App) -> Result<()> {
    todo!()
}

// helper functions follow

fn confirm(prompt: &str, default: bool) -> Result<bool> {
    if let Ok(confirm) = Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
    {
        Ok(confirm)
    } else {
        Err(anyhow!("Input error"))
    }
}

fn prompt_password(prompt: &str) -> Result<String> {
    if let Ok(pass) = Password::new().with_prompt(prompt).interact() {
        Ok(pass)
    } else {
        Err(anyhow!("Input error"))
    }
}

fn prompt_password_confirm(prompt: &str) -> Result<String> {
    let new_pass = prompt_password(prompt)?;
    if prompt_password("Confirm password:")? == new_pass {
        Ok(new_pass)
    } else {
        Err(anyhow!("Mismatched password"))
    }
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
