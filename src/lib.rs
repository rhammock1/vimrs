use std::io;
use std::io::Read;
use std::time::Duration;
use crossterm::{event, terminal};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::*;

const POLL_TIMEOUT: Duration = Duration::from_millis(1500);

pub struct CleanUp;

impl Drop for CleanUp {
  // Implement Drop for this struct so that when it goes out of scope,
  // this function automatically runs
  fn drop(&mut self) {
    terminal::disable_raw_mode().expect("Failed to disable RAW mode.");
  }
}

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

pub struct Editor {
  reader: Reader,
}

impl Editor {
  pub fn new() -> crossterm::Result<Self> {
    // Enable terminal's raw mode
    terminal::enable_raw_mode()?;  
    Ok(Self { reader: Reader })
  }

  pub fn run(&self) -> crossterm::Result<bool> {
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