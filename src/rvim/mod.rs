use std::{
    fs::File,
    io::{stdout, BufWriter, Error, Write},
    sync::Mutex,
};

use crossterm::{
    cursor::MoveTo,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{enable_raw_mode, size, Clear, ClearType},
    ExecutableCommand,
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
        if self.cursor > 0 {
            self.data.remove(self.cursor);
            self.cursor -= 1;
            self.recompute_size();
        }
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

    fn rerender(&mut self, insert: bool) -> Result<(), Error> {
        let mut stdout = stdout();
        stdout
            .execute(Clear(ClearType::Purge))? 
            .execute(MoveTo(0, 0))?;

        // TODO: We should store history first then recover it after rvim finished.

        let (width, height) = size().map(|(w, h)| (w as usize, h as usize))?;
        let cursor_column = self.current_line();
        let mut cursor_row = self.cursor - self.lines[cursor_column].begin;

        if cursor_column < self.view_row {
            self.view_row = cursor_row;
        }

        if cursor_column >= self.view_row + height {
            self.view_row = cursor_column - height + 1;
        }

        for i in 0..height - 1 {
            let row = self.view_row + i;

            if row < self.lines.len() {
                let &Line { begin, end } = &self.lines[row];
                let mut line_size = end - begin;

                if line_size > width {
                    line_size = width;
                }

                write!(
                    &mut stdout,
                    "{}\r\n",
                    self.data[begin..begin + line_size]
                        .iter()
                        .collect::<String>()
                )?;
            } else {
                write!(&mut stdout, "~\r\n")?;
            }
        }

        if cursor_row > width {
            cursor_row = width
        }

        stdout.execute(MoveTo(
            (cursor_row) as u16,
            (cursor_column - self.view_row) as u16,
        ))?;

        Ok(())
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
            self.rerender(insert)?;

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
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::NONE,
                        kind: _,
                        state: _,
                    }) => {
                        self.remove_char();
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
                            if self.cursor < self.data.len() - 1 {
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
