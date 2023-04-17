use std::io;
use std::io::Write;
use std::time::Duration;
use crossterm::{cursor, event, execute, terminal};
use crossterm::event::{Event, KeyCode, KeyEvent};
use colored::*;

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
}

impl Output {
  fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize))
      .unwrap();
    Self { window_size }
  }

  fn clear_screen() -> crossterm::Result<()> {
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))
  }

  fn refresh_screen(&self) -> crossterm::Result<()> {
    Self::clear_screen()?;

    self.draw_rows();

    execute!(io::stdout(), cursor::MoveTo(1, 0))
  }

  fn draw_rows(&self) {
    let screen_rows = self.window_size.1;
    for i in 0..screen_rows {
      // Should line numbers be drawn here?
      print!("{}", "~".purple());
      if i < screen_rows - 1 {
        println!("\r");
      }
      io::stdout().flush().unwrap();
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

  pub fn run(&self) -> crossterm::Result<bool> {
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