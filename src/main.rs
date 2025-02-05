#![allow(unused_variables)]
use anyhow::{anyhow, Result};
use clap::Parser;
use colored::*;
use config::{Args, Ops, PassConfig};
use crypt::{read_encrypted_file, write_encrypted_file};
use dialoguer::{Confirm, Input, Password};
use log::{debug, error, info, LevelFilter};
use rand::prelude::{thread_rng, Rng};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use std::{
    collections::{hash_map::Entry, HashMap},
    fs,
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
    config: PassConfig,
    path: PathBuf,
    master_pass: String,
    passwords: Accounts,
    interactive: bool,
}

impl App {
    /// Initializes the application by parsing arguments and the config file
    /// if present, then handling the password file. Also initialises logger
    fn new() -> Result<Self> {
        let args = Args::parse();
        let config = PassConfig::new()?;

        CombinedLogger::init(vec![TermLogger::new(
            if args.quiet {
                LevelFilter::Off
            } else {
                LevelFilter::from(config.log_level)
            },
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )])
        .unwrap();

        let path = if let Some(p) = args.path.as_ref() {
            p.clone()
        } else if let Some(p) = config.default_path.as_ref() {
            p.clone()
        } else if let Some(p) = dirs::data_dir() {
            p.join("passcli/passwd")
        } else {
            return Err(anyhow!(
                "No target path specified and no data directory found"
            ));
        };

        let master_pass = if let Some(p) = args.pass.as_ref() {
            p.clone()
        } else if let Some(p) = config.default_pass.as_ref() {
            p.clone()
        } else {
            prompt_password(MASTER_PASSWORD_INPUT_PROMPT, false, &args.force)?
        };

        if path.exists() {
            debug!("File found at target path");
            if let Ok(passwords) = read_encrypted_file(&master_pass, &path, &config.kdf_iterations)
            {
                debug!("File read successfully");
                Ok(Self {
                    args,
                    config,
                    path,
                    master_pass,
                    passwords,
                    interactive: false,
                })
            } else {
                Err(anyhow!("Incorrect password\n"))
            }
        } else {
            info!("File not found, new file will be created");
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let master_pass = prompt_password("Create master password", true, &false)?;
            Ok(Self {
                args,
                config,
                path,
                master_pass,
                passwords: HashMap::new(),
                interactive: false,
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
    let mut app = App::new()?;

    match app.args.operation {
        Some(Ops::Interactive) => {
            info!("Interactive mode initialised (q or quit to exit)");
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

                // write file after every command in case of arg parsing error
                if let Some(Ops::Print) | None = app.args.operation {
                } else {
                    write_encrypted_file(&app)?;
                }
            }
            Ok(0)
        }
        _ => {
            handle_cmd(&mut app)?;
            if let Some(Ops::Print) | None = app.args.operation {
                Ok(0)
            } else {
                write_encrypted_file(&app)
            }
        }
    }
}

fn handle_cmd(app: &mut App) -> Result<()> {
    match &app.args.operation {
        Some(Ops::Add) => handle_add(app)?,
        Some(Ops::Remove) => handle_remove(app)?,
        Some(Ops::Edit) => handle_edit(app)?,
        Some(Ops::Print) | None => handle_print(app)?,
        Some(Ops::Interactive) => {}
    }
    Ok(())
}

const ARGUMENT_NOT_FOUND: &str = "Argument not found: ";
const PROPERTY_INPUT_PROMPT: &str = "Enter new property";
const NEW_PASSWORD_INPUT_PROMPT: &str = "Enter new password";
const MASTER_PASSWORD_INPUT_PROMPT: &str = "Enter master password";
const CONFIRM_DELETION_PROMPT: &str = "Confirm deletion of the ";
const CONFIRM_OVERWRITE_PROMPT: &str = "Confirm overwrite";
const ACCOUNT: &str = "account ";
const FIELD: &str = "field ";

/// Add or edit account fields, an empty account can also be added
fn handle_add(app: &mut App) -> Result<()> {
    // create references for relevant fields
    let (account_arg, field_arg, gen_arg, force_arg, hide, interactive, disallow, passwords) = (
        &app.args.account,
        app.args.field.as_ref().unwrap_or(&app.config.default_field),
        // left as arg ref to check if it was passed
        &app.args.gen,
        &app.args.force,
        &app.args.hide,
        &app.interactive,
        app.args
            .disallow
            .as_ref()
            .unwrap_or(&app.config.default_disallow),
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
            hide,
        )))
    } else {
        &app.args.value
    };

