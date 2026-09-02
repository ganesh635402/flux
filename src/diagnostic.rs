// Flux diagnostic system — structured error rendering.

use crate::interpreter::RuntimeError;
use crate::lexer::{LexerError, Span};
use crate::parser::ParseError;

/// A frame in a Flux call stack.
#[derive(Debug, Clone, PartialEq)]
pub struct CallFrame {
    /// Function name (None for anonymous functions, Some("<main>") for top-level).
    pub name: String,
    /// Source file (if known).
    pub file: Option<String>,
    /// Source location of the call.
    pub span: Span,
}

/// Render a lexer error with source context.
pub fn render_lexer_error(err: &LexerError, source: &str, filename: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}:{}:{}: error: {}\n",
        filename, err.span.line, err.span.column, err.message
    ));
    if let Some(line_text) = get_source_line(source, err.span.line) {
        out.push_str(&format!("\n{}\n", line_text));
        out.push_str(&caret(err.span.column));
    }
    out
}

/// Render a parser error with source context.
pub fn render_parse_error(err: &ParseError, source: &str, filename: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}:{}:{}: error: {}\n",
        filename, err.span.line, err.span.column, err.message
    ));
    if let Some(line_text) = get_source_line(source, err.span.line) {
        out.push_str(&format!("\n{}\n", line_text));
        out.push_str(&caret(err.span.column));
    }
    out
}

/// Render a runtime error with source context and call stack.
pub fn render_runtime_error(err: &RuntimeError, source: &str, filename: &str) -> String {
    let mut out = String::new();

    // Primary error location
    let file = if err.span.line > 0 {
        filename
    } else {
        "<unknown>"
    };
    out.push_str(&format!(
        "{}:{}:{}: error: {}\n",
        file, err.span.line, err.span.column, err.message
    ));

    // Source snippet
    if err.span.line > 0 {
        if let Some(line_text) = get_source_line(source, err.span.line) {
            out.push_str(&format!("\n{}\n", line_text));
            out.push_str(&caret(err.span.column));
        }
    }

    // Call stack
    if !err.call_stack.is_empty() {
        out.push_str("\nStack trace:\n");
        let max_frames = 20;
        let frames = &err.call_stack;
        let show = if frames.len() > max_frames {
            &frames[..max_frames]
        } else {
            frames
        };
        for frame in show {
            let file = frame.file.as_deref().unwrap_or("<unknown>");
            out.push_str(&format!(
                "  at {} ({}:{})\n",
                frame.name, file, frame.span.line
            ));
        }
        if frames.len() > max_frames {
            out.push_str(&format!(
                "  ... {} more frames\n",
                frames.len() - max_frames
            ));
        }
    }

    out
}

/// Get a specific 1-based line from source text.
fn get_source_line(source: &str, line: usize) -> Option<&str> {
    if line == 0 {
        return None;
    }
    source.lines().nth(line - 1)
}

/// Generate a caret indicator string for a given column.
fn caret(column: usize) -> String {
    if column == 0 {
        return "^\n".to_string();
    }
    format!("{}^\n", " ".repeat(column - 1))
}
