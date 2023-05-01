use std::{io, time};
use crossterm::{event, terminal, queue};
use crossterm::event::{Event, KeyEvent};

pub mod editor {
  pub mod output;
  pub mod cursor;
  pub mod editor;
  pub mod syntax;
}
mod log;

use editor::output::Output;

pub struct Config {
  pub version: f32,
  pub poll_timeout: time::Duration,
  pub spaces_per_tab: usize,
  pub message_timeout: u64,
  pub max_new_filename_length: usize,
  pub line_number_color: &'static str,
  pub tilde_color: &'static str,
  // command_character: KeyCode,
}

pub const CONFIG: Config = Config {
  version: 0.50,
  poll_timeout: time::Duration::from_millis(1500),
  spaces_per_tab: 2,
  message_timeout: 5,
  max_new_filename_length: 32,
  line_number_color: "red",
  tilde_color: "purple",
  // command_character: KeyCode::Char(':'), // TODO- Actually use this
};

#[macro_export]
macro_rules! prompt {
  ($output:expr, $args:tt) => {
    prompt!($output, $args, callback = |&_, _, _| {})
  };
  ($output:expr, $args:tt, callback = $callback:expr) => {{
    let output: &mut Output = $output;
    let mut input = String::with_capacity(CONFIG.max_new_filename_length);
    loop {
      output.status_message.set_message(format!($args, input));
      output.refresh_screen()?;
      let key_event = Reader.read()?;
      match key_event {
        KeyEvent {
          code: KeyCode::Enter,
          modifiers: event::KeyModifiers::NONE,
          ..
        } => {
          if !input.is_empty() {
            output.status_message.set_message(String::new());
            $callback(output, &input, KeyCode::Enter);
            break;
          }
        },
        KeyEvent {
          code: KeyCode::Esc,
          ..
        } => {
          output.status_message.set_message(String::new());
          input.clear();
          $callback(output, &input, KeyCode::Esc);
          break;
        }
        KeyEvent {
          code: KeyCode::Backspace,
          modifiers: event::KeyModifiers::NONE,
          ..
        } => {
          match input.pop() {
            Some(_) => {},
            None => {},
          }
        },
        KeyEvent {
          code: code @ (KeyCode::Char(..) | KeyCode::Tab),
          modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
          ..
        } => input.push(match code {
          KeyCode::Tab => '\t',
          KeyCode::Char(ch) => ch,
          _ => unreachable!(),
        }),
        _ => {},
      }
      $callback(output, &input, key_event.code);
    }
    if input.is_empty() { None } else { Some(input) }
  }};
}

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
    queue!(io::stdout(), terminal::LeaveAlternateScreen).expect("Failed to leave alternate screen.");
    Output::clear_screen().expect("Failed to clear screen.");
  }
}

/*  

    READER STRUCTURE

*/
pub struct Reader;

impl Reader {
  pub fn read(&self) -> crossterm::Result<KeyEvent> {
    loop {
      if event::poll(CONFIG.poll_timeout)? {
        if let Event::Key(event) = event::read()? {
          return Ok(event);
        }
      }
    }
  }
}
