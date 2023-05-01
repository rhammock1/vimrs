use std::cmp;
use crossterm::style;
use crate::{log, syntax_struct};
use super::editor::{Row, SyntaxHighlight, HighlightType};

syntax_struct! {
  struct RustHighlight {
    extensions: ["rs"],
    file_type: "Rust",
    comment_start:"//",
    keywords: {
      [style::Color::Yellow;
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
    multiline_comment: Some(("/*", "*/"))
  }
}

syntax_struct! {
  struct PlainTextHighlight {
    extensions: ["txt"],
    file_type: "Plain Text",
    comment_start: "~",
    keywords: {},
    multiline_comment: None::<(&'static str, &'static str)>
  }
}

syntax_struct! {
  struct JavaScriptHighlight {
    extensions: ["js"],
    file_type: "JavaScript",
    comment_start: "//",
    keywords: {
      [style::Color::Yellow;
        "const", "static",
        "let", "if", "else", "for", "function", "self", "Self", "while", "true", "false",
        "in", "continue", "break", "loop", "match"
      ],
      [style::Color::Reset;
        "isize", "i8", "i16", "i32", "i64", "usize",
        "u8", "u16", "u32", "u64", "f32", "f64",
        "char", "str", "bool"
      ]
    },
    multiline_comment:Some(("/*", "*/"))
  }
}