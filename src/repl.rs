// Flux REPL — interactive Read-Eval-Print Loop.

use std::io::{self, BufRead, Write};

use crate::diagnostic;
use crate::interpreter::Interpreter;
use crate::lexer::{Lexer, TokenKind};
use crate::parser::Parser;
use crate::runtime::{Output, StdOutput, Value};

/// The result of processing one line of REPL input.
pub enum ReplResult {
    /// Processing complete. Contains lines to display (results, errors, help).
    Output(Vec<String>),
    /// Incomplete input — need more lines (multiline construct).
    NeedMore,
    /// User requested exit.
    Quit,
}

/// A REPL session with persistent interpreter state.
pub struct ReplSession<'a, O: Output> {
    interpreter: Interpreter<'a, O>,
    input_buffer: String,
    in_multiline: bool,
}

impl<'a, O: Output> ReplSession<'a, O> {
    /// Create a new REPL session.
    pub fn new(output: &'a mut O) -> Self {
        let mut interpreter = Interpreter::new(output);
        interpreter.set_repl_mode(true);
        interpreter.set_source_file("<repl>".to_string());
        ReplSession {
            interpreter,
            input_buffer: String::new(),
            in_multiline: false,
        }
    }

    /// Whether the session is currently waiting for more multiline input.
    pub fn is_in_multiline(&self) -> bool {
        self.in_multiline
    }

    /// Process one line of REPL input.
    pub fn process_line(&mut self, line: &str) -> ReplResult {
        let trimmed = line.trim();

        // :quit / :exit always works, even in multiline mode
        if trimmed == ":quit" || trimmed == ":exit" {
            return ReplResult::Quit;
        }

        // Handle empty input at top level
        if !self.in_multiline && trimmed.is_empty() {
            return ReplResult::Output(vec![]);
        }

        // Handle REPL commands at top level
        if !self.in_multiline && trimmed.starts_with(':') {
            return self.handle_command(trimmed);
        }

        // Accumulate input
        self.input_buffer.push_str(line);
        if !self.input_buffer.ends_with('\n') {
            self.input_buffer.push('\n');
        }

        // Check if input is complete
        if !is_input_complete(&self.input_buffer) {
            self.in_multiline = true;
            return ReplResult::NeedMore;
        }

        // Input is complete — execute it
        self.in_multiline = false;
        let source = self.input_buffer.trim().to_string();
        self.input_buffer.clear();

        if source.is_empty() {
            return ReplResult::Output(vec![]);
        }

        self.execute_source(&source)
    }

    /// Execute a complete source string and return display lines.
    fn execute_source(&mut self, source: &str) -> ReplResult {
        // Lex
        let lex_result = Lexer::new(source).tokenize();
        if !lex_result.errors.is_empty() {
            let lines: Vec<String> = lex_result
                .errors
                .iter()
                .map(|e| diagnostic::render_lexer_error(e, source, "<repl>"))
                .collect();
            return ReplResult::Output(lines);
        }

        // Parse (REPL mode: bare expressions are allowed)
        let parse_result = Parser::new(lex_result.tokens).parse_repl();
        if !parse_result.errors.is_empty() {
            let lines: Vec<String> = parse_result
                .errors
                .iter()
                .map(|e| diagnostic::render_parse_error(e, source, "<repl>"))
                .collect();
            return ReplResult::Output(lines);
        }

        // Execute
        let (value, errors) = self.interpreter.execute_repl(&parse_result.program);

        if !errors.is_empty() {
            let lines: Vec<String> = errors
                .iter()
                .map(|e| diagnostic::render_runtime_error(e, source, "<repl>"))
                .collect();
            return ReplResult::Output(lines);
        }

        // Display expression result (non-Nil values)
        match value {
            Some(val) if !matches!(val, Value::Nil) => ReplResult::Output(vec![format!("{}", val)]),
            _ => ReplResult::Output(vec![]),
        }
    }

