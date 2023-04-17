use crossterm::terminal;
use vimrs::{CleanUp, Editor};

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
