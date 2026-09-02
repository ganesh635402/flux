// Flux lexer - tokenizes source code into a stream of tokens.

/// The kinds of tokens that the Flux lexer can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// An identifier: `[a-zA-Z_][a-zA-Z0-9_]*`
    Identifier,
    /// A string literal: `"..."` (no escape sequences yet)
    StringLiteral,
    /// An integer literal: `[0-9]+`
    IntegerLiteral,
    /// A floating-point literal: `[0-9]+ "." [0-9]+`
    FloatLiteral,
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `**`
    StarStar,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `=`
    Equals,
    /// `==`
    EqualEqual,
    /// `!=`
    BangEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `>>`
    GreaterGreater,
    /// `>>=`
    GreaterGreaterEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `<<`
    LessLess,
    /// `<<=`
    LessLessEqual,
    /// `&&`
    AmpAmp,
    /// `&`
    Amp,
    /// `&=`
    AmpEqual,
    /// `||`
    PipePipe,
    /// `|`
    Pipe,
    /// `|=`
    PipeEqual,
    /// `^^`
    CaretCaret,
    /// `^`
    Caret,
    /// `^=`
    CaretEqual,
    /// `~`
    Tilde,
    /// `!`
    Bang,
    /// `+=`
    PlusEqual,
    /// `-=`
    MinusEqual,
    /// `*=`
    StarEqual,
    /// `/=`
    SlashEqual,
    /// `%=`
    PercentEqual,
    /// `**=`
    StarStarEqual,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `->`
    Arrow,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `..<`
    DotDotLess,
    /// `ns` — nanoseconds duration literal
    DurationLiteral,
    /// End of input
    Eof,
}

/// A source location for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
}

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The raw source text of the token (empty for EOF).
    pub lexeme: String,
    /// Where in the source this token starts.
    pub span: Span,
}

/// An error encountered during lexing.
#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    /// A human-readable description of the error.
    pub message: String,
    /// Where in the source the error occurred.
    pub span: Span,
}

/// The result of tokenizing a source string.
pub struct LexResult {
    /// All tokens successfully produced (always ends with EOF).
    pub tokens: Vec<Token>,
    /// Any errors encountered during scanning.
    pub errors: Vec<LexerError>,
}

/// The Flux lexer. Scans source code and produces tokens.
pub struct Lexer {
    /// The source code as a vector of characters.
    source: Vec<char>,
    /// Current position in the source (index into `source`).
    current: usize,
    /// Current 1-based line number.
    line: usize,
    /// Current 1-based column number.
    column: usize,
}

