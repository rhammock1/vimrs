use std::{cmp, env, fs, io, path::PathBuf};
use std::io::Write;
use std::time::Duration;
use crossterm::{cursor, event, execute, terminal, queue};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::{Colorize, ColoredString};

struct Config {
  version: f32,
  poll_timeout: Duration,
}

const CONFIG: Config = Config {
  version: 0.20,
  poll_timeout: Duration::from_millis(1500),
};

/*  

    CLEAN UP STRUCTURE

*/
pub struct CleanUp;

impl Drop for CleanUp {
  // Implement Drop for this struct so that when it goes out of scope,
  // this function automatically runs
  fn drop(&mut self) {
    terminal::disable_raw_mode().expect("Failed to disable RAW mode.");
    Output::clear_screen().expect("Failed to clear screen.");
  }
}

/*  

    READER STRUCTURE

*/
pub struct Reader;

impl Reader {
  fn read(&self) -> crossterm::Result<KeyEvent> {
    loop {
      if event::poll(CONFIG.poll_timeout)? {
        if let Event::Key(event) = event::read()? {
          return Ok(event);
        }
      }
    }
  }
}

/*  

    OUTPUT STRUCTURE

*/
struct Output {
  window_size: (usize, usize), // screen_columns: 0, screen_rows: 1
  editor_contents: EditorContents,
  editor_rows: EditorRows,
  cursor_position: (usize, usize), // 0: x, 1: y
  row_offset: usize,
  column_offset: usize,
}

impl Output {
  fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize))
      .unwrap();
    Self {
      window_size,
      editor_contents: EditorContents::new(),
      editor_rows: EditorRows::new(),
      cursor_position: (0, 0),
      row_offset: 0,
      column_offset: 0,
    }
  }

  fn scroll(&mut self) {
    self.row_offset = cmp::min(self.row_offset, self.cursor_position.1);
    if self.cursor_position.0 >= self.row_offset + self.window_size.1 {
      self.row_offset = self.cursor_position.1 - self.window_size.1 + 1;
    }

    self.column_offset = cmp::min(self.column_offset, self.cursor_position.0);
    if self.cursor_position.0 >= self.column_offset + self.window_size.0 {
      self.column_offset = self.cursor_position.0 - self.window_size.0 + 1;
    }
  }

  fn clear_screen() -> crossterm::Result<()> {
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))
  }

  fn refresh_screen(&mut self) -> crossterm::Result<()> {
    self.scroll();
    queue!(
      self.editor_contents,
      cursor::Hide,
      terminal::Clear(terminal::ClearType::All),
      cursor::MoveTo(0, 0),
    )?;

    self.draw_rows();

    let cursor_x = self.cursor_position.0;
    let cursor_y = self.cursor_position.1 - self.row_offset;

    queue!(
      self.editor_contents,
      cursor::MoveTo(cursor_x as u16, cursor_y as u16),
      cursor::Show,
    )?;
    self.editor_contents.flush()
  }

  fn draw_rows(&mut self) {
    let screen_columns = self.window_size.0;
    let screen_rows = self.window_size.1;

    for i in 0..screen_rows {
      let file_row = i + self.row_offset;
      if file_row >= self.editor_rows.number_of_rows() {
        if self.editor_rows.number_of_rows() == 0 && i == screen_rows / 3 {
          let mut welcome = format!("Vimrs --- Version {}\r\n", CONFIG.version);
          if welcome.len() > screen_columns {
            welcome.truncate(screen_columns);
          }
          let mut welcome_padding = (screen_columns - welcome.len()) / 2;
          if welcome_padding != 0 {
            self.editor_contents.push('~');
            welcome_padding -= 1;
          }
          (0..welcome_padding).for_each(|_| self.editor_contents.push(' '));
          self.editor_contents.push_str(&welcome);

          let mut description = String::from("A text editor written in Rust\r\n");
          if description.len() > screen_columns {
            description.truncate(screen_columns);
          }
          let mut description_padding = (screen_columns - description.len()) / 2;
          if description_padding != 0 {
            self.editor_contents.push('~');
            description_padding -= 1;
          }
          (0..description_padding).for_each(|_| self.editor_contents.push(' '));
          self.editor_contents.push_str(&description);
          self.editor_contents.push('~');
        } else {
          self.editor_contents.push('~');
        }
      } else {
        let row = self.editor_rows.get_row(file_row);
        let column_offset = self.column_offset;
        let len = cmp::min(row.len().saturating_sub(column_offset), screen_columns);
        let start = if len == 0 { 0 } else { column_offset };
        self.editor_contents.push_str(&row[start..start + len]);
      }
      queue!(
        self.editor_contents,
        terminal::Clear(terminal::ClearType::UntilNewLine),
      ).unwrap();
      if i < screen_rows - 1 {
        self.editor_contents.push_str("\r\n");
      }
    }
  }

  fn move_cursor(&mut self, direction: KeyCode, number_of_rows: usize) {
    match direction {
      KeyCode::Up => {
        self.cursor_position.1 = self.cursor_position.1.saturating_sub(1);
      }
      KeyCode::Down => {
        if self.cursor_position.1 < number_of_rows {
          self.cursor_position.1 += 1;
        }
      }
      KeyCode::Left => {
        if self.cursor_position.0 != 0 {
          self.cursor_position.0 -= 1;
        }
      }
      KeyCode::Right => {
        // if self.cursor_position.0 != self.window_size.0 - 1 {
          self.cursor_position.0 += 1;
        // }
      }
      KeyCode::End => self.cursor_position.0 = self.window_size.0 - 1,
      KeyCode::Home => self.cursor_position.0 = 0,
      _ => unimplemented!("Invalid keypress"),
    }
  }
}