    if let Some(account_map) = passwords.get_mut(account) {
        match account_map.entry(field_arg.clone()) {
            Entry::Occupied(mut entry) => {
                // confirm edit if field is already extant
                if confirm(CONFIRM_OVERWRITE_PROMPT, false, force_arg)? {
                    entry.insert(unwrap_or_new_password(value, force_arg)?);
                    info!("Field edited");
                } else {
                    info!("Nothing was changed");
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(unwrap_or_new_password(value, force_arg)?);
                info!("Field created");
            }
        }
    } else {
        passwords.insert(
            account.clone(),
            HashMap::from([(field_arg.clone(), unwrap_or_new_password(value, force_arg)?)]),
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
    let (
        account_arg,
        field_arg,
        hide,
        value_arg,
        force_arg,
        new_password_arg,
        gen_arg,
        interactive,
        disallow,
        passwords,
        master_pass,
    ) = (
        &app.args.account,
        &app.args.field,
        &app.args.hide,
        &app.args.value,
        &app.args.force,
        &app.args.new_password,
        &app.args.gen,
        &app.interactive,
        app.args
            .disallow
            .as_ref()
            .unwrap_or(&app.config.default_disallow),
        &mut app.passwords,
        &mut app.master_pass,
    );

    // Edit master pass if no account arg passed
    if account_arg.is_none() {
        if confirm("Confirm editing master password", false, force_arg)? {
            *master_pass = unwrap_or_new_password(value_arg, force_arg)?;
        }
        return Ok(());
    }

    let account = account_arg.as_ref().unwrap();

    // prepare genned password to simplify nested code
    let genned_password = gen_arg.map(|gen_arg_value| {
        gen_passwd(
            &gen_arg_value.unwrap_or(app.config.default_gen),
            disallow,
            hide,
        )
    });

    // If user specifies only an account or field, that will be edited
    // They can pass -v with a value to edit those values without prompting
    // In order to edit a password they have to specify the account and field with
    // -g or --new-password. An empty --new-password will reult in a prompt
    if let Some((account_key, mut account_map)) = passwords.remove_entry(account) {
        if let Some(field) = field_arg {
            if let (Some(_), _) | (_, Some(_)) = (new_password_arg, gen_arg) {
                info!("Editing password");
                let prev_password = get_or_error(field, &account_map)?;
                if !*hide {
                    println!("Previous password is {}", prev_password);
                }
                // get value for new password, prioritising -g
                let new_password = if let Some(v) = genned_password {
                    v
                } else {
                    unwrap_or_new_password(new_password_arg, force_arg)?
                };
                account_map.insert(field.clone(), new_password);
            } else {
                info!("Editing field name");
                if let Some(old_value) = account_map.remove(field) {
                    let new_key = &unwrap_or_input(value_arg)?;
                    if account_map.contains_key(new_key) {
                        if confirm(CONFIRM_OVERWRITE_PROMPT, false, force_arg)? {
                            account_map.insert(new_key.clone(), old_value);
                        }
                    } else {
                        account_map.insert(new_key.clone(), old_value);
                    }
                } else {
                    return Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, FIELD));
                }
            }
            // Reinsert the modified account_map
            passwords.insert(account_key, account_map);
        } else {
            info!("Editing account name");
            let new_key = &unwrap_or_input(value_arg)?;
            // we know at this point that the account exists
            if passwords.contains_key(new_key) {
                if confirm(CONFIRM_OVERWRITE_PROMPT, false, force_arg)? {
                    passwords.insert(new_key.clone(), account_map);
                }
            } else {
                passwords.insert(new_key.clone(), account_map);
            }
        }
    } else {
        return Err(anyhow!("{}{}", ARGUMENT_NOT_FOUND, ACCOUNT));
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

/// double unwraps the value or new_password argument or prompts user for input
fn unwrap_or_input(value: &Option<Option<String>>) -> Result<String, dialoguer::Error> {
    if let Some(Some(v)) = value {
        Ok(v.clone())
    } else {
        user_input(PROPERTY_INPUT_PROMPT)
    }
}

/// unwraps a value from args or prompts user for password with confirmation
fn unwrap_or_new_password(
    value: &Option<Option<String>>,
    force: &bool,
) -> Result<String, dialoguer::Error> {
    if let Some(Some(v)) = value {
        Ok(v.clone())
    } else {
        prompt_password(NEW_PASSWORD_INPUT_PROMPT, true, force)
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
fn prompt_password(prompt: &str, confirm: bool, force: &bool) -> Result<String, dialoguer::Error> {
    if confirm && !*force {
        Password::new()
            .with_prompt(prompt)
            .report(false)
            .with_confirmation("Confirm password", "Mismatched password")
            .interact()
    } else {
        Password::new().with_prompt(prompt).report(false).interact()
    }
}

/// generates a password with ascii values between 33-126
/// barring any characters that are disallowed
fn gen_passwd(len: &usize, disallow: &str, hide: &bool) -> String {
    let mut rng = thread_rng();

    let disallow: Vec<u8> = disallow
        .split(",,")
        .flat_map(|s| match s {
            "symbol" => (33..=47)
                .chain(58..=64)
                .chain(91..=96)
                .chain(123..=126)
                .collect::<Vec<u8>>(),
            "digit" => (48..=57).collect(),
            "uppercase" => (65..=90).collect(),
            "lowercase" => (97..=122).collect(),
            _ => s.as_bytes().to_vec(),
        })
        .collect();

    let allowed_chars: Vec<u8> = (33..=126).filter(|&c| !disallow.contains(&c)).collect();

    let password: String = repeat_with(|| allowed_chars[rng.gen_range(0..allowed_chars.len())])
        .take(*len)
        .map(|c| c as char)
        .collect();

    if !*hide {
        println!("{}", password);
    }

    password
}
