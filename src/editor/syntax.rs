use std::cmp;
use crossterm::{queue, style};
// use colored::{Colorize, Color};

use crate::syntax_struct;
use super::editor::{Row, EditorContents};

#[derive(Copy, Clone, Debug)]
pub enum HighlightType {
  Normal,
  Number,
  SearchMatch,
  DoubleQuoteString,
  SingleQuoteString,
  Comment,
  MultilineComment,
  Other (style::Color),
}

pub enum FormatType {
  Normal,
  Strikethrough,
  Underline,
  Italic,
  Bold,
}

pub trait SyntaxHighlight {
  fn extensions(&self) -> &[&str];
  fn file_type(&self) -> &str;
  fn comment_start(&self) -> &str;
  fn multiline_comment(&self) -> Option<(&str, &str)>;
  fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>);
  fn syntax_color(&self, highlight_type: &HighlightType) -> style::Color;
  fn color_row(&self, render: &str, highlight: &[HighlightType], out: &mut EditorContents) {
    let mut current_color = self.syntax_color(&HighlightType::Normal);
    render.char_indices().for_each(|(i, c)| {
      let color = self.syntax_color(&highlight[i]);
      if current_color != color {
        current_color = color;
        let _ = queue!(out, style::SetForegroundColor(color));
      }
      out.push(c);
    });
    let _ = queue!(out, style::SetForegroundColor(style::Color::Reset));
  }
  fn is_separator(&self, c: char) -> bool {
    c.is_whitespace() || [
      ',', '.', '(', ')', '+', '-', '/', '*', '=', '~', '%', '<', '>', '"', '\'', ';', '&',
    ].contains(&c)
  }
}

#[macro_export]
macro_rules! syntax_struct {
  (
    struct $Name:ident {
      extensions: $ext:expr,
      file_type: $type:expr,
      comment_start: $start:expr,
      keywords: {
        $([$color:expr; $($words:expr),*]),*
      },
      multiline_comment:$ml_comment:expr,
      colors: {
        $($highlight:pat => $style_color:expr),*
      }
    }
  ) => {
    pub struct $Name {
      extensions: &'static [&'static str],
      file_type: &'static str,
      comment_start: &'static str,
      multiline_comment: Option<(&'static str, &'static str)>,
    }

    impl $Name {
      pub fn new() -> Self {
        Self {
          extensions: &$ext,
          file_type: $type,
          comment_start: $start,
          multiline_comment: $ml_comment,
        }
      }
    }

    impl SyntaxHighlight for $Name {
      fn extensions(&self) -> &[&str] {
        self.extensions
      }

      fn file_type(&self) -> &str {
        self.file_type
      }

      fn comment_start(&self) -> &str {
        self.comment_start
      }

      fn multiline_comment(&self) -> Option<(&str, &str)> {
        self.multiline_comment
      }

      fn syntax_color(&self, highlight_type: &HighlightType) -> style::Color {
        // Compare highlight_type with the color of the highlight stored in the struct
        // If they match, return the color
        // Otherwise, return the default color
        match highlight_type {
          $(
            $highlight => $style_color,
          )*
          HighlightType::Other(color) => *color,
        }
      }

      fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>) {
        let mut in_comment = at > 0 && editor_rows[at - 1].is_comment;
        let current_row = &mut editor_rows[at];

        macro_rules! add {
          ($h:expr) => {
            current_row.highlight.push($h)
          };
        }

        current_row.highlight = Vec::with_capacity(current_row.render.len());

        let render = current_row.render.as_bytes();
        let mut i = 0;
        let mut previous_separater = true;
        let mut in_string: Option<char> = None;
        let comment_start = self.comment_start().as_bytes();

        while i < render.len() {
          let c = render[i] as char;
          let previous_highlight = if i > 0 {
            current_row.highlight[i - 1]
          } else {
            HighlightType::Normal
          };

          if in_string.is_none() && !comment_start.is_empty() && !in_comment {
            let end = i + comment_start.len();
            if render[i..cmp::min(end, render.len())] == *comment_start {
              (i..render.len()).for_each(|_| add!(HighlightType::Comment));
              break;
            }
          }

          if let Some(val) = $ml_comment {
            if in_string.is_none() {
              if in_comment {
                add!(HighlightType::MultilineComment);
                let end = i + val.1.len();
                if render[i..cmp::min(render.len(), end)] == *val.1.as_bytes() {
                  (0..val.1.len().saturating_sub(1)).for_each(|_| add!(HighlightType::MultilineComment));
                  i = end;
                  previous_separater = true;
                  in_comment = false;
                  continue;
                } else {
                  i += 1;
                  continue;
                }
              } else {
                let end = i + val.0.len();
                if render[i..cmp::min(render.len(), end)] == *val.0.as_bytes() {
                  (i..end).for_each(|_| add!(HighlightType::MultilineComment));
                  i += val.0.len();
                  in_comment = true;
                  continue;
                }
              }
            }
          }

          if let Some(val) = in_string {
            add! {
              if val == '"' { HighlightType::DoubleQuoteString } else { HighlightType::SingleQuoteString }
            }
            if c == '\\' && i + 1 < render.len() {
              add! {
                if val == '"' { HighlightType::DoubleQuoteString } else { HighlightType::SingleQuoteString }
              }
              i += 2;
              continue;
            }
            if val == c {
              in_string = None;
            }
            i += 1;
            previous_separater = true;
            continue;
            // We are in a string if the current character is a quote, there is another quote somewhere in the line, and the previous character is a separator
          } else if (c == '"' || c == '\'') && render[i + 1..].contains(&(c as u8)) && previous_separater {
            in_string = Some(c);
            add! {
              if c == '"' { HighlightType::DoubleQuoteString } else { HighlightType::SingleQuoteString }
            }
            i += 1;
            continue;
          }

          if (c.is_digit(10)
            && (previous_separater 
              || matches!(previous_highlight, HighlightType::Number)))
            || (c == '.' && matches!(previous_highlight, HighlightType::Number)) {
            add!(HighlightType::Number);
            i += 1;
            previous_separater = false;
            continue;
          }
          if previous_separater {
            $(
              $(
                let end = i + $words.len();
                let is_end_or_sep = render
                  .get(end)
                  .map(|c| self.is_separator(*c as char))
                  .unwrap_or(end == render.len());
                if is_end_or_sep && render[i..end] == *$words.as_bytes() {
                  (i..i + $words.len()).for_each(|_| add!(HighlightType::Other($color)));
                  i += $words.len();
                  previous_separater = false;
                  continue;
                }
              )*
            )*
          }
          add!(HighlightType::Normal);
          previous_separater = self.is_separator(c);
          i += 1;
        }
        assert_eq!(current_row.render.len(), current_row.highlight.len());
        let changed = current_row.is_comment != in_comment;
        current_row.is_comment = in_comment;
        if (changed && at + 1 < editor_rows.len()) {
          self.update_syntax(at + 1, editor_rows)
        }
      }
    }
  };
}


