#[macro_use]
extern crate defer;
extern crate termios;

mod rvim;

use rvim::EDITOR;
use std::{env, fs, io::Error};

fn main() -> Result<(), Error> {
    let file_path = &env::args().collect::<Vec<_>>()[1];
    let content = fs::read_to_string(file_path)?;

    EDITOR.lock().unwrap().update_data_count(content.len());

    println!("{:?}", EDITOR.lock().unwrap());

    Ok(())
}

fn loop_term() {}
