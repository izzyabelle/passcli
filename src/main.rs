#![allow(unused_variables)]
use anyhow::{anyhow, Result};
use clap::Parser;
use colored::*;
use config::{Args, LocalConfig, Ops};
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
    config: LocalConfig,
    master_pass: String,
    passwords: Accounts,
    interactive: bool,
}

impl App {
    /// Initializes the application by parsing arguments and the config
    /// file if present, configuring them and handling the password file.
    fn new() -> Result<Self> {
        let args = Args::parse();
        let config = LocalConfig::new()?;

        let path = args.path.as_ref().unwrap_or(&config.default_path);

        if path.exists() {
            debug!("File found at target path");
            for attempt in 0..3 {
                let prompt = if attempt == 0 {
                    String::from("Enter master password")
                } else {
                    format!("Attempt {}/3", attempt + 1)
                };

                let master_pass = prompt_password("Enter master pass")?;
                if let Ok(passwords) = read_encrypted_file(&master_pass, path) {
                    debug!("File read successfully");
                    return Ok(Self {
                        args,
                        config,
                        master_pass,
                        passwords,
                        interactive: false,
                    });
                } else {
                    warn!("Incorrect password\n");
                }
            }
            Err(anyhow!("Password attempts exceeded"))
        } else {
            info!("File not found, new file will be created");
            let master_pass = prompt_password_confirm("Create master password")?;
            Ok(Self {
                args,
                config,
                master_pass,
                passwords: HashMap::new(),
                interactive: false,
            })
        }
    }
}

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
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

    match app.args.operation {
        Ops::Interactive => {
            info!("Interactive mode initialised");
            app.interactive = true;
            loop {
                let cmd = user_input("cmd")?;
                if ["quit", "q"].contains(&cmd.as_str()) {
                    break;
                }
                let mut args = vec!["passcli "];
                args.append(&mut cmd.split_whitespace().collect());
                app.args = Args::parse_from(args);

                if let Err(e) = handle_cmd(&mut app) {
                    error!("{}", e);
                }
            }
        }
        _ => handle_cmd(&mut app)?,
    }

    write_encrypted_file(&app)?;
    Ok(0)
}

fn handle_cmd(app: &mut App) -> Result<()> {
    match app.args.operation {
        Ops::Add => handle_add(app)?,
        Ops::Remove => handle_remove(app)?,
        Ops::Edit => handle_edit(app)?,
        Ops::Print => handle_print(app)?,
        Ops::Interactive => {}
    }
    Ok(())
}