syntax_struct! {
  struct RustHighlight {
    extensions: ["rs"],
    file_type: "Rust",
    comment_start:"//",
    keywords: {
      [style::Color::Red;
        "mod", "unsafe", "extern", "crate", "use", "type", "struct", "enum", "union", "const", "static",
        "mut", "let", "if", "else", "impl", "trait", "for", "fn", "self", "Self", "while", "true", "false",
        "in", "continue", "break", "loop", "match"
      ],
      [style::Color::Reset;
        "isize", "i8", "i16", "i32", "i64", "usize",
        "u8", "u16", "u32", "u64", "f32", "f64",
        "char", "str", "bool"
      ]
    },
    multiline_comment: Some(("/*", "*/")),
    colors: {
      HighlightType::Normal => style::Color::Reset,
      HighlightType::Number => style::Color::Cyan,
      HighlightType::SearchMatch => style::Color::Blue,
      HighlightType::DoubleQuoteString => style::Color::Green,
      HighlightType::SingleQuoteString => style::Color::Yellow,
      HighlightType::Comment => style::Color::DarkGrey,
      HighlightType::MultilineComment => style::Color::DarkGrey
    }
  }
}

syntax_struct! {
  struct PlainTextHighlight {
    extensions: ["txt"],
    file_type: "Plain Text",
    comment_start: "~",
    keywords: {},
    multiline_comment: None::<(&'static str, &'static str)>,
    colors: {
      HighlightType::Normal => style::Color::Reset,
      HighlightType::Number => style::Color::Cyan,
      HighlightType::SearchMatch => style::Color::Blue,
      HighlightType::DoubleQuoteString => style::Color::Red,
      HighlightType::SingleQuoteString => style::Color::Yellow,
      HighlightType::Comment => style::Color::DarkGrey,
      HighlightType::MultilineComment => style::Color::DarkGrey
    }
  }
}

syntax_struct! {
  struct JavaScriptHighlight {
    extensions: ["js"],
    file_type: "JavaScript",
    comment_start: "//",
    keywords: {
      [style::Color::Yellow;
        "abstract", "arguments", "await", "boolean", "break", "byte", "case", "catch",
        "char", "class", "const", "continue", "debugger", "default", "delete", "do",
        "double", "else", "enum", "eval", "export", "extends", "false", "final", "finally",
        "float", "for", "function", "goto", "if", "implements", "import", "in", "instanceof", 
        "int", "interface", "let", "long", "native", "new", "null", "package", "private", 
        "protected", "public", "return", "short", "static", "super", "switch", "synchronized", 
        "this", "throw", "throws", "transient", "true", "try", "typeof", "var", "void", 
        "volatile", "while", "with", "yield"
      ],
      [style::Color::Reset;
        "Undefined", "Null", "Boolean", "Number", "String", "Symbol", "Object"
      ]
    },
    multiline_comment:Some(("/*", "*/")),
    colors: {
      HighlightType::Normal => style::Color::Reset,
      HighlightType::Number => style::Color::Cyan,
      HighlightType::SearchMatch => style::Color::Blue,
      HighlightType::DoubleQuoteString => style::Color::Red,
      HighlightType::SingleQuoteString => style::Color::Yellow,
      HighlightType::Comment => style::Color::DarkGrey,
      HighlightType::MultilineComment => style::Color::DarkGrey
    }
  }
}