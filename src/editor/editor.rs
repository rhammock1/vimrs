use std::{cmp, io, env, fs, path::PathBuf, time::{Duration, Instant}};
use std::io::Write;
use crossterm::{event, terminal, queue};
use crossterm::event::{KeyCode, KeyEvent};
use colored::{Colorize, ColoredString};

use crate::{
  log, 
  prompt,
  Reader,
  CONFIG,
};
use super::output::Output;

pub struct Editor {
  pub reader: Reader,
  pub output: Output,
  previous_3_keys: Vec<KeyCode>,
}

impl Editor {
  pub fn new() -> crossterm::Result<Self> {
    // Enable terminal's raw mode
    terminal::enable_raw_mode()?;
    // Enter alternate screen
    queue!(
      io::stdout(),
      terminal::EnterAlternateScreen,
    )?;
    Ok(Self {
      reader: Reader,
      output: Output::new(),
      previous_3_keys: Vec::new(),
    })
  }

  pub fn run(&mut self) -> crossterm::Result<bool> {
    self.output.refresh_screen()?;
    self.process_keypress()
  }

  pub fn set_previous_key(&mut self, key: KeyCode) {
    self.previous_3_keys.push(key);
    if self.previous_3_keys.len() > 3 {
      self.previous_3_keys.remove(0);
    }
  }

  pub fn process_keypress(&mut self) -> crossterm::Result<bool> {
    match self.reader.read()? {
      /* Cursor Control */
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
      } => {
        log::log::log("INFO".to_string(), format!("Moving cursor in direction: {:?}", direction));
        self.set_previous_key(direction);
        self.output.move_cursor(direction)
      },
      KeyEvent {
        code: val @ (KeyCode::PageUp | KeyCode::PageDown),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        log::log::log("INFO".to_string(), format!("Moving cursor in direction: {:?}", val));
        self.set_previous_key(val);
        if matches!(val, KeyCode::PageUp) {
          self.output.cursor_controller.cursor_y = self.output.cursor_controller.row_offset;
        } else {
          self.output.cursor_controller.cursor_y = cmp::min(
            self.output.window_size.1 + self.output.cursor_controller.row_offset - 1,
            self.output.editor_rows.number_of_rows(),
          );
        }
        (0..self.output.window_size.1).for_each(|_| {
          self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
            KeyCode::Up
          } else {
            KeyCode::Down
          });
        })
      },
      /* End Cursor Control */
      /* Flow Control */
      KeyEvent {
        code: KeyCode::Char(':'),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        self.set_previous_key(KeyCode::Char(':'));
      },
      KeyEvent {
        code: KeyCode::Char('f'),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        log::log::log("INFO".to_string(), "Activating find mode.".to_string());
        if self.previous_3_keys.last() == Some(&KeyCode::Char(':')) {
          self.set_previous_key(KeyCode::Char('f'));
          self.output.find()?;
        } else {
          self.set_previous_key(KeyCode::Char('f'));
          self.output.insert_character('f')
        }
      }
      KeyEvent {
        code: KeyCode::Char('w'),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        log::log::log("INFO".to_string(), "Saving file.".to_string());
        // TODO- Check that a filename has been provided, if not, prompt for one
        if self.previous_3_keys.last() == Some(&KeyCode::Char(':')) {
          if matches!(self.output.editor_rows.filename, None) {
            let prompt = prompt!(&mut self.output, "Save as: {}")
              .map(|it| it.into());

            if let None = prompt {
              self.output
                .status_message
                .set_message("Save aborted".into());
              return Ok(true);
            }
            self.output.editor_rows.filename = prompt;
          }
          self.output.editor_rows.save()?;
          self.output.status_message.set_message("File saved.".to_string());
          self.output.dirty = false;
        } else {
          self.set_previous_key(KeyCode::Char('w'));
          self.output.insert_character('w')
        }
      },
      KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        log::log::log("INFO".to_string(), "Exiting editor.".to_string());
        if self.previous_3_keys.last() == Some(&KeyCode::Char('w'))
          && self.previous_3_keys.get(1) == Some(&KeyCode::Char(':')) {
          // This is already saved so we can exit
          return Ok(false);
        } else if self.previous_3_keys.last() == Some(&KeyCode::Char(':')) {
          if self.output.dirty {
            log::log::log("INFO".to_string(), "File has unsaved changes.".to_string());
            self.set_previous_key(KeyCode::Char('q'));
            self.output.status_message.set_message("File has unsaved changes. Press :q! to exit without saving.".to_string())
          } else {
            return Ok(false);
          }
        } else {
          self.set_previous_key(KeyCode::Char('q'));
          self.output.insert_character('q')
        }
      },
      KeyEvent {
        code: KeyCode::Char('!'),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        if self.previous_3_keys.last() == Some(&KeyCode::Char('q'))
          && self.previous_3_keys.get(1) == Some(&KeyCode::Char(':')) {
          log::log::log("INFO".to_string(), "Exiting without saving.".to_string());
          return Ok(false);
        } else {
          self.set_previous_key(KeyCode::Char('!'));
          self.output.insert_character('!')
        }
      }
      /* End Flow Control */
      /* Text Control */
      KeyEvent {
        code: KeyCode::Enter,
        modifiers: event::KeyModifiers::NONE,
        ..
      } => self.output.insert_newline(),
      KeyEvent {
        code: key @ (KeyCode::Backspace | KeyCode::Delete),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        if matches!(key, KeyCode::Delete) {
          self.output.move_cursor(KeyCode::Right)
        }
        self.output.delete_character()
      },
      KeyEvent {
        code: code @ (KeyCode::Char(..) | KeyCode::Tab),
        modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
        ..
      } => {
        self.set_previous_key(match code {
          KeyCode::Char(ch) => KeyCode::Char(ch),
          KeyCode::Tab => KeyCode::Tab,
          _ => unreachable!(),
        });
        self.output.insert_character(match code {
          KeyCode::Char(ch) => ch,
          KeyCode::Tab => '\t',
          _ => unreachable!(),
        })
      },
      /* End Text Control */
      _ => {},
    }
    Ok(true)
  }
}

