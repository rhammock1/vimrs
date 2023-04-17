use std::io;
use std::io::Write;
use std::time::Duration;
use crossterm::{cursor, event, execute, terminal, queue};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::Colorize;

const POLL_TIMEOUT: Duration = Duration::from_millis(1500);

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
      if event::poll(POLL_TIMEOUT)? {
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
  window_size: (usize, usize),
  editor_contents: EditorContents,
}

impl Output {
  fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize))
      .unwrap();
    Self {
      window_size,
      editor_contents: EditorContents::new(),
    }
  }

  fn clear_screen() -> crossterm::Result<()> {
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))
  }

  fn refresh_screen(&mut self) -> crossterm::Result<()> {
    queue!(
      self.editor_contents,
      cursor::Hide,
      cursor::MoveTo(0, 0),
    )?;

    self.draw_rows();

    queue!(
      self.editor_contents,
      cursor::MoveTo(1, 0),
      cursor::Show,
    )?;
    self.editor_contents.flush()
  }

  fn draw_rows(&mut self) {
    let screen_rows = self.window_size.1;
    for i in 0..screen_rows {
      self.editor_contents.push('~');
      queue!(
        self.editor_contents,
        terminal::Clear(terminal::ClearType::UntilNewLine),
      ).unwrap();
      if i < screen_rows - 1 {
        self.editor_contents.push_str("\r\n");
      }
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

  fn process_keypress(&self) -> crossterm::Result<bool> {
    match self.reader.read()? {
      KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: event::KeyModifiers::CONTROL,
        ..
      } => return Ok(false),
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
    let content;
    if self.content.contains("~") {
      content = self.content.purple();
    } else {
      content = self.content.normal();
    }
    let out = write!(io::stdout(), "{}", content);
    io::stdout().flush()?;
    self.content.clear();
    out
  }
}