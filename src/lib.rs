use std::{cmp, env, fs, io, path::PathBuf};
use std::io::Write;
use std::time::Duration;
use crossterm::{cursor, event, execute, terminal, queue, style};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::{Colorize, ColoredString};

mod log;

struct Config {
  version: f32,
  poll_timeout: Duration,
  spaces_per_tab: usize,
}

const CONFIG: Config = Config {
  version: 0.20,
  poll_timeout: Duration::from_millis(1500),
  spaces_per_tab: 2,
};

/*  

    CLEAN UP STRUCTURE

*/
pub struct CleanUp;

impl Drop for CleanUp {
  // Implement Drop for this struct so that when it goes out of scope,
  // this function automatically runs
  fn drop(&mut self) {
    log::log::log("INFO".to_string(), "Cleaning up.".to_string());
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
  cursor_controller: CursorController,
}

impl Output {
  fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize - 1))
      .unwrap();
    Self {
      window_size,
      editor_contents: EditorContents::new(),
      editor_rows: EditorRows::new(),
      cursor_controller: CursorController::new(window_size),
    }
  }

  fn clear_screen() -> crossterm::Result<()> {
    log::log::log("INFO".to_string(), format!("Clearing screen.\n\n"));
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))
  }

  fn refresh_screen(&mut self) -> crossterm::Result<()> {
    log::log::log("INFO".to_string(), "Refreshing screen.".to_string());
    self.cursor_controller.scroll(&self.editor_rows);
    queue!(
      self.editor_contents,
      cursor::Hide,
      terminal::Clear(terminal::ClearType::All),
      cursor::MoveTo(0, 0),
    )?;

    self.draw_rows();
    self.draw_status_bar();

    let cursor_x = self.cursor_controller.render_x - self.cursor_controller.column_offset;
    let cursor_y = self.cursor_controller.cursor_y - self.cursor_controller.row_offset;

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

    log::log::log("INFO".to_string(), format!("Drawing rows. Screen columns: {}, screen rows: {}", screen_columns, screen_rows));

    for i in 0..screen_rows {
      let file_row = i + self.cursor_controller.row_offset;
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
        let row = self.editor_rows.get_render(file_row);
        let column_offset = self.cursor_controller.column_offset;
        let len = cmp::min(row.len().saturating_sub(column_offset), screen_columns);
        let start = if len == 0 { 0 } else { column_offset };
        self.editor_contents.push_str(&row[start..start + len]);
      }
      queue!(
        self.editor_contents,
        terminal::Clear(terminal::ClearType::UntilNewLine),
      ).unwrap();
      // if i < screen_rows - 1 {
      self.editor_contents.push_str("\r\n");
      // }
    }
  }

  fn move_cursor(&mut self, direction: KeyCode) {
    self.cursor_controller.move_cursor(direction, &self.editor_rows);
  }

  fn draw_status_bar(&mut self) {
    // Invert color
    self.editor_contents
      .push_str(&style::Attribute::Reverse.to_string());
    
    (0..self.window_size.0).for_each(|_| self.editor_contents.push(' '));
    
    // Reset color
    self.editor_contents
      .push_str(&style::Attribute::Reset.to_string());
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
      } => {
        log::log::log("INFO".to_string(), "Exiting editor.".to_string());
        return Ok(false)
      },
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
        self.output.move_cursor(direction)
      },
      KeyEvent {
        code: val @ (KeyCode::PageUp | KeyCode::PageDown),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        log::log::log("INFO".to_string(), format!("Moving cursor in direction: {:?}", val));
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

    Row Structure

*/
struct Row {
  row_content: Box<str>,
  render: String,
}

impl Row {
  fn new(row_content: Box<str>, render: String) -> Self {
    Self {
      row_content,
      render,
    }
  }
}

/*

    Editor Rows Structure

*/
struct EditorRows {
  row_contents: Vec<Row>,
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
      row_contents: file_contents
        .lines()
        .map(|s| {
          let mut row = Row::new(s.into(), String::new());
          Self::render_row(&mut row);
          row
        })
        .collect(),
    }
  }

  fn number_of_rows(&self) -> usize {
    self.row_contents.len()
  }

  fn get_render(&self, at: usize) -> &String {
    &self.row_contents[at].render
  }

  fn get_row(&self, at: usize) -> &str {
    &self.row_contents[at].row_content
  }

  fn get_editor_row(&self, at: usize) -> &Row {
    &self.row_contents[at]
  }

  fn render_row(row: &mut Row) {
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

/*

    Cursor Position Controller

*/
struct CursorController {
  cursor_x: usize,
  cursor_y: usize,
  screen_columns: usize,
  screen_rows: usize,
  row_offset: usize,
  column_offset: usize,
  render_x: usize,
}

impl CursorController {
  fn new(window_size: (usize, usize)) -> Self {
    Self {
      cursor_x: 0,
      cursor_y: 0,
      screen_columns: window_size.0,
      screen_rows: window_size.1,
      row_offset: 0,
      column_offset: 0,
      render_x: 0,
    }
  }

  fn get_render_x(&self, row: &Row) -> usize {
    row.row_content[..self.cursor_x]
      .chars()
      .fold(0, |render_x, c| {
        if c == '\t' {
          render_x + (CONFIG.spaces_per_tab - 1) - (render_x % CONFIG.spaces_per_tab)
        } else {
          render_x + 1
        }
      })
  }

  fn scroll(&mut self, editor_rows: &EditorRows) {
    self.render_x = 0;
    if self.cursor_y < editor_rows.number_of_rows() {
      self.render_x = self.get_render_x(editor_rows.get_editor_row(self.cursor_y));
    }

    self.row_offset = cmp::min(self.row_offset, self.cursor_y);
    if self.cursor_y >= self.row_offset + self.screen_rows {
      self.row_offset = self.cursor_y - self.screen_rows + 1;
    }

    self.column_offset = cmp::min(self.column_offset, self.render_x);
    if self.render_x >= self.column_offset + self.screen_columns {
      self.column_offset = self.render_x - self.screen_columns + 1;
    }
  }

  fn move_cursor(&mut self, direction: KeyCode, editor_rows: &EditorRows) {
    let number_of_rows = editor_rows.number_of_rows();
    match direction {
      KeyCode::Up => {
        self.cursor_y = self.cursor_y.saturating_sub(1);
      }
      KeyCode::Down => {
        if self.cursor_y < number_of_rows {
          self.cursor_y += 1;
        }
      }
      KeyCode::Left => {
        if self.cursor_x != 0 {
          self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
          self.cursor_y -= 1;
          self.cursor_x = editor_rows.get_row(self.cursor_y).len();
        }
      }
      KeyCode::Right => {
        if self.cursor_y < number_of_rows {
          match self.cursor_x.cmp(&editor_rows.get_row(self.cursor_y).len()) {
            cmp::Ordering::Less => self.cursor_x += 1,
            cmp::Ordering::Equal => {
              self.cursor_y += 1;
              self.cursor_x = 0;
            },
            _ => {},
          }
        }
      }
      KeyCode::End => {
        if self.cursor_y < number_of_rows {
          self.cursor_x = editor_rows.get_row(self.cursor_y).len();
        }
      }
      KeyCode::Home => self.cursor_x = 0,
      _ => unimplemented!("Invalid keypress"),
    }

    let row_length = if self.cursor_y < number_of_rows {
      editor_rows.get_row(self.cursor_y).len()
    } else {
      0
    };
    self.cursor_x = cmp::min(self.cursor_x, row_length);
  }
}