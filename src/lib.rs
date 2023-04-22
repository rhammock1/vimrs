use std::{cmp, env, fs, io, path::PathBuf};
use std::io::Write;
use std::time::{Duration, Instant};
use crossterm::{cursor, event, execute, terminal, queue, style};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::{Colorize, ColoredString};

mod log;

struct Config {
  version: f32,
  poll_timeout: Duration,
  spaces_per_tab: usize,
  message_timeout: u64,
  // command_character: KeyCode,
}

const CONFIG: Config = Config {
  version: 0.30,
  poll_timeout: Duration::from_millis(1500),
  spaces_per_tab: 2,
  message_timeout: 5,
  // command_character: KeyCode::Char(':'), // TODO- Actually use this
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
struct Reader;

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
  status_message: StatusMessage,
  dirty: bool,
}

impl Output {
  fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize - 2))
      .unwrap();
    Self {
      window_size,
      editor_contents: EditorContents::new(),
      editor_rows: EditorRows::new(),
      cursor_controller: CursorController::new(window_size),
      status_message: StatusMessage::new("HELP: :w = Save | :q = Quit".into()),
      dirty: false,
    }
  }
  
  fn insert_character(&mut self, character: char) {
    if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
      self.editor_rows.insert_row();
      self.dirty = true;
    }
    self.editor_rows
      .get_editor_row_mut(self.cursor_controller.cursor_y)
      .insert_character(self.cursor_controller.cursor_x, character);

    self.cursor_controller.cursor_x += 1;
    self.dirty = true;
  }

  fn delete_character(&mut self) {
    if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
      return;
    }
    if self.cursor_controller.cursor_y == 0 && self.cursor_controller.cursor_x == 0 {
      return;
    }
    let row = self.editor_rows
      .get_editor_row_mut(self.cursor_controller.cursor_y);

    if self.cursor_controller.cursor_x > 0 {
      row.delete_character(self.cursor_controller.cursor_x - 1);
      self.cursor_controller.cursor_x -= 1;
    } else {
      let previous_row_content = self
        .editor_rows
        .get_row(self.cursor_controller.cursor_y - 1);

      self.cursor_controller.cursor_x = previous_row_content.len();
      self.editor_rows
        .join_adjacent_rows(self.cursor_controller.cursor_y);
      self.cursor_controller.cursor_y -= 1;
    }
    self.dirty = true;
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

    // TODO- Only draw status bar if there is a message or there has been a timeout
    self.draw_status_bar();

    self.draw_message_bar();

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

      self.editor_contents.push_str("\r\n");
    }
  }

  fn move_cursor(&mut self, direction: KeyCode) {
    self.cursor_controller.move_cursor(direction, &self.editor_rows);
  }

  fn draw_status_bar(&mut self) {
    // Invert color
    self.editor_contents
      .push_str(&style::Attribute::Reverse.to_string());

    let info = format!(
      // Name, number of lines, size in bytes
      "\"{}\" {} Lines, {:?}B written    {}",
      self.editor_rows
        .filename
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|filename| filename.to_str())
        .unwrap_or("[Untitled]"),
      self.editor_rows.number_of_rows(),
      match self.editor_rows.file_size {
        Some(size) => size,
        _ => 0,
      },
      if self.dirty { "(modified)" } else { "" },
    );

    let info_length = cmp::min(info.len(), self.window_size.0);

    let line_info = format!(
      "Ln {}, Col {}",
      self.cursor_controller.cursor_y + 1,
      self.cursor_controller.cursor_x + 1,
    );

    self.editor_contents.push_str(&info[..info_length]);

    for i in info_length..self.window_size.0 {
      if self.window_size.0 - i == line_info.len() {
        self.editor_contents.push_str(&line_info);
        break;
      } else {
        self.editor_contents.push(' ');
      }
    }
        
    // Reset color
    self.editor_contents
      .push_str(&style::Attribute::Reset.to_string());

    self.editor_contents.push_str("\r\n");
  }

  fn draw_message_bar(&mut self) {
    queue!(
      self.editor_contents,
      terminal::Clear(terminal::ClearType::UntilNewLine),
    ).unwrap();

    if let Some(msg) = self.status_message.message() {
      self.editor_contents
        .push_str(&msg[..cmp::min(self.window_size.0, msg.len())]);
    }
  }
}

/*  

    EDITOR STRUCTURE

*/
pub struct Editor {
  reader: Reader,
  output: Output,
  previous_3_keys: Vec<KeyCode>,
}

impl Editor {
  pub fn new() -> crossterm::Result<Self> {
    // Enable terminal's raw mode
    terminal::enable_raw_mode()?;  
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

  fn set_previous_key(&mut self, key: KeyCode) {
    self.previous_3_keys.push(key);
    if self.previous_3_keys.len() > 3 {
      self.previous_3_keys.remove(0);
    }
  }

  fn process_keypress(&mut self) -> crossterm::Result<bool> {
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
        code: KeyCode::Char('w'),
        modifiers: event::KeyModifiers::NONE,
        ..
      } => {
        log::log::log("INFO".to_string(), "Saving file.".to_string());
        // TODO- Check that a filename has been provided, if not, prompt for one
        if self.previous_3_keys.last() == Some(&KeyCode::Char(':')) {
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
#[derive(Default)]
struct Row {
  row_content: String,
  render: String,
}

impl Row {
  fn new(row_content: String, render: String) -> Self {
    Self {
      row_content,
      render,
    }
  }

  fn insert_character(&mut self, at: usize, character: char) {
    self.row_content.insert(at, character);
    EditorRows::render_row(self)
  }

  fn delete_character(&mut self, at: usize) {
    self.row_content.remove(at);
    EditorRows::render_row(self)
  }
}

/*

    Editor Rows Structure

*/
struct EditorRows {
  row_contents: Vec<Row>,
  filename: Option<PathBuf>,
  file_size: Option<u64>,
}

impl EditorRows {
  fn new() -> Self {
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

  fn join_adjacent_rows(&mut self, at: usize) {
    let current_row = self.row_contents.remove(at);
    let previous_row = self.get_editor_row_mut(at - 1);

    previous_row.row_content.push_str(&current_row.row_content);
    Self::render_row(previous_row);
  }

  fn save(&mut self) -> io::Result<()> {
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

  fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
    &mut self.row_contents[at]
  }

  fn insert_row(&mut self) {
    self.row_contents.push(Row::default());
  }

  fn from_file(file: PathBuf) -> Self {
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
          render_x + (CONFIG.spaces_per_tab - 1) - (render_x % CONFIG.spaces_per_tab) + 1
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

/*

    Status Message Structure

*/

struct StatusMessage {
  message: Option<String>,
  set_time: Option<Instant>,
}

impl StatusMessage {
  fn new(initial_message: String) -> Self {
    Self {
      message: Some(initial_message),
      set_time: Some(Instant::now()),
    }
  }

  fn set_message(&mut self, message: String) {
    self.message = Some(message);
    self.set_time = Some(Instant::now());
  }

  fn message(&mut self) -> Option<&String> {
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