/// Add or edit account fields, an empty account can also be added
fn handle_add(app: &mut App) -> Result<()> {
    // create references for relevant fields
    let (account, field, gen, disallow, passwords) = (
        &app.args.account,
        app.args.field.as_ref().unwrap_or(&app.config.default_field),
        // left as arg ref to check if it was passed
        &app.args.gen,
        app.args
            .disallow
            .as_ref()
            .unwrap_or(&app.config.default_disallow),
        &mut app.passwords,
    );

    // early return if there is no account specified and allows reduced nesting
    if account.is_none() {
        return Err(anyhow!("Insufficient arguments supplied"));
    }
    let account = account.as_ref().unwrap();

    // check if password gen argument was specified and if so override the value
    let value = if let Some(gen) = gen {
        debug!("Password generated");
        &Some(Some(gen_passwd(
            &gen.unwrap_or(app.config.default_gen),
            disallow,
        )))
    } else {
        &app.args.value
    };

    if let Some(account_map) = passwords.get_mut(account) {
        match account_map.entry(field.clone()) {
            Entry::Occupied(mut entry) => {
                // confirm edit if field is already extant
                if confirm(
                    "This field already has a value, would you like to change it?",
                    false,
                )? {
                    entry.insert(unwrap_or_password(value)?);
                    info!("Field edited");
                } else {
                    info!("Nothing was changed");
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(unwrap_or_password(value)?);
                info!("Field created");
            }
        }
    } else {
        passwords.insert(
            account.clone(),
            HashMap::from([(field.clone(), unwrap_or_password(value)?)]),
        );
        info!("Account and field created");
    }

    Ok(())
}

fn handle_print(app: &App) -> Result<()> {
    let (account, field, hide, passwords, all) = (
        &app.args.account,
        app.args.field.as_ref().unwrap_or(&app.config.default_field),
        &app.args.hide,
        &app.passwords,
        &app.args.all_fields,
    );

    if account.is_none() {
        for (k, v) in passwords.iter() {
            print_account(k, v, hide)?;
        }
        return Ok(());
    }

    let account = account.as_ref().unwrap();

    if let Some(account_map) = passwords.get(account) {
        if *all {
            print_account(account, account_map, hide)
        } else if let Some(password) = account_map.get(field) {
            // if non interactive then have entire stdout be just the password
            if app.interactive {
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

const PROPERTY_MISSING_EDIT: &str = "No property to edit";
const PROPERTY_INPUT_PROMPT: &str = "Enter new property";
const PASSWORD_INPUT_PROMPT: &str = "Enter new password";

/// Operation to edit properties, requires specific arguments
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
        if confirm("Confirm editing master password", false)? {
            *master_pass = unwrap_or_password(value)?;
        } else {
            info!("Nothing was changed");
        }
        return Ok(());
    }

    let account = account.as_ref().unwrap();

    // flags to edit property that args specify
    let mut account_edit: Option<(String, Account)> = None;

    // if user specifies only an account or field, that will be edited
    // they can pass -v to edit the value without prompting
    // in order to edit a password they have to specify the account and field and an empty -v
    // they will then be prompted to enter a new password which will be confirmed
    if let Some(account_map) = passwords.get_mut(account) {
        if let Some(field) = field {
            if let Some(value) = value {
                // refactor to if let value
                if value.is_none() {
                    info!("Editing password");
                    let prev_password = get_or_error(field, account_map)?;
                    if !*hide {
                        println!("Current password is {}", prev_password);
                    }
                    account_map.insert(
                        field.clone(),
                        prompt_password_confirm("Enter new password")?,
                    );
                } else {
                    info!("Editing field name");
                    let field_value = get_or_error(field, account_map)?;
                    account_map.insert(value.clone().unwrap(), field_value.clone());
                    account_map.remove(field);
                }
            } else {
                info!("Editing field name");
                let field_value = get_or_error(field, account_map)?;
                account_map.insert(user_input(PROPERTY_INPUT_PROMPT)?, field_value.clone());
                account_map.remove(field);
            }
        } else {
            info!("Editing account name");
            account_edit = Some((unwrap_or_input(value)?, account_map.clone()))
        }
    } else {
        return Err(anyhow!(PROPERTY_MISSING_EDIT));
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

// convenience functions follow

fn get_or_error(field: &str, map: &Account) -> Result<String> {
    if let Some(value) = map.get(field) {
        Ok(value.clone())
    } else {
        Err(anyhow!(PROPERTY_MISSING_EDIT))
    }
}

fn unwrap_or_input(value: &Option<Option<String>>) -> Result<String, dialoguer::Error> {
    if let Some(Some(value)) = value {
        Ok(value.clone())
    } else {
        user_input(PROPERTY_INPUT_PROMPT)
    }
}

fn unwrap_or_password(value: &Option<Option<String>>) -> Result<String> {
    if let Some(Some(value)) = value {
        Ok(value.clone())
    } else {
        prompt_password_confirm(PASSWORD_INPUT_PROMPT)
    }
}

fn user_input(prompt: &str) -> Result<String, dialoguer::Error> {
    Input::new().with_prompt(prompt).interact_text()
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

fn confirm(prompt: &str, default: bool) -> Result<bool, dialoguer::Error> {
    Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
}

fn prompt_password(prompt: &str) -> Result<String, dialoguer::Error> {
    Password::new().with_prompt(prompt).interact()
}

fn prompt_password_confirm(prompt: &str) -> Result<String> {
    let new_pass = prompt_password(prompt)?;
    if prompt_password("Confirm password")? == new_pass {
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
