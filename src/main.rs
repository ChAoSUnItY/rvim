#[macro_use]
extern crate defer;
extern crate console;
extern crate termios;

mod rvim;

use rvim::EDITOR;
use std::{env, fs, io::Error};

fn main() -> Result<(), Error> {
    let file_path = &env::args().collect::<Vec<_>>()[1];
    let content = fs::read_to_string(file_path)?;

    EDITOR.lock().unwrap().reset();

    EDITOR.lock().unwrap().start_interactive(file_path)
}

fn loop_term() {}
