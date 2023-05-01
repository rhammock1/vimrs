use std::{cmp, io, env, fs, path::PathBuf, time::{Duration, Instant}};
use std::io::Write;
use crossterm::{event, terminal, queue, style};
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

            if prompt.is_none() {
              self.output
                .status_message
                .set_message("Save aborted".into());
              return Ok(true);
            }
            prompt
              .as_ref()
              .and_then(|path: &PathBuf| path.extension())
              .and_then(|ext| ext.to_str())
              .map(|ext| {
                Output::select_syntax(ext).map(|syntax| {
                  let highlight = self.output.syntax_highlight.insert(syntax);
                  for i in 0..self.output.editor_rows.number_of_rows() {
                    highlight
                      .update_syntax(i, &mut self.output.editor_rows.row_contents)
                  }
                })
              });
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

  pub fn push_str(&mut self, string: &str, str_color: Option<String>) {
    self.content.push_str(
      string.color(
        str_color.unwrap_or(String::from("normal")
      )
    ).to_string().as_str())
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

#[derive(Copy, Clone)]
pub enum HighlightType {
  Normal,
  Number,
  SearchMatch,
  String,
  CharLiteral,
  Comment,
  MultilineComment,
  Other (style::Color),
}

#[derive(Default)]
pub struct Row {
  pub row_content: String,
  pub render: String,
  pub highlight: Vec<HighlightType>,
}

impl Row {
  pub fn new(row_content: String, render: String) -> Self {
    Self {
      row_content,
      render,
      highlight: Vec::new(),
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
  pub fn new(syntax_highlight: &mut Option<Box<dyn SyntaxHighlight>>) -> Self {
    
    match env::args().nth(1) {
      None => Self {
        row_contents: Vec::new(),
        filename: None,
        file_size: None,
      },
      Some(file) => Self::from_file(file.into(), syntax_highlight),
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

  pub fn from_file(file: PathBuf, syntax_highlight: &mut Option<Box<dyn SyntaxHighlight>>) -> Self {
    // Create the file if it doesn't exist
    fs::OpenOptions::new()
      .write(true)
      .create(true)
      .read(true)
      .open(&file)
      .expect("Unable to create file.");

    file.extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| Output::select_syntax(ext).map(|syntax| syntax_highlight.insert(syntax)));

    // Convert file_contents to string
    let file_contents = fs::read_to_string(&file).expect("Unable to read file.");

    let mut row_contents = Vec::new();
    file_contents.lines().enumerate().for_each(|(i, line)| {
      let mut row = Row::new(line.into(), String::new());
      Self::render_row(&mut row);
      row_contents.push(row);
      if let Some(it) = syntax_highlight {
        it.update_syntax(i, &mut row_contents)
      }
    });
    Self {
      filename: Some(file),
      row_contents,
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

pub trait SyntaxHighlight {
  fn extensions(&self) -> &[&str];
  fn file_type(&self) -> &str;
  fn comment_start(&self) -> &str;
  fn multiline_comment(&self) -> Option<(&str, &str)>;
  fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>);
  fn syntax_color(&self, highlight_type: &HighlightType) -> style::Color;
  fn color_row(&self, render: &str, highlight: &[HighlightType], out: &mut EditorContents) {
    let mut current_color = self.syntax_color(&HighlightType::Normal);
    render.char_indices().for_each(|(i, c)| {
      let color = self.syntax_color(&highlight[i]);
      if current_color != color {
        current_color = color;
        let _ = queue!(out, style::SetForegroundColor(color));
      }
      out.push(c);
    });
    let _ = queue!(out, style::ResetColor);
  }
  fn is_separator(&self, c: char) -> bool {
    c.is_whitespace() || [
      ',', '.', '(', ')', '+', '-', '/', '*', '=', '~', '%', '<', '>', '"', '\'', ';', '&',
    ].contains(&c)
  }
}

#[macro_export]
macro_rules! syntax_struct {
  (
    struct $Name:ident {
      extensions: $ext:expr,
      file_type: $type:expr,
      comment_start: $start:expr,
      keywords: {
        $([$color:expr; $($words:expr),*]),*
      },
      multiline_comment:$ml_comment:expr
    }
  ) => {
    struct $Name {
      extensions: &'static [&'static str],
      file_type: &'static str,
      comment_start: &'static str,
      multiline_comment: Option<(&'static str, &'static str)>,
    }

    impl $Name {
      fn new() -> Self {
        Self {
          extensions: &$ext,
          file_type: $type,
          comment_start: $start,
          multiline_comment: $ml_comment,
        }
      }
    }

    impl SyntaxHighlight for $Name {
      fn extensions(&self) -> &[&str] {
        self.extensions
      }

      fn file_type(&self) -> &str {
        self.file_type
      }

      fn comment_start(&self) -> &str {
        self.comment_start
      }

      fn multiline_comment(&self) -> Option<(&str, &str)> {
        self.multiline_comment
      }

      fn syntax_color(&self, highlight_type: &HighlightType) -> style::Color {
        match highlight_type {
          HighlightType::Normal => style::Color::Reset,
          HighlightType::Number => style::Color::Cyan,
          HighlightType::SearchMatch => style::Color::Blue,
          HighlightType::String => style::Color::Green,
          HighlightType::Comment | HighlightType::MultilineComment => style::Color::DarkGrey,
          HighlightType::CharLiteral => style::Color::DarkGreen,
          HighlightType::Other(color) => *color,
        }
      }

      fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>) {
        let current_row = &mut editor_rows[at];

        macro_rules! add {
          ($h:expr) => {
            current_row.highlight.push($h)
          };
        }

        current_row.highlight = Vec::with_capacity(current_row.render.len());

        let render = current_row.render.as_bytes();
        let mut i = 0;
        let mut previous_separater = true;
        let mut in_string: Option<char> = None;
        let comment_start = self.comment_start().as_bytes();
        let mut in_comment = false;

        while i < render.len() {
          let c = render[i] as char;
          let previous_highlight = if i > 0 {
            current_row.highlight[i - 1]
          } else {
            HighlightType::Normal
          };

          if in_string.is_none() && !comment_start.is_empty() && !in_comment {
            let end = i + comment_start.len();
            if render[i..cmp::min(end, render.len())] == *comment_start {
              (i..render.len()).for_each(|_| add!(HighlightType::Comment));
              break;
            }
          }

          if let Some(val) = $ml_comment {
            if in_string.is_none() {
              if in_comment {
                add!(HighlightType::MultilineComment);
                let end = i + val.1.len();
                if render[i..cmp::min(render.len(), end)] == *val.1.as_bytes() {
                  (0..val.1.len().saturating_sub(1)).for_each(|_| add!(HighlightType::MultilineComment));
                  i += val.1.len();
                  previous_separater = true;
                  in_comment = false;
                  continue;
                } else {
                  i += 1;
                  continue;
                }
              } else {
                let end = i + val.0.len();
                if render[i..cmp::min(render.len(), end)] == *val.0.as_bytes() {
                  (i..end).for_each(|_| add!(HighlightType::MultilineComment));
                  i += val.0.len();
                  in_comment = true;
                  continue;
                }
              }
            }
          }

          if let Some(val) = in_string {
            add! {
              if val == '"' { HighlightType::String } else { HighlightType::CharLiteral }
            }
            if c == '\\' && i + 1 < render.len() {
              add! {
                if val == '"' { HighlightType::String } else { HighlightType::CharLiteral }
              }
              i += 2;
              continue;
            }
            if val == c {
              in_string = None;
            }
            i += 1;
            previous_separater = true;
            continue;
          } else if c == '"' || c == '\'' {
            in_string = Some(c);
            add! {
              if c == '"' { HighlightType::String } else { HighlightType::CharLiteral }
            }
          }

          if (c.is_digit(10)
            && (previous_separater 
              || matches!(previous_highlight, HighlightType::Number)))
            || (c == '.' && matches!(previous_highlight, HighlightType::Number)) {
            add!(HighlightType::Number);
            i += 1;
            previous_separater = false;
            continue;
          }
          if previous_separater {
            $(
              $(
                let end = i + $words.len();
                let is_end_or_sep = render
                  .get(end)
                  .map(|c| self.is_separator(*c as char))
                  .unwrap_or(end == render.len());
                if is_end_or_sep && render[i..end] == *$words.as_bytes() {
                  (i..i + $words.len()).for_each(|_| add!(HighlightType::Other($color)));
                  i += $words.len();
                  previous_separater = false;
                  continue;
                }
              )*
            )*
          }
          add!(HighlightType::Normal);
          previous_separater = self.is_separator(c);
          i += 1;
        }
        assert_eq!(current_row.render.len(), current_row.highlight.len())
      }
    }
  };
}
