use anyhow::Result;
use orion::{aead, errors, kdf, pwhash};
use rand::{thread_rng, Rng};
use serde_json;
use std::fs::File;
use std::io::{self, Read, Write};
use std::num::NonZeroU32;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::{fs, thread};

use crate::{Accounts, App};

const KEY_SIZE: u32 = 32;
const SALT_SIZE: usize = 16;
const KDF_ITERATIONS: u32 = 3;

pub fn write_encrypted_file(app: &App) -> Result<()> {
    // generate salt and derive key
    let password = kdf::Password::from_slice(app.master_pass.as_bytes())?;
    let salt = kdf::Salt::default();
    let key = kdf::derive_key(&password, &salt, KDF_ITERATIONS, 1 << 16, KEY_SIZE)?;

    // encrypt passwords
    let passwords = serde_json::to_vec(&app.passwords)?;
    let ciphertext = aead::seal(&key, &passwords)?;

    // write data to file with salt unencrypted
    let file_data = serde_json::to_vec(&(salt, ciphertext))?;
    let mut file = File::create(&app.args.path)?;
    file.write_all(&file_data)?;
    Ok(())
}

pub fn read_encrypted_file(password: &String, path: &PathBuf) -> Result<Accounts> {
    // read raw file
    let mut file_data = Vec::new();
    File::open(path)?.read_to_end(&mut file_data)?;
    let (salt, passwords): (Vec<u8>, Vec<u8>) = serde_json::from_slice(&file_data)?;

    // derive key from password and salt
    let salt = kdf::Salt::from_slice(&salt)?;
    let password = kdf::Password::from_slice(password.as_bytes())?;
    let key = kdf::derive_key(&password, &salt, KDF_ITERATIONS, 1 << 16, KEY_SIZE)?;

    // decrypt and deserialize passwords
    let plaintext = aead::open(&key, &passwords)?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(test)]
mod crypto_tests {
    use std::collections::HashMap;

    use crate::config::{Args, Config};

    use super::*;

    #[test]
    fn test_io() {
        let mut app = App {
            args: Args::default().configure(Config::default()).unwrap(),
            master_pass: String::from("crypto test password"),
            passwords: HashMap::from([
                (
                    String::from("account 1"),
                    HashMap::from([
                        (String::from("pass2"), String::from("thisispass2")),
                        (String::from("pass"), String::from("thisispass1")),
                    ]),
                ),
                (
                    String::from("account 2"),
                    HashMap::from([
                        (String::from("pass2"), String::from("thisispass2")),
                        (String::from("pass"), String::from("thisispass1")),
                    ]),
                ),
            ]),
        };

        app.args.path = PathBuf::from("test/crypt_test_file");

        write_encrypted_file(&app).unwrap();
        let decrypted_passwords = read_encrypted_file(&app.master_pass, &app.args.path).unwrap();

        assert_eq!(app.passwords, decrypted_passwords);
    }
}