    /// Handle a REPL command (e.g., :help, :clear).
    fn handle_command(&mut self, cmd: &str) -> ReplResult {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let command = parts[0];
        let arg = parts.get(1).map(|s| s.trim());

        match command {
            ":help" => ReplResult::Output(vec![
                "REPL commands:".to_string(),
                "  :help       Show this help".to_string(),
                "  :clear      Reset the environment".to_string(),
                "  :reset      Reset the environment".to_string(),
                "  :version    Show Flux version".to_string(),
                "  :type <expr> Show the type of an expression".to_string(),
                "  :load <file> Load and execute a Flux file".to_string(),
                "  :quit       Exit the REPL".to_string(),
                "  :exit       Exit the REPL".to_string(),
            ]),
            ":clear" | ":reset" => {
                self.interpreter.reset();
                ReplResult::Output(vec!["Environment cleared.".to_string()])
            }
            ":version" => ReplResult::Output(vec![format!("Flux {}", env!("CARGO_PKG_VERSION"))]),
            ":type" => {
                if let Some(expr_str) = arg {
                    let result = self.execute_source(&format!("print(type_of({}))", expr_str));
                    result
                } else {
                    ReplResult::Output(vec!["Usage: :type <expression>".to_string()])
                }
            }
            ":load" => {
                if let Some(file) = arg {
                    match std::fs::read_to_string(file) {
                        Ok(source) => self.execute_source(&source),
                        Err(e) => {
                            ReplResult::Output(vec![format!("Cannot load '{}': {}", file, e)])
                        }
                    }
                } else {
                    ReplResult::Output(vec!["Usage: :load <file>".to_string()])
                }
            }
            _ => ReplResult::Output(vec![format!("Unknown command: {}", cmd)]),
        }
    }
}

/// Check whether the accumulated input is complete (all delimiters matched).
/// Uses the Flux lexer to respect string boundaries.
fn is_input_complete(source: &str) -> bool {
    let lex_result = Lexer::new(source).tokenize();
    let mut depth: i32 = 0;

    for token in &lex_result.tokens {
        match token.kind {
            TokenKind::LeftBrace | TokenKind::LeftBracket | TokenKind::LeftParen => depth += 1,
            TokenKind::RightBrace | TokenKind::RightBracket | TokenKind::RightParen => depth -= 1,
            _ => {}
        }
    }

    depth <= 0
}

