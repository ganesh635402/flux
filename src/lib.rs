pub mod ast;
pub mod diagnostic;
pub mod formatter;
pub mod interpreter;
pub mod lexer;
pub mod lint;
pub mod module_loader;
pub mod package;
pub mod parser;
pub mod repl;
pub mod runtime;
pub mod scheduler;
pub mod stdlib;
pub mod time;

use std::path::{Path, PathBuf};

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;
use runtime::Output;

/// The result of running a Flux program.
pub enum RunResult {
    /// Program executed successfully.
    Ok,
    /// Lexer errors occurred.
    LexErrors(Vec<String>),
    /// Parser errors occurred.
    ParseErrors(Vec<String>),
    /// Runtime errors occurred.
    RuntimeErrors(Vec<String>),
}

/// Resolve a source file path, automatically appending `.flux` if needed.
/// Returns the resolved path or None if no file is found.
pub fn resolve_source_path(input: &str) -> Option<PathBuf> {
    let path = Path::new(input);
    if path.exists() && path.is_file() {
        return Some(path.to_path_buf());
    }
    // Try appending .flux
    if path.extension().is_none() {
        let with_ext = path.with_extension("flux");
        if with_ext.exists() && with_ext.is_file() {
            return Some(with_ext);
        }
    }
    None
}

/// Run a Flux source string. Returns a RunResult.
/// `filename` is used for error diagnostics. `base_dir` is used for module resolution.
pub fn run_source(
    source: &str,
    filename: &str,
    base_dir: &Path,
    output: &mut impl Output,
) -> RunResult {
    // Lex
    let lex_result = Lexer::new(source).tokenize();
    if !lex_result.errors.is_empty() {
        let rendered: Vec<String> = lex_result
            .errors
            .iter()
            .map(|e| diagnostic::render_lexer_error(e, source, filename))
            .collect();
        return RunResult::LexErrors(rendered);
    }

    // Parse
    let parse_result = Parser::new(lex_result.tokens).parse();
    if !parse_result.errors.is_empty() {
        let rendered: Vec<String> = parse_result
            .errors
            .iter()
            .map(|e| diagnostic::render_parse_error(e, source, filename))
            .collect();
        return RunResult::ParseErrors(rendered);
    }

    // Execute
    let mut interp = Interpreter::new(output);
    interp.set_base_dir(base_dir.to_path_buf());
    interp.set_source_file(filename.to_string());
    let mut errors = interp.execute(&parse_result.program);
    if !errors.is_empty() {
        let rendered: Vec<String> = errors
            .iter()
            .map(|e| diagnostic::render_runtime_error(e, source, filename))
            .collect();
        return RunResult::RuntimeErrors(rendered);
    }

    // Run scheduler for pending after/every tasks
    if interp.has_scheduled_tasks() {
        let sched_errors = interp.run_scheduler();
        if !sched_errors.is_empty() {
            errors.extend(sched_errors);
            let rendered: Vec<String> = errors
                .iter()
                .map(|e| diagnostic::render_runtime_error(e, source, filename))
                .collect();
            return RunResult::RuntimeErrors(rendered);
        }
    }

    RunResult::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime::TestOutput;
    use std::fs;

    #[test]
    fn run_source_hello() {
        let mut output = TestOutput::new();
        let result = run_source("print(42)", "test.flux", Path::new("."), &mut output);
        assert!(matches!(result, RunResult::Ok));
        assert_eq!(output.lines, vec!["42"]);
    }

    #[test]
    fn run_source_lex_error() {
        let mut output = TestOutput::new();
        let result = run_source("let x = @", "test.flux", Path::new("."), &mut output);
        assert!(matches!(result, RunResult::LexErrors(_)));
    }

    #[test]
    fn run_source_parse_error() {
        let mut output = TestOutput::new();
        let result = run_source("let = 10", "test.flux", Path::new("."), &mut output);
        assert!(matches!(result, RunResult::ParseErrors(_)));
    }

    #[test]
    fn run_source_runtime_error() {
        let mut output = TestOutput::new();
        let result = run_source(
            "print(undefined_var)",
            "test.flux",
            Path::new("."),
            &mut output,
        );
        assert!(matches!(result, RunResult::RuntimeErrors(_)));
    }

    #[test]
    fn resolve_existing_flux_file() {
        // Create a temp .flux file
        let dir = std::env::temp_dir().join("flux_test_resolve");
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("hello.flux");
        fs::write(&file_path, "print(1)").unwrap();

        // Resolve without extension
        let resolved = resolve_source_path(dir.join("hello").to_str().unwrap());
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), file_path);

        // Resolve with extension
        let resolved = resolve_source_path(file_path.to_str().unwrap());
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), file_path);

        // Clean up
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_nonexistent_file() {
        let resolved = resolve_source_path("nonexistent_file_12345");
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_with_extension_directly() {
        let dir = std::env::temp_dir().join("flux_test_resolve2");
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.flux");
        fs::write(&file_path, "print(1)").unwrap();

        let resolved = resolve_source_path(file_path.to_str().unwrap());
        assert!(resolved.is_some());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_does_not_double_extension() {
        // If user passes "foo.flux" but only "foo.flux" exists (not "foo.flux.flux"),
        // it should resolve to "foo.flux"
        let dir = std::env::temp_dir().join("flux_test_resolve3");
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("bar.flux");
        fs::write(&file_path, "print(1)").unwrap();

        let resolved = resolve_source_path(file_path.to_str().unwrap());
        assert_eq!(resolved.unwrap(), file_path);

        fs::remove_dir_all(&dir).unwrap();
    }
}
