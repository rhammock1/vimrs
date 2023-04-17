use std::io;
use std::io::Read;
use std::time::Duration;
use crossterm::{event, terminal};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::*;

const POLL_TIMEOUT: Duration = Duration::from_millis(1500);

struct CleanUp;

impl Drop for CleanUp {
  // Implement Drop for this struct so that when it goes out of scope,
  // this function automatically runs
  fn drop(&mut self) {
    terminal::disable_raw_mode().expect("Failed to disable RAW mode.");
  }
}

struct Reader;

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

struct Editor {
  reader: Reader,
}

impl Editor {
  fn new() -> Self {
    Self { reader: Reader }
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

  fn run(&self) -> crossterm::Result<bool> {
    self.process_keypress()
  }
}

fn main() -> crossterm::Result<()> {
  // Prefix with underscore so Rust ignores it as unused
  let _clean_up = CleanUp;

  // Enable terminal's raw mode
  terminal::enable_raw_mode()?;  
  
  // Create a new editor
  let editor = Editor::new();
  while editor.run()? {}

  Ok(())
}