/// Start the interactive REPL using stdin/stdout.
pub fn start_interactive() {
    println!("Flux {}", env!("CARGO_PKG_VERSION"));
    println!("Interactive mode. Type :help for help, :quit to exit.");
    println!();

    let mut output = StdOutput;
    let mut session = ReplSession::new(&mut output);

    let stdin = io::stdin();

    loop {
        if session.is_in_multiline() {
            print!("... ");
        } else {
            print!(">>> ");
        }
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        match session.process_line(&line) {
            ReplResult::Output(lines) => {
                for l in &lines {
                    println!("{}", l);
                }
            }
            ReplResult::NeedMore => {}
            ReplResult::Quit => break,
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::TestOutput;

    #[test]
    fn repl_basic_integer() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("42\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["42"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_basic_addition() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("10 + 20\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["30"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_boolean_expression() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("10 > 5\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["true"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_string_concat() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("\"Hello\" + \" Flux\"\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["Hello Flux"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_array_literal() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("[1, 2, 3]\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["[1, 2, 3]"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_persistent_variable() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // Define variable — no display output
        match session.process_line("let x = 10\n") {
            ReplResult::Output(lines) => assert!(lines.is_empty()),
            _ => panic!("expected Output"),
        }

        // Use variable
        match session.process_line("x\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["10"]),
            _ => panic!("expected Output"),
        }

        // Use in expression
        match session.process_line("x + 5\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["15"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_persistent_function() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // Define function (single line)
        match session.process_line("fn add(a, b) { return a + b }\n") {
            ReplResult::Output(lines) => assert!(lines.is_empty()),
            _ => panic!("expected Output"),
        }

        // Call function
        match session.process_line("add(2, 3)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["5"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_multiline_function() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // First line — incomplete
        match session.process_line("fn square(n) {\n") {
            ReplResult::NeedMore => {}
            other => panic!(
                "expected NeedMore, got {:?}",
                match other {
                    ReplResult::Output(l) => format!("Output({:?})", l),
                    ReplResult::Quit => "Quit".to_string(),
                    _ => "?".to_string(),
                }
            ),
        }
        assert!(session.is_in_multiline());

        // Second line — still incomplete
        match session.process_line("    return n * n\n") {
            ReplResult::NeedMore => {}
            _ => panic!("expected NeedMore"),
        }

        // Closing brace — complete
        match session.process_line("}\n") {
            ReplResult::Output(lines) => assert!(lines.is_empty()),
            _ => panic!("expected Output"),
        }
        assert!(!session.is_in_multiline());

        // Call the function
        match session.process_line("square(7)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["49"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_error_recovery() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // Runtime error
        match session.process_line("undefined_var\n") {
            ReplResult::Output(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("undefined"));
            }
            _ => panic!("expected Output with error"),
        }

        // Session should still work after error
        match session.process_line("10 + 20\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["30"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_parse_error_recovery() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // Parse error
        match session.process_line("let = 10\n") {
            ReplResult::Output(lines) => {
                assert!(!lines.is_empty());
            }
            _ => panic!("expected Output with error"),
        }

        // Session continues
        match session.process_line("42\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["42"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_clear_command() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // Define variable
        session.process_line("let x = 10\n");

        // Verify it exists
        match session.process_line("x\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["10"]),
            _ => panic!("expected Output"),
        }

        // Clear
        match session.process_line(":clear\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["Environment cleared."]),
            _ => panic!("expected Output"),
        }

        // Variable should be gone
        match session.process_line("x\n") {
            ReplResult::Output(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("undefined"));
            }
            _ => panic!("expected Output with error"),
        }
    }

    #[test]
    fn repl_help_command() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line(":help\n") {
            ReplResult::Output(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("REPL commands"));
            }
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_version_command() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line(":version\n") {
            ReplResult::Output(lines) => {
                assert_eq!(lines.len(), 1);
                assert!(lines[0].starts_with("Flux "));
                assert!(lines[0].contains(env!("CARGO_PKG_VERSION")));
            }
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_quit_command() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line(":quit\n") {
            ReplResult::Quit => {}
            _ => panic!("expected Quit"),
        }
    }

    #[test]
    fn repl_exit_command() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line(":exit\n") {
            ReplResult::Quit => {}
            _ => panic!("expected Quit"),
        }
    }

    #[test]
    fn repl_quit_in_multiline() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        // Start multiline
        session.process_line("fn f() {\n");
        assert!(session.is_in_multiline());

        // :quit should still work
        match session.process_line(":quit\n") {
            ReplResult::Quit => {}
            _ => panic!("expected Quit"),
        }
    }

    #[test]
    fn repl_empty_input() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("\n") {
            ReplResult::Output(lines) => assert!(lines.is_empty()),
            _ => panic!("expected empty Output"),
        }
    }

    #[test]
    fn repl_print_no_extra_nil() {
        let mut output = TestOutput::new();
        {
            let mut session = ReplSession::new(&mut output);
            // print(42) should NOT display nil as a result
            match session.process_line("print(42)\n") {
                ReplResult::Output(lines) => assert!(lines.is_empty()),
                _ => panic!("expected empty Output"),
            }
        }
        // But print should have written to output
        assert_eq!(output.lines, vec!["42"]);
    }

    #[test]
    fn repl_redefine_variable() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        session.process_line("let x = 10\n");
        match session.process_line("x\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["10"]),
            _ => panic!("expected Output"),
        }

        // Redefine
        session.process_line("let x = 20\n");
        match session.process_line("x\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["20"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_redefine_function() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        session.process_line("fn f() { return 1 }\n");
        match session.process_line("f()\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["1"]),
            _ => panic!("expected Output"),
        }

        // Redefine
        session.process_line("fn f() { return 2 }\n");
        match session.process_line("f()\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["2"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_unknown_command() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line(":foo\n") {
            ReplResult::Output(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("Unknown command"));
            }
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_multiline_if() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        assert!(matches!(
            session.process_line("if true {\n"),
            ReplResult::NeedMore
        ));
        assert!(matches!(
            session.process_line("    10 + 20\n"),
            ReplResult::NeedMore
        ));
        // The closing brace completes the if statement
        match session.process_line("}\n") {
            ReplResult::Output(_) => {} // if statement doesn't produce a display value
            other => panic!("expected Output, got something else"),
        }
    }

    #[test]
    fn repl_for_loop() {
        let mut output = TestOutput::new();
        {
            let mut session = ReplSession::new(&mut output);
            session.process_line("for i in 1..3 { print(i) }\n");
        }
        assert_eq!(output.lines, vec!["1", "2", "3"]);
    }

    #[test]
    fn repl_input_complete_simple() {
        assert!(is_input_complete("10 + 20"));
        assert!(is_input_complete("let x = 10"));
        assert!(is_input_complete("[1, 2, 3]"));
    }

    #[test]
    fn repl_input_incomplete() {
        assert!(!is_input_complete("fn f() {"));
        assert!(!is_input_complete("if true {"));
        assert!(!is_input_complete("[1, 2,"));
    }

    #[test]
    fn repl_input_complete_nested() {
        assert!(is_input_complete("fn f() { if true { return 1 } }"));
        assert!(!is_input_complete("fn f() { if true {"));
    }

    #[test]
    fn repl_range_expression() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("1..5\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["1..5"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_map_literal() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("{\"a\": 1}\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["{\"a\": 1}"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_closure() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        session.process_line("let f = fn(x) { return x * 2 }\n");
        match session.process_line("f(21)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["42"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_destructuring() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);

        session.process_line("let [a, b] = [10, 20]\n");
        match session.process_line("a + b\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["30"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_logical_and() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("true && false\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["false"]),
            _ => panic!("expected Output"),
        }
    }

    // === Unary minus in REPL ===

    #[test]
    fn repl_negate_integer() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("-100\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["-100"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_negate_float() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("-3.14\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["-3.14"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_negate_variable() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        session.process_line("let x = 10\n");
        match session.process_line("-x\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["-10"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_negate_parenthesized() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("-(10 + 5)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["-15"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_binary_minus_unary() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("10 - -5\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["15"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_let_negative() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        session.process_line("let y = -100\n");
        match session.process_line("y\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["-100"]),
            _ => panic!("expected Output"),
        }
    }

    // === New operators in REPL ===

    #[test]
    fn repl_modulo() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("10 % 3\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["1"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_power() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("2 ** 10\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["1024"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_logical_xor() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("true ^^ false\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["true"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_bitwise_and() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("5 & 3\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["1"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_bitwise_xor() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("5 ^ 3\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["6"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_shift() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("1 << 4\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["16"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_compound_assign() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        session.process_line("let x = 10\n");
        session.process_line("x += 5\n");
        match session.process_line("x\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["15"]),
            _ => panic!("expected Output"),
        }
    }

    // === Temporal in REPL ===

    #[test]
    fn repl_duration_seconds() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("seconds(5)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["5s"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_duration_minutes() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("minutes(2)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["2m"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_duration_add() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("seconds(5) + seconds(10)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["15s"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_duration_comparison() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("seconds(10) > seconds(5)\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["true"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_now_type() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        match session.process_line("type(now())\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["Instant"]),
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn repl_instant_plus_duration() {
        let mut output = TestOutput::new();
        let mut session = ReplSession::new(&mut output);
        session.process_line("let start = now()\n");
        match session.process_line("type(start + seconds(30))\n") {
            ReplResult::Output(lines) => assert_eq!(lines, vec!["Instant"]),
            _ => panic!("expected Output"),
        }
    }
}
