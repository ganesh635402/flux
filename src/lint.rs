// Flux linter — lightweight static analysis for common issues.

use crate::ast::{Program, Statement};

/// A lint warning.
#[derive(Debug)]
pub struct LintWarning {
    pub message: String,
    pub line: usize,
}

/// Lint a parsed Flux program and return warnings.
pub fn lint_program(program: &Program) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    for stmt in &program.statements {
        lint_statement(stmt, &mut warnings);
    }

    warnings
}

fn lint_statement(stmt: &Statement, warnings: &mut Vec<LintWarning>) {
    match stmt {
        Statement::If(if_stmt) => {
            // Check for empty then block
            if if_stmt.then_branch.statements.is_empty() {
                warnings.push(LintWarning {
                    message: "empty if block".to_string(),
                    line: if_stmt.span.line,
                });
            }
            // Check else block
            if let Some(ref else_block) = if_stmt.else_branch {
                if else_block.statements.is_empty() {
                    warnings.push(LintWarning {
                        message: "empty else block".to_string(),
                        line: if_stmt.span.line,
                    });
                }
            }
            // Recurse into blocks
            for s in &if_stmt.then_branch.statements {
                lint_statement(s, warnings);
            }
            if let Some(ref else_block) = if_stmt.else_branch {
                for s in &else_block.statements {
                    lint_statement(s, warnings);
                }
            }
        }
        Statement::While(while_stmt) => {
            if while_stmt.body.statements.is_empty() {
                warnings.push(LintWarning {
                    message: "empty while loop body".to_string(),
                    line: while_stmt.span.line,
                });
            }
            for s in &while_stmt.body.statements {
                lint_statement(s, warnings);
            }
        }
        Statement::For(for_stmt) => {
            if for_stmt.body.statements.is_empty() {
                warnings.push(LintWarning {
                    message: "empty for loop body".to_string(),
                    line: for_stmt.span.line,
                });
            }
            for s in &for_stmt.body.statements {
                lint_statement(s, warnings);
            }
        }
        Statement::Function(func) => {
            if func.body.statements.is_empty() {
                warnings.push(LintWarning {
                    message: format!("empty function body in '{}'", func.name),
                    line: func.span.line,
                });
            }
            for s in &func.body.statements {
                lint_statement(s, warnings);
            }
        }
        Statement::TryCatch(tc) => {
            for s in &tc.try_body.statements {
                lint_statement(s, warnings);
            }
            if let Some(ref catch_body) = tc.catch_body {
                for s in &catch_body.statements {
                    lint_statement(s, warnings);
                }
            }
            if let Some(ref finally_body) = tc.finally_body {
                for s in &finally_body.statements {
                    lint_statement(s, warnings);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn lint(source: &str) -> Vec<LintWarning> {
        let lex = Lexer::new(source).tokenize();
        assert!(lex.errors.is_empty());
        let parse = Parser::new(lex.tokens).parse();
        assert!(parse.errors.is_empty());
        lint_program(&parse.program)
    }

    #[test]
    fn no_warnings_for_valid_code() {
        let warnings = lint("let x = 10\nprint(x)");
        assert!(warnings.is_empty());
    }

    #[test]
    fn warn_empty_if_block() {
        let warnings = lint("if true { }");
        assert!(!warnings.is_empty());
        assert!(warnings[0].message.contains("empty if"));
    }

    #[test]
    fn warn_empty_while_body() {
        let warnings = lint("while true { }");
        assert!(!warnings.is_empty());
        assert!(warnings[0].message.contains("empty while"));
    }

    #[test]
    fn warn_empty_function() {
        let warnings = lint("fn f() { }");
        assert!(!warnings.is_empty());
        assert!(warnings[0].message.contains("empty function"));
    }

    #[test]
    fn no_warning_for_nonempty_function() {
        let warnings = lint("fn f() { print(1) }");
        assert!(warnings.is_empty());
    }

    #[test]
    fn warn_empty_for_body() {
        let warnings = lint("for x in [1, 2] { }");
        assert!(!warnings.is_empty());
        assert!(warnings[0].message.contains("empty for"));
    }
}
