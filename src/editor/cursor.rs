use std::cmp;
use crossterm::event::KeyCode;
use crate::editor::editor::{EditorRows, Row};
use crate::CONFIG;

#[derive(Copy, Clone)]
pub struct CursorController {
  pub cursor_x: usize,
  pub cursor_y: usize,
  pub screen_columns: usize,
  pub screen_rows: usize,
  pub row_offset: usize,
  pub column_offset: usize,
  pub render_x: usize,
}

impl CursorController {
  pub fn new(window_size: (usize, usize)) -> Self {
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

  pub fn get_render_x(&self, row: &Row) -> usize {
    row.row_content[..self.cursor_x]
      .chars()
      .fold(4, |render_x, c| {
        if c == '\t' {
          render_x + (CONFIG.spaces_per_tab - 1) - (render_x % CONFIG.spaces_per_tab) + 1
        } else {
          render_x + 1
        }
      })
  }

  pub fn scroll(&mut self, editor_rows: &EditorRows) {
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

  pub fn move_cursor(&mut self, direction: KeyCode, editor_rows: &EditorRows) {
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
