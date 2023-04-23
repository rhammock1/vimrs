use std::{io, cmp};
use std::io::Write;
use crossterm::{cursor, event, execute, terminal, queue, style};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
  log,
  prompt,
  CONFIG,
  Reader,
};
use super::{
  cursor::CursorController,
  editor::{
    EditorContents,
    EditorRows,
    StatusMessage,
  }};

pub struct Output {
  pub window_size: (usize, usize), // screen_columns: 0, screen_rows: 1
  pub editor_contents: EditorContents,
  pub editor_rows: EditorRows,
  pub cursor_controller: CursorController,
  pub status_message: StatusMessage,
  pub dirty: bool,
}

impl Output {
  pub fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize - 2))
      .unwrap();
    Self {
      window_size,
      editor_contents: EditorContents::new(),
      editor_rows: EditorRows::new(),
      cursor_controller: CursorController::new(window_size),
      status_message: StatusMessage::new("HELP: :w = Save | :q = Quit | :f = Find".into()),
      dirty: false,
    }
  }

  fn find_callback(output: &mut Output, keyword: &str, key_code: KeyCode) {
    match key_code {
      KeyCode::Enter | KeyCode::Esc => {},
      _ => {
        for i in 0..output.editor_rows.number_of_rows() {
          let row = output.editor_rows.get_editor_row(i);
          if let Some(index) = row.render.find(&keyword) {
            output.cursor_controller.cursor_y = i;
            output.cursor_controller.cursor_x = row.get_row_content_x(index);
            output.cursor_controller.row_offset = output.editor_rows.number_of_rows();
            break;
          }
        }
      }
    }
  }

  pub fn find(&mut self) -> io::Result<()> {
    let cursor_controller = self.cursor_controller;
    if prompt!(
      self,
      "Search: {} (ESC to cancel)",
      callback = Output::find_callback
    ).is_none() {
      self.cursor_controller = cursor_controller;
    }
    Ok(())
  }

  pub fn insert_newline(&mut self) {
    if self.cursor_controller.cursor_x == 0 {
      self.editor_rows
        .insert_row(self.cursor_controller.cursor_y, String::new())
    } else {
      let current_row = self
        .editor_rows
        .get_editor_row_mut(self.cursor_controller.cursor_y);

      let new_row_content = current_row
        .row_content[self.cursor_controller.cursor_x..]
        .into();

      current_row
        .row_content
        .truncate(self.cursor_controller.cursor_x);

      EditorRows::render_row(current_row);
      self.editor_rows
        .insert_row(self.cursor_controller.cursor_y + 1, new_row_content);
    }
    self.cursor_controller.cursor_x = 0;
    self.cursor_controller.cursor_y += 1;
    self.dirty = true;
  }
  
  pub fn insert_character(&mut self, character: char) {
    if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
      self.editor_rows
        .insert_row(self.editor_rows.number_of_rows(), String::new());
      self.dirty = true;
    }
    self.editor_rows
      .get_editor_row_mut(self.cursor_controller.cursor_y)
      .insert_character(self.cursor_controller.cursor_x, character);

    self.cursor_controller.cursor_x += 1;
    self.dirty = true;
  }

  pub fn delete_character(&mut self) {
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

  pub fn clear_screen() -> crossterm::Result<()> {
    log::log::log("INFO".to_string(), format!("Clearing screen.\n\n"));
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))
  }

  pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
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

  pub fn draw_rows(&mut self) {
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

  pub fn move_cursor(&mut self, direction: KeyCode) {
    self.cursor_controller.move_cursor(direction, &self.editor_rows);
  }

  pub fn draw_status_bar(&mut self) {
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

  pub fn draw_message_bar(&mut self) {
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