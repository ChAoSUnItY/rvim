extern crate crossterm;
extern crate termios;

mod rvim;

use rvim::EDITOR;
use std::{env, fs, io::Error};

fn main() -> Result<(), Error> {
    let file_path = &env::args().collect::<Vec<_>>()[1];
    let content = fs::read_to_string(file_path)?;

    EDITOR.lock().unwrap().init(&content);

    EDITOR.lock().unwrap().start_interactive(file_path)
}
