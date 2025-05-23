use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use orion::{aead, kdf};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::{Accounts, App};

const KEY_SIZE: u32 = 32;
const SALT_SIZE: usize = 16;

pub fn write_encrypted_file(app: &App) -> Result<i32> {
    // generate salt and derive key
    let password = kdf::Password::from_slice(app.master_pass.as_bytes())?;
    let salt = kdf::Salt::generate(SALT_SIZE)?;
    let key = kdf::derive_key(
        &password,
        &salt,
        app.config.kdf_iterations,
        1 << 16,
        KEY_SIZE,
    )?;

    // encrypt passwords
    let passwords = serde_json::to_vec(&app.passwords)?;
    let ciphertext = aead::seal(&key, &passwords)?;

    // write data to file with salt unencrypted
    let tuple_data = serde_json::to_vec(&(salt, ciphertext))?;

    let encoded_data = general_purpose::STANDARD.encode(&tuple_data);

    let mut file = File::create(app.path.clone())?;
    file.write_all(encoded_data.as_bytes())?;
    Ok(0)
}

pub fn read_encrypted_file(
    password: &String,
    path: &PathBuf,
    kdf_iterations: &u32,
) -> Result<Accounts> {
    // read raw file
    let mut file_data = Vec::new();
    File::open(path)?.read_to_end(&mut file_data)?;
    let decoded_data = general_purpose::STANDARD.decode(file_data)?;
    let (salt, passwords): (Vec<u8>, Vec<u8>) = serde_json::from_slice(&decoded_data)?;

    // derive key from password and salt
    let salt = kdf::Salt::from_slice(&salt)?;
    let password = kdf::Password::from_slice(password.as_bytes())?;
    let key = kdf::derive_key(&password, &salt, *kdf_iterations, 1 << 16, KEY_SIZE)?;

    // decrypt and deserialize passwords
    let plaintext = aead::open(&key, &passwords)?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(test)]
mod crypto_tests {
    use std::collections::HashMap;

    use crate::config::{Args, PassConfig};

    use super::*;

    #[test]
    fn test_io() {
        let mut app = App {
            args: Args::default(),
            config: PassConfig::new().unwrap(),
            path: PathBuf::from("test/passwds"),
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
            interactive: false,
        };

        app.args.path = Some(PathBuf::from("test/crypt_test_file"));

        write_encrypted_file(&app).unwrap();
        let decrypted_passwords =
            read_encrypted_file(&app.master_pass, &app.args.path.unwrap(), &3).unwrap();

        assert_eq!(app.passwords, decrypted_passwords);
    }
}