impl Lexer {
    /// Create a new lexer for the given source code.
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
        }
    }

    /// Tokenize the entire source, returning all tokens and any errors.
    pub fn tokenize(mut self) -> LexResult {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        loop {
            self.skip_whitespace();

            if self.is_at_end() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    lexeme: String::new(),
                    span: self.current_span(),
                });
                break;
            }

            let ch = self.peek();

            if ch == '(' {
                tokens.push(Token {
                    kind: TokenKind::LeftParen,
                    lexeme: "(".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == ')' {
                tokens.push(Token {
                    kind: TokenKind::RightParen,
                    lexeme: ")".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == '{' {
                tokens.push(Token {
                    kind: TokenKind::LeftBrace,
                    lexeme: "{".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == '}' {
                tokens.push(Token {
                    kind: TokenKind::RightBrace,
                    lexeme: "}".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == '[' {
                tokens.push(Token {
                    kind: TokenKind::LeftBracket,
                    lexeme: "[".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == ']' {
                tokens.push(Token {
                    kind: TokenKind::RightBracket,
                    lexeme: "]".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == ',' {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    lexeme: ",".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == ':' {
                tokens.push(Token {
                    kind: TokenKind::Colon,
                    lexeme: ":".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == '.' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '.' {
                    self.advance();
                    if !self.is_at_end() && self.peek() == '<' {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::DotDotLess,
                            lexeme: "..<".to_string(),
                            span,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::DotDot,
                            lexeme: "..".to_string(),
                            span,
                        });
                    }
                } else if !self.is_at_end() && self.peek().is_ascii_digit() {
                    // Leading-dot float: .9, .25, .001
                    let mut lexeme = String::from("0.");
                    while !self.is_at_end() && self.peek().is_ascii_digit() {
                        lexeme.push(self.peek());
                        self.advance();
                    }
                    tokens.push(Token {
                        kind: TokenKind::FloatLiteral,
                        lexeme,
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Dot,
                        lexeme: ".".to_string(),
                        span,
                    });
                }
            } else if ch == '+' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::PlusEqual,
                        lexeme: "+=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Plus,
                        lexeme: "+".to_string(),
                        span,
                    });
                }
            } else if ch == '-' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '>' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Arrow,
                        lexeme: "->".to_string(),
                        span,
                    });
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::MinusEqual,
                        lexeme: "-=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Minus,
                        lexeme: "-".to_string(),
                        span,
                    });
                }
            } else if ch == '*' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '*' {
                    self.advance();
                    if !self.is_at_end() && self.peek() == '=' {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::StarStarEqual,
                            lexeme: "**=".to_string(),
                            span,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::StarStar,
                            lexeme: "**".to_string(),
                            span,
                        });
                    }
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::StarEqual,
                        lexeme: "*=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Star,
                        lexeme: "*".to_string(),
                        span,
                    });
                }
            } else if ch == '/' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '/' {
                    // Single-line comment: skip to end of line
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                } else if !self.is_at_end() && self.peek() == '*' {
                    // Multi-line comment: skip until */
                    self.advance(); // consume '*'
                    let mut terminated = false;
                    while !self.is_at_end() {
                        if self.peek() == '*' {
                            self.advance();
                            if !self.is_at_end() && self.peek() == '/' {
                                self.advance();
                                terminated = true;
                                break;
                            }
                        } else if self.peek() == '\n' {
                            self.advance_newline();
                        } else {
                            self.advance();
                        }
                    }
                    if !terminated {
                        errors.push(LexerError {
                            message: "unterminated multi-line comment".to_string(),
                            span,
                        });
                    }
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::SlashEqual,
                        lexeme: "/=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Slash,
                        lexeme: "/".to_string(),
                        span,
                    });
                }
            } else if ch == '%' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::PercentEqual,
                        lexeme: "%=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Percent,
                        lexeme: "%".to_string(),
                        span,
                    });
                }
            } else if ch == '~' {
                tokens.push(Token {
                    kind: TokenKind::Tilde,
                    lexeme: "~".to_string(),
                    span: self.current_span(),
                });
                self.advance();
            } else if ch == '=' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::EqualEqual,
                        lexeme: "==".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Equals,
                        lexeme: "=".to_string(),
                        span,
                    });
                }
            } else if ch == '!' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::BangEqual,
                        lexeme: "!=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Bang,
                        lexeme: "!".to_string(),
                        span,
                    });
                }
            } else if ch == '>' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '>' {
                    self.advance();
                    if !self.is_at_end() && self.peek() == '=' {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::GreaterGreaterEqual,
                            lexeme: ">>=".to_string(),
                            span,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::GreaterGreater,
                            lexeme: ">>".to_string(),
                            span,
                        });
                    }
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::GreaterEqual,
                        lexeme: ">=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Greater,
                        lexeme: ">".to_string(),
                        span,
                    });
                }
            } else if ch == '<' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '<' {
                    self.advance();
                    if !self.is_at_end() && self.peek() == '=' {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::LessLessEqual,
                            lexeme: "<<=".to_string(),
                            span,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::LessLess,
                            lexeme: "<<".to_string(),
                            span,
                        });
                    }
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::LessEqual,
                        lexeme: "<=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Less,
                        lexeme: "<".to_string(),
                        span,
                    });
                }
            } else if ch == '&' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '&' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::AmpAmp,
                        lexeme: "&&".to_string(),
                        span,
                    });
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::AmpEqual,
                        lexeme: "&=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Amp,
                        lexeme: "&".to_string(),
                        span,
                    });
                }
            } else if ch == '|' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '|' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::PipePipe,
                        lexeme: "||".to_string(),
                        span,
                    });
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::PipeEqual,
                        lexeme: "|=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Pipe,
                        lexeme: "|".to_string(),
                        span,
                    });
                }
            } else if ch == '^' {
                let span = self.current_span();
                self.advance();
                if !self.is_at_end() && self.peek() == '^' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::CaretCaret,
                        lexeme: "^^".to_string(),
                        span,
                    });
                } else if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::CaretEqual,
                        lexeme: "^=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Caret,
                        lexeme: "^".to_string(),
                        span,
                    });
                }
            } else if ch == '"' {
                match self.scan_string('"') {
                    Ok(token) => tokens.push(token),
                    Err(err) => errors.push(err),
                }
            } else if ch == '\'' {
                match self.scan_string('\'') {
                    Ok(token) => tokens.push(token),
                    Err(err) => errors.push(err),
                }
            } else if ch.is_ascii_digit() {
                tokens.push(self.scan_number());
            } else if Self::is_identifier_start(ch) {
                tokens.push(self.scan_identifier());
            } else {
                errors.push(LexerError {
                    message: format!("unexpected character '{}'", ch),
                    span: self.current_span(),
                });
                self.advance();
            }
        }

        LexResult { tokens, errors }
    }

    /// Skip over whitespace characters, updating line/column tracking.
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let ch = self.peek();
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else if ch == '\n' {
                self.advance_newline();
            } else {
                break;
            }
        }
    }

    /// Scan a string literal starting at the opening quote character.
    /// Escape sequences (\n, \t, \r, \\, \", \0) are stored raw in the
    /// lexeme and decoded later by the parser, so the formatter can
    /// round-trip source faithfully.
    fn scan_string(&mut self, quote: char) -> Result<Token, LexerError> {
        let span = self.current_span();
        // Skip the opening quote
        self.advance();

        let mut value = String::new();
        value.push('"');

        loop {
            if self.is_at_end() {
                return Err(LexerError {
                    message: "unterminated string literal".to_string(),
                    span,
                });
            }

            let ch = self.peek();

            if ch == '\n' {
                return Err(LexerError {
                    message: "unterminated string literal (newline in string)".to_string(),
                    span,
                });
            }

            if ch == quote {
                value.push('"');
                self.advance();
                break;
            }

            if ch == '\\' {
                // Store the backslash and next char raw; parser will decode
                value.push('\\');
                self.advance();
                if self.is_at_end() {
                    return Err(LexerError {
                        message: "unterminated string literal (backslash at end)".to_string(),
                        span,
                    });
                }
                let escaped = self.peek();
                match escaped {
                    'n' | 't' | 'r' | '\\' | '"' | '0' => {
                        value.push(escaped);
                        self.advance();
                    }
                    _ => {
                        return Err(LexerError {
                            message: format!("invalid escape sequence '\\{}'", escaped),
                            span,
                        });
                    }
                }
                continue;
            }

            value.push(ch);
            self.advance();
        }

        Ok(Token {
            kind: TokenKind::StringLiteral,
            lexeme: value,
            span,
        })
    }

    /// Scan an identifier starting at the current position.
    fn scan_identifier(&mut self) -> Token {
        let span = self.current_span();
        let mut lexeme = String::new();

        while !self.is_at_end() && Self::is_identifier_continue(self.peek()) {
            lexeme.push(self.peek());
            self.advance();
        }

        Token {
            kind: TokenKind::Identifier,
            lexeme,
            span,
        }
    }

    /// Scan a number literal (integer, float, or duration).
    fn scan_number(&mut self) -> Token {
        let span = self.current_span();
        let mut lexeme = String::new();

        // Consume integer part
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            lexeme.push(self.peek());
            self.advance();
        }

        // Check for fractional part
        if !self.is_at_end() && self.peek() == '.' && self.peek_next_is_digit() {
            lexeme.push('.');
            self.advance();
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                lexeme.push(self.peek());
                self.advance();
            }
            Token {
                kind: TokenKind::FloatLiteral,
                lexeme,
                span,
            }
        } else if !self.is_at_end() && self.peek().is_ascii_alphabetic() {
            // Check for duration suffix immediately after integer digits
            let suffix_start = self.current;
            let suffix_col = self.column;
            let mut suffix = String::new();
            while !self.is_at_end() && self.peek().is_ascii_alphabetic() {
                suffix.push(self.peek());
                self.advance();
            }
            match suffix.as_str() {
                "ns" | "us" | "ms" | "s" | "m" | "h" | "d" => {
                    lexeme.push_str(&suffix);
                    Token {
                        kind: TokenKind::DurationLiteral,
                        lexeme,
                        span,
                    }
                }
                _ => {
                    // Not a valid duration suffix — rewind and emit integer,
                    // let the identifier be scanned separately
                    self.current = suffix_start;
                    self.column = suffix_col;
                    Token {
                        kind: TokenKind::IntegerLiteral,
                        lexeme,
                        span,
                    }
                }
            }
        } else {
            Token {
                kind: TokenKind::IntegerLiteral,
                lexeme,
                span,
            }
        }
    }

    /// Check if a character can start an identifier.
    fn is_identifier_start(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_'
    }

    /// Check if a character can continue an identifier.
    fn is_identifier_continue(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }

    /// Returns true if we've consumed all source characters.
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    /// Peek at the current character without consuming it.
    fn peek(&self) -> char {
        self.source[self.current]
    }

    /// Check if the character after the current one is a digit.
    /// Used for distinguishing `3.14` from `3.something_else`.
    fn peek_next_is_digit(&self) -> bool {
        let next = self.current + 1;
        next < self.source.len() && self.source[next].is_ascii_digit()
    }

    /// Advance past the current character (not a newline).
    fn advance(&mut self) {
        self.current += 1;
        self.column += 1;
    }

    /// Advance past a newline character, updating line/column.
    fn advance_newline(&mut self) {
        self.current += 1;
        self.line += 1;
        self.column = 1;
    }

    /// Get the current source position as a Span.
    fn current_span(&self) -> Span {
        Span {
            line: self.line,
            column: self.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: tokenize and assert no errors
    fn tokenize_ok(source: &str) -> Vec<Token> {
        let result = Lexer::new(source).tokenize();
        assert!(
            result.errors.is_empty(),
            "expected no errors, got: {:?}",
            result.errors
        );
        result.tokens
    }

    // Helper: tokenize and return errors
    fn tokenize_errors(source: &str) -> (Vec<Token>, Vec<LexerError>) {
        let result = Lexer::new(source).tokenize();
        (result.tokens, result.errors)
    }

    // --- Basic tests ---

    #[test]
    fn empty_source() {
        let tokens = tokenize_ok("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
    }

    #[test]
    fn simple_identifier() {
        let tokens = tokenize_ok("print");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "print");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn multiple_identifiers() {
        let tokens = tokenize_ok("foo bar baz");
        assert_eq!(tokens.len(), 4); // 3 identifiers + EOF
        assert_eq!(tokens[0].lexeme, "foo");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
        assert_eq!(tokens[1].lexeme, "bar");
        assert_eq!(tokens[1].span, Span { line: 1, column: 5 });
        assert_eq!(tokens[2].lexeme, "baz");
        assert_eq!(tokens[2].span, Span { line: 1, column: 9 });
        assert_eq!(tokens[3].kind, TokenKind::Eof);
    }

    #[test]
    fn left_paren() {
        let tokens = tokenize_ok("(");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::LeftParen);
        assert_eq!(tokens[0].lexeme, "(");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
    }

    #[test]
    fn right_paren() {
        let tokens = tokenize_ok(")");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::RightParen);
        assert_eq!(tokens[0].lexeme, ")");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
    }

    #[test]
    fn simple_string() {
        let tokens = tokenize_ok("\"Hello\"");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"Hello\"");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
    }

    #[test]
    fn empty_string() {
        let tokens = tokenize_ok("\"\"");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"\"");
    }

    #[test]
    fn complete_program() {
        let tokens = tokenize_ok("print(\"Hello, Flux!\")");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "print");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });

        assert_eq!(tokens[1].kind, TokenKind::LeftParen);
        assert_eq!(tokens[1].lexeme, "(");
        assert_eq!(tokens[1].span, Span { line: 1, column: 6 });

        assert_eq!(tokens[2].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[2].lexeme, "\"Hello, Flux!\"");
        assert_eq!(tokens[2].span, Span { line: 1, column: 7 });

        assert_eq!(tokens[3].kind, TokenKind::RightParen);
        assert_eq!(tokens[3].lexeme, ")");
        assert_eq!(
            tokens[3].span,
            Span {
                line: 1,
                column: 21
            }
        );

        assert_eq!(tokens[4].kind, TokenKind::Eof);
    }

    #[test]
    fn whitespace_between_tokens() {
        let tokens = tokenize_ok("  print  (  )  ");
        assert_eq!(tokens.len(), 4); // print ( ) EOF
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].span, Span { line: 1, column: 3 });
        assert_eq!(tokens[1].kind, TokenKind::LeftParen);
        assert_eq!(
            tokens[1].span,
            Span {
                line: 1,
                column: 10
            }
        );
        assert_eq!(tokens[2].kind, TokenKind::RightParen);
        assert_eq!(
            tokens[2].span,
            Span {
                line: 1,
                column: 13
            }
        );
    }

    #[test]
    fn newlines_and_source_locations() {
        let tokens = tokenize_ok("foo\nbar\nbaz");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
        assert_eq!(tokens[1].span, Span { line: 2, column: 1 });
        assert_eq!(tokens[2].span, Span { line: 3, column: 1 });
    }

    #[test]
    fn underscores_in_identifiers() {
        let tokens = tokenize_ok("_foo __bar baz_qux _");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].lexeme, "_foo");
        assert_eq!(tokens[1].lexeme, "__bar");
        assert_eq!(tokens[2].lexeme, "baz_qux");
        assert_eq!(tokens[3].lexeme, "_");
    }

    #[test]
    fn digits_after_first_identifier_character() {
        let tokens = tokenize_ok("x1 foo123 _99");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].lexeme, "x1");
        assert_eq!(tokens[1].lexeme, "foo123");
        assert_eq!(tokens[2].lexeme, "_99");
    }

    #[test]
    fn unsupported_characters() {
        let (tokens, errors) = tokenize_errors("@#$");
        // Should still produce EOF
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
        // Three errors for three invalid characters
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].message, "unexpected character '@'");
        assert_eq!(errors[0].span, Span { line: 1, column: 1 });
        assert_eq!(errors[1].message, "unexpected character '#'");
        assert_eq!(errors[1].span, Span { line: 1, column: 2 });
        assert_eq!(errors[2].message, "unexpected character '$'");
        assert_eq!(errors[2].span, Span { line: 1, column: 3 });
    }

    #[test]
    fn unsupported_character_among_valid_tokens() {
        let (tokens, errors) = tokenize_errors("print @ (");
        assert_eq!(tokens.len(), 3); // print ( EOF
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::LeftParen);
        assert_eq!(tokens[2].kind, TokenKind::Eof);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].span, Span { line: 1, column: 7 });
    }

    #[test]
    fn unterminated_string_at_eof() {
        let (tokens, errors) = tokenize_errors("\"hello");
        assert_eq!(tokens.len(), 1); // just EOF
        assert_eq!(tokens[0].kind, TokenKind::Eof);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "unterminated string literal");
        assert_eq!(errors[0].span, Span { line: 1, column: 1 });
    }

    #[test]
    fn unterminated_string_at_newline() {
        let (tokens, errors) = tokenize_errors("\"hello\nworld\"");
        // First error: the opening string hits a newline
        assert_eq!(
            errors[0].message,
            "unterminated string literal (newline in string)"
        );
        assert_eq!(errors[0].span, Span { line: 1, column: 1 });
        // After recovery: newline is skipped, "world" becomes an identifier,
        // then the trailing `"` starts another unterminated string (hits EOF).
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[1].message, "unterminated string literal");
        // "world" should appear as an identifier token
        let idents: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Identifier)
            .collect();
        assert_eq!(idents.len(), 1);
        assert_eq!(idents[0].lexeme, "world");
    }

    #[test]
    fn exactly_one_eof_token() {
        let tokens = tokenize_ok("a b c");
        let eof_count = tokens.iter().filter(|t| t.kind == TokenKind::Eof).count();
        assert_eq!(eof_count, 1);
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn tabs_are_whitespace() {
        let tokens = tokenize_ok("a\tb");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].lexeme, "a");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
        assert_eq!(tokens[1].lexeme, "b");
        assert_eq!(tokens[1].span, Span { line: 1, column: 3 });
    }

    #[test]
    fn carriage_return_newline() {
        let tokens = tokenize_ok("a\r\nb");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 });
        assert_eq!(tokens[1].span, Span { line: 2, column: 1 });
    }

    #[test]
    fn multiline_program() {
        let source = "print(\"one\")\nprint(\"two\")";
        let tokens = tokenize_ok(source);
        assert_eq!(tokens.len(), 9); // 4 tokens per line + EOF
        assert_eq!(tokens[4].span, Span { line: 2, column: 1 });
    }

    #[test]
    fn string_with_spaces() {
        let tokens = tokenize_ok("\"hello world\"");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"hello world\"");
    }

    #[test]
    fn string_with_parens_inside() {
        let tokens = tokenize_ok("\"(hello)\"");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"(hello)\"");
    }

    // --- Number tests ---

    #[test]
    fn integer_literal() {
        let tokens = tokenize_ok("42");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[0].lexeme, "42");
    }

    #[test]
    fn zero_literal() {
        let tokens = tokenize_ok("0");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[0].lexeme, "0");
    }

    #[test]
    fn float_literal() {
        let tokens = tokenize_ok("3.14");
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral);
        assert_eq!(tokens[0].lexeme, "3.14");
    }

    #[test]
    fn float_zero_point_five() {
        let tokens = tokenize_ok("0.5");
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral);
        assert_eq!(tokens[0].lexeme, "0.5");
    }

    #[test]
    fn boolean_true() {
        // Booleans are identifiers at the lexer level
        let tokens = tokenize_ok("true");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "true");
    }

    #[test]
    fn boolean_false() {
        let tokens = tokenize_ok("false");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "false");
    }

    // --- Operator tests ---

    #[test]
    fn plus_operator() {
        let tokens = tokenize_ok("+");
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[0].lexeme, "+");
    }

    #[test]
    fn minus_operator() {
        let tokens = tokenize_ok("-");
        assert_eq!(tokens[0].kind, TokenKind::Minus);
        assert_eq!(tokens[0].lexeme, "-");
    }

    #[test]
    fn star_operator() {
        let tokens = tokenize_ok("*");
        assert_eq!(tokens[0].kind, TokenKind::Star);
        assert_eq!(tokens[0].lexeme, "*");
    }

    #[test]
    fn slash_operator() {
        let tokens = tokenize_ok("/");
        assert_eq!(tokens[0].kind, TokenKind::Slash);
        assert_eq!(tokens[0].lexeme, "/");
    }

    #[test]
    fn arithmetic_expression_tokens() {
        let tokens = tokenize_ok("10 + 20 * 3");
        assert_eq!(tokens.len(), 6); // 10 + 20 * 3 EOF
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[0].lexeme, "10");
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[2].lexeme, "20");
        assert_eq!(tokens[3].kind, TokenKind::Star);
        assert_eq!(tokens[4].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[4].lexeme, "3");
    }

    #[test]
    fn parenthesized_expression_tokens() {
        let tokens = tokenize_ok("(10 + 20) * 3");
        assert_eq!(tokens.len(), 8); // ( 10 + 20 ) * 3 EOF
        assert_eq!(tokens[0].kind, TokenKind::LeftParen);
        assert_eq!(tokens[1].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[2].kind, TokenKind::Plus);
        assert_eq!(tokens[3].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[4].kind, TokenKind::RightParen);
        assert_eq!(tokens[5].kind, TokenKind::Star);
        assert_eq!(tokens[6].kind, TokenKind::IntegerLiteral);
    }

    // --- Equals and let tests ---

    #[test]
    fn equals_token() {
        let tokens = tokenize_ok("=");
        assert_eq!(tokens[0].kind, TokenKind::Equals);
        assert_eq!(tokens[0].lexeme, "=");
    }

    #[test]
    fn let_is_identifier() {
        let tokens = tokenize_ok("let");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "let");
    }

    #[test]
    fn let_declaration_tokens() {
        let tokens = tokenize_ok("let x = 10");
        assert_eq!(tokens.len(), 5); // let x = 10 EOF
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "let");
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "x");
        assert_eq!(tokens[2].kind, TokenKind::Equals);
        assert_eq!(tokens[3].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[3].lexeme, "10");
    }

    // --- Comparison and logical operator tests ---

    #[test]
    fn greater_token() {
        let tokens = tokenize_ok(">");
        assert_eq!(tokens[0].kind, TokenKind::Greater);
        assert_eq!(tokens[0].lexeme, ">");
    }

    #[test]
    fn greater_equal_token() {
        let tokens = tokenize_ok(">=");
        assert_eq!(tokens.len(), 2); // >= EOF (one token, not two)
        assert_eq!(tokens[0].kind, TokenKind::GreaterEqual);
        assert_eq!(tokens[0].lexeme, ">=");
    }

    #[test]
    fn less_token() {
        let tokens = tokenize_ok("<");
        assert_eq!(tokens[0].kind, TokenKind::Less);
        assert_eq!(tokens[0].lexeme, "<");
    }

    #[test]
    fn less_equal_token() {
        let tokens = tokenize_ok("<=");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::LessEqual);
        assert_eq!(tokens[0].lexeme, "<=");
    }

    #[test]
    fn equal_equal_token() {
        let tokens = tokenize_ok("==");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::EqualEqual);
        assert_eq!(tokens[0].lexeme, "==");
    }

    #[test]
    fn bang_equal_token() {
        let tokens = tokenize_ok("!=");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::BangEqual);
        assert_eq!(tokens[0].lexeme, "!=");
    }

    #[test]
    fn bang_token() {
        let tokens = tokenize_ok("!");
        assert_eq!(tokens[0].kind, TokenKind::Bang);
        assert_eq!(tokens[0].lexeme, "!");
    }

    #[test]
    fn amp_amp_token() {
        let tokens = tokenize_ok("&&");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::AmpAmp);
        assert_eq!(tokens[0].lexeme, "&&");
    }

    #[test]
    fn pipe_pipe_token() {
        let tokens = tokenize_ok("||");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::PipePipe);
        assert_eq!(tokens[0].lexeme, "||");
    }

    #[test]
    fn single_ampersand_is_bitwise_and() {
        let tokens = tokenize_ok("&");
        assert_eq!(tokens[0].kind, TokenKind::Amp);
    }

    #[test]
    fn single_pipe_is_bitwise_or() {
        let tokens = tokenize_ok("|");
        assert_eq!(tokens[0].kind, TokenKind::Pipe);
    }

    #[test]
    fn equals_still_works_for_let() {
        let tokens = tokenize_ok("let x = 10");
        assert_eq!(tokens[2].kind, TokenKind::Equals);
    }

    #[test]
    fn comparison_expression_tokens() {
        let tokens = tokenize_ok("10 >= 5 && x != 20");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[1].kind, TokenKind::GreaterEqual);
        assert_eq!(tokens[2].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[3].kind, TokenKind::AmpAmp);
        assert_eq!(tokens[4].kind, TokenKind::Identifier);
        assert_eq!(tokens[5].kind, TokenKind::BangEqual);
        assert_eq!(tokens[6].kind, TokenKind::IntegerLiteral);
    }

    // --- Single-quote string tests ---

    #[test]
    fn single_quote_string() {
        let tokens = tokenize_ok("'hello'");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        // Lexeme is normalized to double quotes internally
        assert_eq!(tokens[0].lexeme, "\"hello\"");
    }

    #[test]
    fn single_quote_empty_string() {
        let tokens = tokenize_ok("''");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"\"");
    }

    #[test]
    fn single_quote_with_double_quote_inside() {
        let tokens = tokenize_ok("'he said \"hi\"'");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"he said \"hi\"\"");
    }

    #[test]
    fn unterminated_single_quote_string() {
        let (_, errors) = tokenize_errors("'hello");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "unterminated string literal");
    }

    // --- Brace and control flow token tests ---

    #[test]
    fn left_brace_token() {
        let tokens = tokenize_ok("{");
        assert_eq!(tokens[0].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[0].lexeme, "{");
    }

    #[test]
    fn right_brace_token() {
        let tokens = tokenize_ok("}");
        assert_eq!(tokens[0].kind, TokenKind::RightBrace);
        assert_eq!(tokens[0].lexeme, "}");
    }

    #[test]
    fn if_is_identifier() {
        let tokens = tokenize_ok("if");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "if");
    }

    #[test]
    fn else_is_identifier() {
        let tokens = tokenize_ok("else");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "else");
    }

    #[test]
    fn if_else_block_tokens() {
        let tokens = tokenize_ok("if x > 10 { print(\"large\") } else { print(\"small\") }");
        assert_eq!(tokens[0].kind, TokenKind::Identifier); // if
        assert_eq!(tokens[0].lexeme, "if");
        assert_eq!(tokens[1].kind, TokenKind::Identifier); // x
        assert_eq!(tokens[2].kind, TokenKind::Greater);
        assert_eq!(tokens[3].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[4].kind, TokenKind::LeftBrace);
        // ... print("large") ...
        assert_eq!(tokens[9].kind, TokenKind::RightBrace);
        assert_eq!(tokens[10].kind, TokenKind::Identifier); // else
        assert_eq!(tokens[10].lexeme, "else");
        assert_eq!(tokens[11].kind, TokenKind::LeftBrace);
    }

    // --- While and assignment token tests ---

    #[test]
    fn while_is_identifier() {
        let tokens = tokenize_ok("while");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "while");
    }

    #[test]
    fn assignment_tokens() {
        let tokens = tokenize_ok("x = 10");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "x");
        assert_eq!(tokens[1].kind, TokenKind::Equals);
        assert_eq!(tokens[2].kind, TokenKind::IntegerLiteral);
    }

    #[test]
    fn equals_vs_equal_equal_distinct() {
        let tokens = tokenize_ok("x = 10 == 10");
        assert_eq!(tokens[1].kind, TokenKind::Equals);
        assert_eq!(tokens[3].kind, TokenKind::EqualEqual);
    }

    // --- Function token tests ---

    #[test]
    fn fn_is_identifier() {
        let tokens = tokenize_ok("fn");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "fn");
    }

    #[test]
    fn return_is_identifier() {
        let tokens = tokenize_ok("return");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "return");
    }

    #[test]
    fn comma_token() {
        let tokens = tokenize_ok(",");
        assert_eq!(tokens[0].kind, TokenKind::Comma);
        assert_eq!(tokens[0].lexeme, ",");
    }

    #[test]
    fn function_call_tokens() {
        let tokens = tokenize_ok("add(10, 20)");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::LeftParen);
        assert_eq!(tokens[2].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[3].kind, TokenKind::Comma);
        assert_eq!(tokens[4].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[5].kind, TokenKind::RightParen);
    }

    // --- Array token tests ---

    #[test]
    fn left_bracket_token() {
        let tokens = tokenize_ok("[");
        assert_eq!(tokens[0].kind, TokenKind::LeftBracket);
    }

    #[test]
    fn right_bracket_token() {
        let tokens = tokenize_ok("]");
        assert_eq!(tokens[0].kind, TokenKind::RightBracket);
    }

    #[test]
    fn array_literal_tokens() {
        let tokens = tokenize_ok("[1, 2, 3]");
        assert_eq!(tokens[0].kind, TokenKind::LeftBracket);
        assert_eq!(tokens[1].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[2].kind, TokenKind::Comma);
        assert_eq!(tokens[3].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[4].kind, TokenKind::Comma);
        assert_eq!(tokens[5].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[6].kind, TokenKind::RightBracket);
    }

    // --- Map token tests ---

    #[test]
    fn colon_token() {
        let tokens = tokenize_ok(":");
        assert_eq!(tokens[0].kind, TokenKind::Colon);
        assert_eq!(tokens[0].lexeme, ":");
    }

    #[test]
    fn map_literal_tokens() {
        let tokens = tokenize_ok("{\"a\": 1}");
        assert_eq!(tokens[0].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[2].kind, TokenKind::Colon);
        assert_eq!(tokens[3].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[4].kind, TokenKind::RightBrace);
    }

    // --- Module token tests ---

    #[test]
    fn dot_token() {
        let tokens = tokenize_ok(".");
        assert_eq!(tokens[0].kind, TokenKind::Dot);
    }

    #[test]
    fn import_is_identifier() {
        let tokens = tokenize_ok("import");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "import");
    }

    #[test]
    fn member_access_tokens() {
        let tokens = tokenize_ok("math.add");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "math");
        assert_eq!(tokens[1].kind, TokenKind::Dot);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
        assert_eq!(tokens[2].lexeme, "add");
    }

    #[test]
    fn float_still_works_with_dot() {
        let tokens = tokenize_ok("3.14");
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral);
        assert_eq!(tokens[0].lexeme, "3.14");
    }

    // === Stage 20: Range tokens ===

    #[test]
    fn lex_dot_dot() {
        let tokens = tokenize_ok("1..5");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[1].kind, TokenKind::DotDot);
        assert_eq!(tokens[1].lexeme, "..");
        assert_eq!(tokens[2].kind, TokenKind::IntegerLiteral);
    }

    #[test]
    fn lex_dot_dot_less() {
        let tokens = tokenize_ok("1..<5");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[1].kind, TokenKind::DotDotLess);
        assert_eq!(tokens[1].lexeme, "..<");
        assert_eq!(tokens[2].kind, TokenKind::IntegerLiteral);
    }

    #[test]
    fn lex_dot_dot_with_spaces() {
        let tokens = tokenize_ok("a .. b");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::DotDot);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
    }

    #[test]
    fn lex_dot_dot_less_with_spaces() {
        let tokens = tokenize_ok("a ..< b");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::DotDotLess);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
    }

    #[test]
    fn lex_dot_still_works() {
        let tokens = tokenize_ok("obj.method");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::Dot);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
    }

    #[test]
    fn lex_float_before_dot_dot() {
        let tokens = tokenize_ok("3.14");
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral);
        assert_eq!(tokens[0].lexeme, "3.14");
    }

    // === New operator tokens ===

    #[test]
    fn lex_star_star() {
        let tokens = tokenize_ok("**");
        assert_eq!(tokens[0].kind, TokenKind::StarStar);
    }

    #[test]
    fn lex_percent() {
        let tokens = tokenize_ok("%");
        assert_eq!(tokens[0].kind, TokenKind::Percent);
    }

    #[test]
    fn lex_tilde() {
        let tokens = tokenize_ok("~");
        assert_eq!(tokens[0].kind, TokenKind::Tilde);
    }

    #[test]
    fn lex_caret() {
        let tokens = tokenize_ok("^");
        assert_eq!(tokens[0].kind, TokenKind::Caret);
    }

    #[test]
    fn lex_caret_caret() {
        let tokens = tokenize_ok("^^");
        assert_eq!(tokens[0].kind, TokenKind::CaretCaret);
    }

    #[test]
    fn lex_shift_left() {
        let tokens = tokenize_ok("<<");
        assert_eq!(tokens[0].kind, TokenKind::LessLess);
    }

    #[test]
    fn lex_shift_right() {
        let tokens = tokenize_ok(">>");
        assert_eq!(tokens[0].kind, TokenKind::GreaterGreater);
    }

    #[test]
    fn lex_plus_equal() {
        let tokens = tokenize_ok("+=");
        assert_eq!(tokens[0].kind, TokenKind::PlusEqual);
    }

    #[test]
    fn lex_minus_equal() {
        let tokens = tokenize_ok("-=");
        assert_eq!(tokens[0].kind, TokenKind::MinusEqual);
    }

    #[test]
    fn lex_star_equal() {
        let tokens = tokenize_ok("*=");
        assert_eq!(tokens[0].kind, TokenKind::StarEqual);
    }

    #[test]
    fn lex_slash_equal() {
        let tokens = tokenize_ok("/=");
        assert_eq!(tokens[0].kind, TokenKind::SlashEqual);
    }

    #[test]
    fn lex_percent_equal() {
        let tokens = tokenize_ok("%=");
        assert_eq!(tokens[0].kind, TokenKind::PercentEqual);
    }

    #[test]
    fn lex_amp_equal() {
        let tokens = tokenize_ok("&=");
        assert_eq!(tokens[0].kind, TokenKind::AmpEqual);
    }

    #[test]
    fn lex_pipe_equal() {
        let tokens = tokenize_ok("|=");
        assert_eq!(tokens[0].kind, TokenKind::PipeEqual);
    }

    #[test]
    fn lex_caret_equal() {
        let tokens = tokenize_ok("^=");
        assert_eq!(tokens[0].kind, TokenKind::CaretEqual);
    }

    #[test]
    fn lex_shift_left_equal() {
        let tokens = tokenize_ok("<<=");
        assert_eq!(tokens[0].kind, TokenKind::LessLessEqual);
    }

    #[test]
    fn lex_shift_right_equal() {
        let tokens = tokenize_ok(">>=");
        assert_eq!(tokens[0].kind, TokenKind::GreaterGreaterEqual);
    }

    #[test]
    fn lex_disambiguate_star_star_vs_star() {
        let tokens = tokenize_ok("a ** b * c");
        assert_eq!(tokens[1].kind, TokenKind::StarStar);
        assert_eq!(tokens[3].kind, TokenKind::Star);
    }

    #[test]
    fn lex_disambiguate_amp_amp_vs_amp() {
        let tokens = tokenize_ok("a && b & c");
        assert_eq!(tokens[1].kind, TokenKind::AmpAmp);
        assert_eq!(tokens[3].kind, TokenKind::Amp);
    }

    #[test]
    fn lex_disambiguate_pipe_pipe_vs_pipe() {
        let tokens = tokenize_ok("a || b | c");
        assert_eq!(tokens[1].kind, TokenKind::PipePipe);
        assert_eq!(tokens[3].kind, TokenKind::Pipe);
    }

    #[test]
    fn lex_disambiguate_caret_caret_vs_caret() {
        let tokens = tokenize_ok("a ^^ b ^ c");
        assert_eq!(tokens[1].kind, TokenKind::CaretCaret);
        assert_eq!(tokens[3].kind, TokenKind::Caret);
    }

    #[test]
    fn lex_disambiguate_less_less_vs_less() {
        let tokens = tokenize_ok("a << b < c");
        assert_eq!(tokens[1].kind, TokenKind::LessLess);
        assert_eq!(tokens[3].kind, TokenKind::Less);
    }

    #[test]
    fn lex_disambiguate_greater_greater_vs_greater() {
        let tokens = tokenize_ok("a >> b > c");
        assert_eq!(tokens[1].kind, TokenKind::GreaterGreater);
        assert_eq!(tokens[3].kind, TokenKind::Greater);
    }

    // --- Duration Literals ---

    #[test]
    fn lex_duration_seconds() {
        let tokens = tokenize_ok("5s");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "5s");
    }

    #[test]
    fn lex_duration_milliseconds() {
        let tokens = tokenize_ok("100ms");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "100ms");
    }

    #[test]
    fn lex_duration_microseconds() {
        let tokens = tokenize_ok("500us");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "500us");
    }

    #[test]
    fn lex_duration_nanoseconds() {
        let tokens = tokenize_ok("100ns");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "100ns");
    }

    #[test]
    fn lex_duration_minutes() {
        let tokens = tokenize_ok("1m");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "1m");
    }

    #[test]
    fn lex_duration_hours() {
        let tokens = tokenize_ok("2h");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "2h");
    }

    #[test]
    fn lex_duration_days() {
        let tokens = tokenize_ok("3d");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "3d");
    }

    #[test]
    fn lex_duration_zero() {
        let tokens = tokenize_ok("0s");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "0s");
    }

    #[test]
    fn lex_integer_followed_by_non_suffix() {
        // `10x` should NOT lex as a duration — instead: integer + identifier
        let tokens = tokenize_ok("10 x");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[0].lexeme, "10");
    }

    #[test]
    fn lex_integer_not_duration_suffix() {
        // Invalid suffix like 'xyz' should produce integer + identifier
        let tokens = tokenize_ok("10xyz");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[0].lexeme, "10");
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "xyz");
    }

    #[test]
    fn lex_duration_in_expression() {
        let tokens = tokenize_ok("5s + 2s");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "5s");
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[2].lexeme, "2s");
    }

    #[test]
    fn lex_identifier_starting_with_duration_suffix() {
        // `msg` is a valid identifier, not a duration
        let tokens = tokenize_ok("msg");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "msg");
    }

    #[test]
    fn lex_float_not_duration() {
        // `3.14` should remain a float, not become a duration
        let tokens = tokenize_ok("3.14");
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral);
    }

    #[test]
    fn lex_duration_large_number() {
        let tokens = tokenize_ok("86400s");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "86400s");
    }

    #[test]
    fn lex_duration_uppercase_not_valid() {
        // 5S should NOT become a DurationLiteral (case-sensitive suffixes)
        let tokens = tokenize_ok("5S");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
        assert_eq!(tokens[0].lexeme, "5");
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "S");
    }

    #[test]
    fn lex_duration_no_space_before_brace() {
        // `5s{` should lex as DurationLiteral + LeftBrace
        let tokens = tokenize_ok("5s{");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].lexeme, "5s");
        assert_eq!(tokens[1].kind, TokenKind::LeftBrace);
    }

    #[test]
    fn lex_duration_in_after_context() {
        let tokens = tokenize_ok("after 5s { }");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "after");
        assert_eq!(tokens[1].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[1].lexeme, "5s");
        assert_eq!(tokens[2].kind, TokenKind::LeftBrace);
    }

    #[test]
    fn lex_duration_ms_uppercase_invalid() {
        // 5MS should NOT be a duration
        let tokens = tokenize_ok("5MS");
        assert_eq!(tokens[0].kind, TokenKind::IntegerLiteral);
    }
}
