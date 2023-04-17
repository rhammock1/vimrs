use vimrs::{CleanUp, Editor};

fn main() -> crossterm::Result<()> {
  // Prefix with underscore so Rust ignores it as unused
  let _clean_up = CleanUp;
  
  // Create a new editor
  let mut editor = Editor::new()?;
  while editor.run()? {}

  Ok(())
}
