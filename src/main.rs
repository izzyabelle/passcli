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

    if let Ops::Print = app.args.operation {
    } else {
        write_encrypted_file(&app)?;
    }
    Ok(0)
}

fn handle_cmd(app: &mut App) -> Result<()> {
    match &app.args.operation {
        Ops::Add => handle_add(app)?,
        Ops::Remove => handle_remove(app)?,
        Ops::Edit => handle_edit(app)?,
        Ops::Print => handle_print(app)?,
        Ops::Interactive => {}
    }
    Ok(())
}

const ARGUMENT_NOT_FOUND: &str = "Argument not found: ";
const PROPERTY_INPUT_PROMPT: &str = "Enter new property";
const PASSWORD_INPUT_PROMPT: &str = "Enter new password";
const CONFIRM_DELETION_PROMPT: &str = "Confirm deletion of the ";
const CONFIRM_OVERWRITE_PROMPT: &str = "Confirm overwrite";
const ACCOUNT: &str = "account ";
const FIELD: &str = "field ";

/// Add or edit account fields, an empty account can also be added
fn handle_add(app: &mut App) -> Result<()> {
    // create references for relevant fields
    let (account_arg, field_arg, gen_arg, disallow, force_arg, passwords) = (
        &app.args.account,
        app.args.field.as_ref().unwrap_or(&app.config.default_field),
        // left as arg ref to check if it was passed
        &app.args.gen,
        app.args
            .disallow
            .as_ref()
            .unwrap_or(&app.config.default_disallow),
        &app.args.force,
        &mut app.passwords,
    );

    // refactor to allow entering account and password via prompting
    if account_arg.is_none() {
        return Err(anyhow!("Insufficient arguments supplied"));
    }
    let account = account_arg.as_ref().unwrap();

    // check if password gen argument was specified and if so override the value
    let value = if let Some(gen) = gen_arg {
        debug!("Password generated");
        &Some(Some(gen_passwd(
            &gen.unwrap_or(app.config.default_gen),
            disallow,
        )))
    } else {
        &app.args.value
    };

    if let Some(account_map) = passwords.get_mut(account) {
        match account_map.entry(field_arg.clone()) {
            Entry::Occupied(mut entry) => {
                // confirm edit if field is already extant
                if confirm(CONFIRM_OVERWRITE_PROMPT, false, force_arg)? {
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
            HashMap::from([(field_arg.clone(), unwrap_or_password(value)?)]),
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
            if !app.interactive {
                println!("{}", password);
            } else {
                print!("{}", password);
            }
            Ok(())
        } else {
            Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, field))
        }
    } else {
        Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, account))
    }
}

/// Operation to edit properties, requires specific arguments
fn handle_edit(app: &mut App) -> Result<()> {
    let (account, field, hide, value, force_arg, passwords, master_pass) = (
        &app.args.account,
        &app.args.field,
        &app.args.hide,
        &app.args.value,
        &app.args.force,
        &mut app.passwords,
        &mut app.master_pass,
    );

    // Edit master pass if no account arg passed
    if account.is_none() {
        if confirm("Confirm editing master password", false, force_arg)? {
            *master_pass = unwrap_or_password(value)?;
        } else {
            info!("Nothing was changed");
        }
        return Ok(());
    }

    let account = account.as_ref().unwrap();

    // If user specifies only an account or field, that will be edited
    // They can pass -v to edit the value without prompting
    // In order to edit a password they have to specify the account and field and an empty -v
    // They will then be prompted to enter a new password which will be confirmed
    if let Some((account_key, mut account_map)) = passwords.remove_entry(account) {
        if let Some(field) = field {
            if let Some(value) = value {
                if value.is_none() {
                    info!("Editing password");
                    let prev_password = get_or_error(field, &account_map)?;
                    if !*hide {
                        println!("Current password is {}", prev_password);
                    }
                    account_map.insert(
                        field.clone(),
                        prompt_password_confirm("Enter new password")?,
                    );
                } else {
                    info!("Editing field name");
                    let field_value = get_or_error(field, &account_map)?;
                    account_map.insert(value.clone().unwrap(), field_value.clone());
                    account_map.remove(field);
                }
            } else {
                // also edits field name but no need for info! because user is prompted
                let field_value = get_or_error(field, &account_map)?;
                account_map.insert(user_input(PROPERTY_INPUT_PROMPT)?, field_value.clone());
                account_map.remove(field);
            }
        } else {
            info!("Editing account name");
            let new_account_name = unwrap_or_input(value)?;
            passwords.insert(new_account_name, account_map);
            return Ok(());
        }

        // Reinsert the modified account_map
        passwords.insert(account_key, account_map);
    } else {
        return Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, account));
    }

    Ok(())
}

