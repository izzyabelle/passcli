use anyhow::Result;
use orion::{aead, kdf, pwhash};
use rand::{thread_rng, Rng};
use serde_json;
use std::fs::File;
use std::io::{self, Read, Write};
use std::num::NonZeroU32;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::{fs, thread};

use crate::{App, Passwords};

const KEY_SIZE: u32 = 32;
const SALT_SIZE: usize = 16;
const KDF_ITERATIONS: u32 = 100_000;

pub fn write_encrypted_file(app: &App) -> Result<()> {
    let password = kdf::Password::from_slice(&app.master_pass.as_bytes())?;

    // generate 16 byte salt
    let salt = kdf::Salt::default();

    // derive key from password
    let key = kdf::derive_key(&password, &salt, KDF_ITERATIONS, 1 << 16, KEY_SIZE)?;

    // serialise passwords and salt
    let data = serde_json::to_vec(&app.passwords)?;
    let salt_data = serde_json::to_vec(&salt)?;

    // encrypt data
    let ciphertext = aead::seal(&key, &data)?;

    // write data to file
    let mut file = File::create(&app.path)?;
    file.write_all(&salt_data)?;
    file.write_all(&ciphertext)?;

    Ok(())
}

pub fn read_encrypted_file(password: &String, path: &PathBuf) -> Result<Passwords> {
    // read raw file
    let mut file_data = Vec::new();
    fs::File::open(&path)?.read_to_end(&mut file_data)?;
    let file_data = file_data;

    // split salt from password data
    let (salt, data) = file_data.split_at(SALT_SIZE);
    let salt = kdf::Salt::from_slice(&salt)?;

    // derive key from password and salt
    let password = kdf::Password::from_slice(&password.as_bytes())?;
    let key = kdf::derive_key(&password, &salt, KDF_ITERATIONS, 1 << 16, KEY_SIZE)?;

    // decrypt data
    let plaintext = aead::open(&key, &data)?;

    // deserialise data
    Ok(serde_json::from_slice(&plaintext)?)
}
