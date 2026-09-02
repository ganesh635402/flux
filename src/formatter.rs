// Flux source code formatter.
// Parses source and produces canonically formatted output.

use crate::lexer::{Lexer, TokenKind};

/// Format Flux source code into canonical form.
pub fn format_source(source: &str) -> String {
    let lex_result = Lexer::new(source).tokenize();
    if !lex_result.errors.is_empty() {
        return source.to_string(); // Can't format invalid source
    }

    let mut output = String::new();
    let mut indent = 0usize;
    let mut prev_kind: Option<TokenKind> = None;
    let mut line_start = true;

    for token in &lex_result.tokens {
        if token.kind == TokenKind::Eof {
            break;
        }

        // Handle closing braces — decrease indent before printing
        if token.kind == TokenKind::RightBrace {
            if indent > 0 {
                indent -= 1;
            }
            if !line_start {
                output.push('\n');
            }
            for _ in 0..indent {
                output.push_str("    ");
            }
            output.push('}');
            line_start = false;
            prev_kind = Some(token.kind.clone());
            // Add newline after }
            output.push('\n');
            line_start = true;
            continue;
        }

        // Start new line with indent
        if line_start {
            for _ in 0..indent {
                output.push_str("    ");
            }
            line_start = false;
        } else {
            // Add space between tokens (with some exceptions)
            let needs_space = match (&prev_kind, &token.kind) {
                (Some(TokenKind::LeftParen), _) => false,
                (_, TokenKind::RightParen) => false,
                (Some(TokenKind::LeftBracket), _) => false,
                (_, TokenKind::RightBracket) => false,
                (_, TokenKind::Comma) => false,
                (_, TokenKind::Colon) => false,
                (Some(TokenKind::Comma), _) => true,
                (Some(TokenKind::Colon), _) => true,
                (_, TokenKind::LeftParen) => false,
                (_, TokenKind::LeftBracket) => false,
                _ => true,
            };
            if needs_space {
                output.push(' ');
            }
        }

        // Print the token
        output.push_str(&token.lexeme);

        // Handle opening braces — increase indent and newline
        if token.kind == TokenKind::LeftBrace {
            indent += 1;
            output.push('\n');
            line_start = true;
            prev_kind = Some(token.kind.clone());
            continue;
        }

        prev_kind = Some(token.kind.clone());
    }

    // Ensure trailing newline
    if !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_simple_let() {
        let input = "let x=10";
        let output = format_source(input);
        assert!(output.contains("let x = 10"));
    }

    #[test]
    fn format_function() {
        let input = "fn add(a,b){return a+b}";
        let output = format_source(input);
        assert!(output.contains("fn add(a, b)"));
        assert!(output.contains("return a + b"));
    }

    #[test]
    fn format_idempotent() {
        let input = "let x = 10\nprint(x)\n";
        let first = format_source(input);
        let second = format_source(&first);
        assert_eq!(first, second);
    }

    #[test]
    fn format_preserves_strings() {
        let input = "print(\"hello world\")";
        let output = format_source(input);
        assert!(output.contains("\"hello world\""));
    }

    #[test]
    fn format_indentation() {
        let input = "if true{print(1)}";
        let output = format_source(input);
        assert!(output.contains("    print"));
    }

    #[test]
    fn format_invalid_source_unchanged() {
        // Source with actual lexer errors (unsupported character) should be returned unchanged
        let input = "let x = @invalid";
        let output = format_source(input);
        assert_eq!(output, input);
    }
}