/*  

    EDITOR STRUCTURE

*/
pub struct Editor {
  reader: Reader,
  output: Output,
}

impl Editor {
  pub fn new() -> crossterm::Result<Self> {
    // Enable terminal's raw mode
    terminal::enable_raw_mode()?;  
    Ok(Self {
      reader: Reader,
      output: Output::new(),
    })
  }

  pub fn run(&mut self) -> crossterm::Result<bool> {
    self.output.refresh_screen()?;
    self.process_keypress()
  }

  fn process_keypress(&mut self) -> crossterm::Result<bool> {
    match self.reader.read()? {
      KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: event::KeyModifiers::CONTROL,
        ..
      } => return Ok(false),
      KeyEvent {
        code: direction @ (
          KeyCode::Up 
          | KeyCode::Down 
          | KeyCode::Left 
          | KeyCode::Right
          | KeyCode::Home
          | KeyCode::End
        ),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => self.output.move_cursor(direction, self.output.editor_rows.number_of_rows()),
      KeyEvent {
        code: val @ (KeyCode::PageUp | KeyCode::PageDown),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => (0..self.output.window_size.1).for_each(|_| {
          self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
            KeyCode::Up
          } else {
            KeyCode::Down
          }, self.output.editor_rows.number_of_rows());
        }),
      _ => {},
    }
    Ok(true)
  }
}

/*  

    Editor Contents Structure

*/
struct EditorContents {
  content: String,
}

impl EditorContents {
  fn new() -> Self {
    Self {
      content: String::new(),
    }
  }

  fn push(&mut self, ch: char) {
    self.content.push(ch)
  }

  fn push_str(&mut self, string: &str) {
    self.content.push_str(string)
  }
}

impl io::Write for EditorContents {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    match std::str::from_utf8(buf) {
      Ok(s) => {
        self.content.push_str(s);
        Ok(s.len())
      },
      Err(_) => Err(io::ErrorKind::WriteZero.into()),
    }
  }

  fn flush(&mut self) -> io::Result<()> {
    let out = write!(io::stdout(), "{}", self.content);
    io::stdout().flush()?;
    self.content.clear();
    out
  }
}

/*

    Editor Rows Structure

*/
struct EditorRows {
  row_contents: Vec<Box<str>>,
}

impl EditorRows {
  fn new() -> Self {
    let mut arg = env::args();
    
    match arg.nth(1) {
      None => Self {
        row_contents: Vec::new(),
      },
      Some(file) => Self::from_file(file.into()),
    }
  }

  fn from_file(file: PathBuf) -> Self {
    let file_contents = fs::read_to_string(file).expect("Unable to read file.");
    Self {
      row_contents: file_contents.lines().map(|s| s.into()).collect(),
    }
  }

  fn number_of_rows(&self) -> usize {
    self.row_contents.len()
  }

  fn get_row(&self, at: usize) -> &str {
    &self.row_contents[at]
  }
}