pub struct EditorContents {
  pub content: String,
}

impl EditorContents {
  pub fn new() -> Self {
    Self {
      content: String::new(),
    }
  }

  pub fn push(&mut self, ch: char) {
    self.content.push(ch)
  }

  pub fn push_str(&mut self, string: &str) {
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

#[derive(Default)]
pub struct Row {
  pub row_content: String,
  pub render: String,
}

impl Row {
  pub fn new(row_content: String, render: String) -> Self {
    Self {
      row_content,
      render,
    }
  }

  pub fn get_row_content_x(&self, render_x: usize) -> usize {
    let mut current_render_x = 0;
    for(cursor_x, character) in self.row_content.chars().enumerate() {
      if character == '\t' {
        current_render_x += (CONFIG.spaces_per_tab - 1) - (current_render_x % CONFIG.spaces_per_tab);
      }
      current_render_x += 1;
      if current_render_x > render_x {
        return cursor_x;
      }
    }
    0
  }

  pub fn insert_character(&mut self, at: usize, character: char) {
    self.row_content.insert(at, character);
    EditorRows::render_row(self)
  }

  pub fn delete_character(&mut self, at: usize) {
    self.row_content.remove(at);
    EditorRows::render_row(self)
  }
}

pub struct EditorRows {
  pub row_contents: Vec<Row>,
  pub filename: Option<PathBuf>,
  pub file_size: Option<u64>,
}

impl EditorRows {
  pub fn new() -> Self {
    let mut arg = env::args();
    
    match arg.nth(1) {
      None => Self {
        row_contents: Vec::new(),
        filename: None,
        file_size: None,
      },
      Some(file) => Self::from_file(file.into()),
    }
  }

  pub fn join_adjacent_rows(&mut self, at: usize) {
    let current_row = self.row_contents.remove(at);
    let previous_row = self.get_editor_row_mut(at - 1);

    previous_row.row_content.push_str(&current_row.row_content);
    Self::render_row(previous_row);
  }

  pub fn save(&mut self) -> io::Result<()> {
    match &self.filename {
      None => Err(io::Error::new(io::ErrorKind::Other, "No filename specified.")),
      Some(name) => {
        let mut file = fs::OpenOptions::new()
          .write(true)
          .create(true)
          .open(name)?;

        let contents: String = self
          .row_contents
          .iter()
          .map(|it| it.row_content.as_str())
          .collect::<Vec<&str>>()
          .join("\n");

        let size = contents.as_bytes().len() as u64;
        file.set_len(size)?;
        self.file_size = Some(size);
        file.write_all(contents.as_bytes())
      }
    }
  }

  pub fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
    &mut self.row_contents[at]
  }

  pub fn insert_row(&mut self, at: usize, contents: String) {
    let mut new_row = Row::new(contents, String::new());

    Self::render_row(&mut new_row);
    self.row_contents.insert(at, new_row);
  }

  pub fn from_file(file: PathBuf) -> Self {
    // Create the file if it doesn't exist
    fs::OpenOptions::new()
      .write(true)
      .create(true)
      .read(true)
      .open(&file)
      .expect("Unable to create file.");

    // Convert file_contents to string
    let file_contents = fs::read_to_string(&file).expect("Unable to read file.");

    Self {
      filename: Some(file),
      row_contents: file_contents
        .lines()
        .map(|s| {
          let mut row = Row::new(s.into(), String::new());
          Self::render_row(&mut row);
          row
        })
        .collect(),
      file_size: Some(file_contents.len() as u64),
    }
  }

  pub fn number_of_rows(&self) -> usize {
    self.row_contents.len()
  }

  pub fn get_render(&self, at: usize) -> &String {
    &self.row_contents[at].render
  }

  pub fn get_row(&self, at: usize) -> &str {
    &self.row_contents[at].row_content
  }

  pub fn get_editor_row(&self, at: usize) -> &Row {
    &self.row_contents[at]
  }

  pub fn render_row(row: &mut Row) {
    let mut index = 0;
    let capacity = row
      .row_content
      .chars()
      .fold(0, |acc, next| acc + if next == '\t' { CONFIG.spaces_per_tab } else { 1 });
    row.render = String::with_capacity(capacity);
    row.row_content.chars().for_each(|c| {
      index += 1;
      if c == '\t' {
        row.render.push(' ');
        while index % CONFIG.spaces_per_tab != 0 {
          row.render.push(' ');
          index += 1
        }
      } else {
        row.render.push(c)
      }
    })
  }
}

pub struct StatusMessage {
  pub message: Option<String>,
  pub set_time: Option<Instant>,
}

impl StatusMessage {
  pub fn new(initial_message: String) -> Self {
    Self {
      message: Some(initial_message),
      set_time: Some(Instant::now()),
    }
  }

  pub fn set_message(&mut self, message: String) {
    self.message = Some(message);
    self.set_time = Some(Instant::now());
  }

  pub fn message(&mut self) -> Option<&String> {
    self.set_time
      .and_then(|time| {
        if time.elapsed() > Duration::from_secs(CONFIG.message_timeout) {
          self.message = None;
          self.set_time = None;
          None
        } else {
          Some(self.message.as_ref().unwrap())
        }
      })
  }
}
