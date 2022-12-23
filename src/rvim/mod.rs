use std::{sync::Mutex, io::{Error, stdin, Read}};

use defer::defer;
use once_cell::sync::Lazy;
use termios::{Termios, tcgetattr, tcsetattr, ECHO, ICANON};

pub static EDITOR: Lazy<Mutex<Editor>> = Lazy::new(|| Mutex::new(Editor{
    char: vec![],
    data_count: 0,
    lines: vec![],
    lines_count: 0,
    cursor: 0
}));

#[derive(Debug)]
pub struct Line {
    begin: usize,
    end: usize
}

#[derive(Debug)]
pub struct Editor {
    char: Vec<char>,
    data_count: usize,
    lines: Vec<Line>,
    lines_count: usize,
    cursor: usize
}

impl Editor {
    pub fn update_data_count(&mut self, data_count: usize) {
        self.data_count = data_count;
    }

    pub fn start_interactive<'a>(&mut self, file_path: &'a str) -> Result<(), Error> {
        let mut stdin = stdin();
        let mut term = Termios::from_fd(0)?;

        let mut term_booted = false;
        let mut quit = false;
        let mut insert = false;

        defer(|| {
            if term_booted {
                println!("\033[2J");
                term.c_lflag |= ECHO;
                tcsetattr(0, 0, &term); // Ignored
            }
        });

        if let Err(err) = tcgetattr(0, &mut term) {
            println!("ERROR: Could not get status of terminal");
            return Err(err);
        }

        term.c_lflag &= !ECHO;
        term.c_lflag &= !ICANON;

        if let Err(err) = tcsetattr(0, 0, &term) {
            println!("ERROR: Could not update status of terminal");
            return Err(err);
        }

        term_booted = true;

        while !quit {
            let mut char_buffer = [0u8; 1];

            if insert {
                stdin.read_exact(&mut char_buffer)?;

                if char_buffer[0] == 27 {
                    insert = false;
                } else {

                }
            } else {
                stdin.read_exact(&mut char_buffer)?;

                match char_buffer[0] as char {
                    'q' => quit = true,
                    'e' => insert = true,
                    
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
