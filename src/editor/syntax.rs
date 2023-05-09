use crossterm::style;
use std::cmp;

use crate::syntax_struct;
use super::{editor::Row, highlight::{SyntaxHighlight, HighlightType}};

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
    multiline_comment: Some(("/*", "*/")),
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
  struct ShellScriptHighlight {
    extensions: ["sh"],
    file_type: "Shell",
    comment_start: "#",
    keywords: {
      [style::Color::Yellow;
        "if", "then", "else", "elif", "fi", "case", "esac", "for",
        "while", "until", "do", "done", "in", "function", "return",
        "exit", "break", "continue", "declare", "local", "export",
        "readonly", "eval", "shift", "source", "trap", "test", "true",
        "false", "unset", "alias", "command", "type", "echo", "printf",
        "read", "cd", "pwd", "ls", "cat", "grep", "sed", "awk", "cut",
        "find", "sort", "wc", "mkdir", "rm", "mv", "cp", "touch", "chmod",
        "chown", "chgrp", "ln", "tar"
      ]
    },
    multiline_comment: None::<(&'static str, &'static str)>,
    colors: {
      HighlightType::Normal => style::Color::Reset,
      HighlightType::Number => style::Color::Cyan,
      HighlightType::SearchMatch => style::Color::Blue,
      HighlightType::DoubleQuoteString => style::Color::Magenta,
      HighlightType::SingleQuoteString => style::Color::DarkYellow,
      HighlightType::Comment => style::Color::DarkGrey,
      HighlightType::MultilineComment => style::Color::DarkGrey
    }
  }
}