fn handle_remove(app: &mut App) -> Result<()> {
    let (account_arg, field_arg, force_arg, passwords) = (
        &app.args.account,
        &app.args.field,
        &app.args.force,
        &mut app.passwords,
    );

    if account_arg.is_none() {
        if confirm(
            "No account specified, would you like to delete the entire database?",
            false,
            force_arg,
        )? && confirm(
            "Are you sure you would like to delete all your passwords?",
            false,
            force_arg,
        )? {
            *passwords = HashMap::new();
        }
        return Ok(());
    }
    let account = account_arg.as_ref().unwrap();

    if let Some((account_key, mut account_map)) = passwords.remove_entry(account) {
        match field_arg {
            Some(field) => {
                if account_map.contains_key(field) {
                    if confirm(
                        &format!("{}{}{}", CONFIRM_DELETION_PROMPT, FIELD, field),
                        false,
                        force_arg,
                    )? {
                        account_map.remove(field);
                    }
                } else {
                    // Reinsert account before returning error
                    passwords.insert(account_key, account_map);
                    return Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, FIELD));
                }
                // Reinsert the modified account_map
                passwords.insert(account_key, account_map);
            }
            None => {
                if !confirm(
                    &format!("{}{}{}", CONFIRM_DELETION_PROMPT, ACCOUNT, account),
                    false,
                    force_arg,
                )? {
                    passwords.insert(account_key, account_map);
                }
            }
        }
    } else {
        return Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, ACCOUNT));
    }

    Ok(())
}

// convenience functions follow

/// gets value from an Account or returns an error
fn get_or_error(field: &str, map: &Account) -> Result<String> {
    map.get(field)
        .cloned()
        .ok_or_else(|| anyhow!(ARGUMENT_NOT_FOUND))
}

/// unwraps a value from args or prompts user for input
fn unwrap_or_input(value: &Option<Option<String>>) -> Result<String, dialoguer::Error> {
    if let Some(Some(v)) = value {
        Ok(v.clone())
    } else {
        user_input(PROPERTY_INPUT_PROMPT)
    }
}

/// unwraps a value from args or prompts user for password with confirmation
fn unwrap_or_password(value: &Option<Option<String>>) -> Result<String> {
    if let Some(Some(v)) = value {
        Ok(v.clone())
    } else {
        prompt_password_confirm(PASSWORD_INPUT_PROMPT)
    }
}

/// shortened dialoguer user input prompt
fn user_input(prompt: &str) -> Result<String, dialoguer::Error> {
    Input::new().with_prompt(prompt).interact_text()
}

/// prints Account aesthetically, passwords hidden if specified
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

/// shortened dialoguer confirmation prompt
fn confirm(prompt: &str, default: bool, force: &bool) -> Result<bool, dialoguer::Error> {
    if *force {
        Ok(true)
    } else {
        Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact()
    }
}

/// shortened dialoguer password prompt
fn prompt_password(prompt: &str) -> Result<String, dialoguer::Error> {
    Password::new().with_prompt(prompt).interact()
}

/// prompts for password twice and only returns Ok if they match
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
