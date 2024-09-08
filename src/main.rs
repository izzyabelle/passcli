use passcli::run;
use rand::prelude::{thread_rng, Rng};
use std::{iter::repeat_with, process::exit};

fn main() {
    let ret = run().expect("hihi");
    exit(ret);
}

// generates a password with ascii values between 33-126
// fn gen_passwd(len: usize, args: Args) -> String {
//     let mut rng = thread_rng();
//     String::from_utf8(repeat_with(|| rng.gen_range(33..=126)).take(len).collect())
//         // cannot possibly have err value
//         .unwrap()
// }
