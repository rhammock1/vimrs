use std::{io, cmp};
use std::io::Write;
use crossterm::{cursor, event, execute, terminal, queue, style};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
  log,
  prompt,
  syntax_struct,
  CONFIG,
  Reader,
};
use super::{
  cursor::CursorController,
  editor::{
    Row,
    EditorContents,
    EditorRows,
    StatusMessage,
    SyntaxHighlight,
    HighlightType,
  }
};

syntax_struct! {
  struct RustHighlight;
}

pub struct Output {
  pub window_size: (usize, usize), // screen_columns: 0, screen_rows: 1
  pub editor_contents: EditorContents,
  pub editor_rows: EditorRows,
  pub cursor_controller: CursorController,
  pub status_message: StatusMessage,
  pub dirty: bool,
  search_index: SearchIndex,
  syntax_highlight: Option<Box<dyn SyntaxHighlight>>,
}

impl Output {
  pub fn new() -> Self {
    let window_size = terminal::size()
      .map(|(x, y)| (x as usize, y as usize - 2))
      .unwrap();

    let syntax_highlight: Option<Box<dyn SyntaxHighlight>> = Some(Box::new(RustHighlight));
    Self {
      window_size,
      editor_contents: EditorContents::new(),
      editor_rows: EditorRows::new(syntax_highlight.as_deref()),
      cursor_controller: CursorController::new(window_size),
      status_message: StatusMessage::new("HELP: :w = Save | :q = Quit | :f = Find".into()),
      dirty: false,
      search_index: SearchIndex::new(),
      syntax_highlight,
    }
  }

  fn find_callback(output: &mut Output, keyword: &str, key_code: KeyCode) {
    if let Some((index, highlight)) = output.search_index.previous_highlight.take() {
      output.editor_rows.get_editor_row_mut(index).highlight = highlight;
    }
    match key_code {
      KeyCode::Enter | KeyCode::Esc => {
        output.search_index.reset();
      },
      _ => {
        output.search_index.y_direction = None;
        output.search_index.x_direction = None;
        match key_code {
          KeyCode::Down => {
            output.search_index.y_direction = SearchDirection::Forward.into()
          },
          KeyCode::Up => {
            output.search_index.y_direction = SearchDirection::Backward.into()
          },
          KeyCode::Left => {
            output.search_index.x_direction = SearchDirection::Backward.into()
          },
          KeyCode::Right => {
            output.search_index.x_direction = SearchDirection::Forward.into()
          },
          _ => {},
        }
        for i in 0..output.editor_rows.number_of_rows() {
          let row_index = match output.search_index.y_direction.as_ref() {
            None => {
              if output.search_index.x_direction.is_none() {
                output.search_index.y_index = i;
              }
              output.search_index.y_index
            },
            Some(direction) => {
              if matches!(direction, SearchDirection::Forward) {
                output.search_index.y_index + i + 1
              } else {
                let res = output.search_index.y_index.saturating_sub(i);
                if res == 0 {
                  break;
                }
                res - 1
              }
            }
          };
          if row_index > output.editor_rows.number_of_rows() - 1 {
            break;
          }
          let row = output.editor_rows.get_editor_row_mut(row_index);
          let index = match output.search_index.x_direction.as_ref() {
            None => row.render.find(&keyword),
            Some(direction) => {
              let index = if matches!(direction, SearchDirection::Forward) {
                let start = cmp::min(row.render.len(), output.search_index.x_index + 1);
                row.render[start..]
                  .find(&keyword)
                  .map(|x| x + start)
              } else {
                row.render[..output.search_index.x_index]
                  .rfind(&keyword)
              };
              if index.is_none() {
                break;
              }
              index
            }
          };
          if let Some(index) = index {
            output.search_index.previous_highlight = Some((
              row_index,
              row.highlight.clone(),
            ));
            (index..index + keyword.len())
              .for_each(|index| row.highlight[index] = HighlightType::SearchMatch);

            output.cursor_controller.cursor_y = row_index;
            output.search_index.y_index = row_index;
            output.search_index.x_index = index;
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

      if let Some(it) = self.syntax_highlight.as_ref() {
        it.update_syntax(
          self.cursor_controller.cursor_y,
          &mut self.editor_rows.row_contents,
        );
        it.update_syntax(
          self.cursor_controller.cursor_y + 1,
          &mut self.editor_rows.row_contents,
        )
      }
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

    if let Some(it) = self.syntax_highlight.as_ref() {
      it.update_syntax(
        self.cursor_controller.cursor_y,
        &mut self.editor_rows.row_contents,
      )
    }

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
    if let Some(it) = self.syntax_highlight.as_ref() {
      it.update_syntax(
        self.cursor_controller.cursor_y,
        &mut self.editor_rows.row_contents,
      );
      it.update_syntax(
        self.cursor_controller.cursor_y + 1,
        &mut self.editor_rows.row_contents,
      )
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
        let line_number = (file_row + 1) as u32;
        self.editor_contents.push_str(format!("{:>3} ", line_number).as_str());
        let row = self.editor_rows.get_editor_row(file_row);
        let render = &row.render;
        let column_offset = self.cursor_controller.column_offset;
        let len = cmp::min(render.len().saturating_sub(column_offset), screen_columns);
        let start = if len == 0 { 0 } else { column_offset };

        self.syntax_highlight
          .as_ref()
          .map(|syntax_highlight| {
            syntax_highlight.color_row(
              &render[start..start + len],
              &row.highlight[start..start + len],
              &mut self.editor_contents,
            )
          })
          .unwrap_or_else(|| self.editor_contents.push_str(&render[start..start + len]));

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

enum SearchDirection {
  Forward,
  Backward,
}

struct SearchIndex {
  x_index: usize,
  y_index: usize,
  x_direction: Option<SearchDirection>,
  y_direction: Option<SearchDirection>,
  previous_highlight: Option<(usize, Vec<HighlightType>)>,
}

impl SearchIndex {
  fn new() -> Self {
    Self {
      x_index: 0,
      y_index: 0,
      x_direction: None,
      y_direction: None,
      previous_highlight: None,
    }
  }

  fn reset(&mut self) {
    self.x_index = 0;
    self.y_index = 0;
    self.x_direction = None;
    self.y_direction = None;
    self.previous_highlight = None;
  }
}