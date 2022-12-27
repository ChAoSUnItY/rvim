use std::{
    fs::File,
    io::{BufWriter, Error, Write},
    sync::Mutex,
};

use crossterm::{
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::enable_raw_mode,
};
use once_cell::sync::Lazy;
use termios::{tcgetattr, tcsetattr, Termios, ECHO, ICANON};

pub static EDITOR: Lazy<Mutex<Editor>> = Lazy::new(|| {
    Mutex::new(Editor {
        data: vec![],
        lines: vec![],
        cursor: 0,
        view_row: 0,
    })
});

#[derive(Debug)]
pub struct Line {
    begin: usize,
    end: usize,
}

impl Line {
    pub fn new(begin: usize, end: usize) -> Self {
        Self { begin, end }
    }
}

#[derive(Debug)]
pub struct Editor {
    data: Vec<char>,
    lines: Vec<Line>,
    cursor: usize,
    view_row: usize,
}

impl Editor {
    pub fn reset(&mut self) {
        self.data.clear();
        self.lines.clear();
        self.cursor = 0;
    }

    pub fn init<T>(&mut self, content: &T)
    where
        T: AsRef<str>,
    {
        self.reset();

        for char in content.as_ref().chars() {
            self.data.push(char);
        }
    }

    fn recompute_size(&mut self) {
        let mut begin = 0;

        for (i, &char) in self.data.iter().enumerate() {
            if char == '\n' {
                self.lines.push(Line::new(begin, i));

                begin = i + 1;
            }
        }

        self.lines.push(Line::new(begin, self.data.len()));
    }

    fn insert_char(&mut self, char: char) {
        self.data.insert(self.cursor, char);
        self.cursor += 1;
        self.recompute_size();
    }

    fn remove_char(&mut self) {
        self.data.remove(self.cursor);
        self.cursor -= 1;
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
        print!("\x1B[2J\x1B[H");

        for char in &self.data {
            print!("{}", char);
        }

        println!();

        if insert {
            print!("[INSERT]");
        }

        let line = self.current_line();
        print!(
            "\x1B[{};{}H",
            line + 1,
            self.cursor - self.lines[line].begin + 1
        );
    }

    fn save_to_file<'a>(&self, file_path: &'a str) -> Result<(), Error> {
        let file = File::create(file_path)?;
        let mut buf_writer = BufWriter::new(file);

        for char in &self.data {
            buf_writer.write_all(char.to_string().as_bytes())?;
        }

        buf_writer.flush()?;

        Ok(())
    }

    pub fn start_interactive<'a>(&mut self, file_path: &'a str) -> Result<(), Error> {
        let mut termios = Termios::from_fd(0)?;
        let mut quit = false;
        let mut insert = false;

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

        enable_raw_mode()?;

        self.recompute_size();

        while !quit {
            self.rerender(insert);

            if insert {
                match read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        modifiers: KeyModifiers::NONE,
                        kind: _,
                        state: _,
                    }) => {
                        insert = false;

                        self.save_to_file(file_path)?;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(key_code),
                        modifiers: KeyModifiers::NONE,
                        kind: _,
                        state: _,
                    }) => {
                        self.insert_char(key_code);
                    }
                    _ => {}
                };
            } else {
                match read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(code),
                        modifiers: _,
                        kind: _,
                        state: _,
                    }) => match code {
                        'q' => quit = true,
                        'e' => insert = true,
                        's' => {
                            let line = self.current_line();
                            let column = self.cursor - self.lines[line].begin;

                            if line < self.lines.len() - 1 {
                                self.cursor = self.lines[line + 1].begin + column;

                                if self.cursor > self.lines[line + 1].end {
                                    self.cursor = self.lines[line + 1].end;
                                }
                            }
                        }
                        'w' => {
                            let line = self.current_line();
                            let column = self.cursor - self.lines[line].begin;

                            if line > 0 {
                                self.cursor = self.lines[line + 1].begin + column;

                                if self.cursor > self.lines[line + 1].end {
                                    self.cursor = self.lines[line + 1].end;
                                }
                            }
                        }
                        'a' => {
                            if self.cursor > 0 {
                                self.cursor -= 1;
                            }
                        }
                        'd' => {
                            if self.cursor < self.data.len() {
                                self.cursor += 1;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        print!("\x1B[2J");
        termios.c_lflag |= ECHO;
        tcsetattr(0, 0, &termios)?;

        Ok(())
    }
}
