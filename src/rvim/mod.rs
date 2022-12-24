use std::{sync::Mutex, io::{Error, stdin, Read, BufWriter, Write}, fs::File};

use console::Term;
use defer::defer;
use once_cell::sync::Lazy;
use termios::{Termios, tcgetattr, tcsetattr, ECHO, ICANON};

pub static EDITOR: Lazy<Mutex<Editor>> = Lazy::new(|| Mutex::new(Editor{
    data: vec![],
    lines: vec![],
    cursor: 0
}));

#[derive(Debug)]
pub struct Line {
    begin: usize,
    end: usize
}

#[derive(Debug)]
pub struct Editor {
    data: Vec<char>,
    lines: Vec<Line>,
    cursor: usize
}

impl Editor {
    pub fn reset(&mut self) {
        self.data.clear();
        self.lines.clear();
        self.cursor = 0;
    }

    fn recompute_size(&mut self) {
        let mut line_count = 0;
        let mut begin = 0usize;

        for (i, &char) in self.data.iter().enumerate() {
            if char == '\n' {
                let mut line = &mut self.lines[line_count];

                line.begin = begin;
                line.end = i;
                begin = i + 1;
            }
        }

        let mut last_line = &mut self.lines[line_count];
        last_line.begin = begin;
        last_line.end = self.data.len();
    }

    fn insert_char(&mut self, char: char) {
        self.data.insert(self.cursor, char);
        self.cursor += 1;
        self.recompute_size();
    }

    fn current_line(&self) -> usize {
        assert!(self.cursor <= self.data.len() - 1);

        for (i, line) in self.lines.iter().enumerate() {
            if line.begin <= self.cursor && self.cursor <= line.end {
                return i;
            }
        }

        0
    }

    fn rerender(&self, insert: bool) {
        print!("\033[2J\033[H");

        for char in &self.data {
            print!("{}", char);
        }

        println!();

        if insert {
            print!("[INSERT]");
        }

        let line = self.current_line();
        print!("\033[{};{}H", line + 1, self.cursor - self.lines[line].begin + 1);
    }

    fn save_to_file<'a>(&self, file_path: &'a str) -> Result<(), Error> {
        let file = File::open(file_path)?;
        let mut buf_writer = BufWriter::new(file);

        for char in &self.data {
            buf_writer.write_all(char.to_string().as_bytes())?;
        }

        Ok(())
    }

    pub fn start_interactive<'a>(&mut self, file_path: &'a str) -> Result<(), Error> {
        let mut stdin = stdin();
        let mut term = Term::stdout();
        let mut termios = Termios::from_fd(0)?;

        let mut term_booted = false;
        let mut quit = false;
        let mut insert = false;

        defer(|| {
            if term_booted {
                println!("\033[2J");
                termios.c_lflag |= ECHO;
                tcsetattr(0, 0, &termios); // Ignored
            }
        });

        if let Err(err) = tcgetattr(0, &mut termios) {
            println!("ERROR: Could not get status of terminal");
            return Err(err);
        }

        termios.c_lflag &= !ECHO;
        termios.c_lflag &= !ICANON;

        if let Err(err) = tcsetattr(0, 0, &termios) {
            println!("ERROR: Could not update status of terminal");
            return Err(err);
        }

        term_booted = true;

        while !quit {
            self.rerender(insert);

            if insert {
                let char = Term::read_char(&term)?;

                if char == '\x27' {
                    insert = false;

                    self.save_to_file(file_path)?;
                } else {
                    self.insert_char(char);
                }
            } else {
                let char = Term::read_char(&term)?;

                match char as char {
                    'q' => quit = true,
                    'e' => insert = true,
                    
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
