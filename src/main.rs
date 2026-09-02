use std::env;
use std::fs;
use std::path::Path;
use std::process;

use flux::package;
use flux::repl;
use flux::runtime::StdOutput;
use flux::{RunResult, resolve_source_path, run_source};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
Flux programming language

Usage:
    flux                Start interactive REPL
    flux <file>         Run a Flux program
    flux run <file>     Run a Flux program
    flux check [file]   Check a Flux program for errors
    flux fmt [file]     Format Flux source code
    flux test [dir]     Run Flux tests
    flux lint [file]    Lint Flux source code
    flux init [name]    Initialize a new Flux project
    flux deps           Show project dependencies
    flux repl           Start interactive REPL
    flux --version      Print version information
    flux --help         Print this help message

The .flux extension is optional:
    flux main           runs main.flux
    flux main.flux      runs main.flux

Examples:
    flux hello
    flux run examples/demo
    flux check main.flux
    flux fmt src/
    flux test
    flux init myproject
    flux deps";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        repl::start_interactive();
        process::exit(0);
    }

    let first = &args[1];

    // Handle flags
    match first.as_str() {
        "--version" | "-V" => {
            println!("Flux {}", VERSION);
            process::exit(0);
        }
        "--help" | "-h" => {
            println!("{}", HELP);
            process::exit(0);
        }
        _ => {}
    }

    // Determine the file argument
    let file_arg = if first == "run" {
        if args.len() < 3 {
            // Check for run --help
            if args.len() == 2 {
                eprintln!("Flux error: no source file specified after 'run'\n");
                eprintln!("{}", HELP);
                process::exit(2);
            }
            unreachable!()
        }
        let second = &args[2];
        if second == "--help" || second == "-h" {
            println!("{}", HELP);
            process::exit(0);
        }
        second.as_str()
    } else if first == "init" {
        // Initialize a new Flux project
        let name = if args.len() >= 3 {
            args[2].clone()
        } else {
            // Use current directory name
            env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "flux_project".to_string())
        };
        let dir = env::current_dir().unwrap_or_default();
        match package::init_project(&dir, &name) {
            Ok(()) => {
                println!(
                    "Created Flux project '{}' with flux.toml and src/main.flux",
                    name
                );
                process::exit(0);
            }
            Err(e) => {
                eprintln!("Flux error: {}", e);
                process::exit(1);
            }
        }
    } else if first == "deps" {
        // Show project dependencies
        let dir = env::current_dir().unwrap_or_default();
        match package::load_manifest(&dir) {
            Ok(manifest) => {
                println!(
                    "Package: {} v{}",
                    manifest.package.name, manifest.package.version
                );
                if manifest.dependencies.is_empty() {
                    println!("No dependencies.");
                } else {
                    println!("Dependencies:");
                    for (name, spec) in &manifest.dependencies {
                        match spec {
                            package::DependencySpec::Version(v) => {
                                println!("  {} = \"{}\"", name, v);
                            }
                            package::DependencySpec::Detailed(d) => {
                                if let Some(ref path) = d.path {
                                    println!("  {} (path: {})", name, path);
                                }
                                if let Some(ref ver) = d.version {
                                    println!("  {} (version: {})", name, ver);
                                }
                            }
                        }
                    }
                    // Check for resolution errors
                    match package::resolve_dependencies(&dir, &manifest) {
                        Ok((deps, _)) => {
                            println!("\nResolved {} dependency(ies).", deps.len());
                        }
                        Err(e) => {
                            eprintln!("\nResolution error: {}", e);
                            process::exit(1);
                        }
                    }
                }
                process::exit(0);
            }
            Err(e) => {
                eprintln!("Flux error: {}", e);
                process::exit(1);
            }
        }
    } else if first == "repl" {
        repl::start_interactive();
        process::exit(0);
    } else if first == "check" {
        // Check a Flux file for errors without executing
        let file = if args.len() >= 3 {
            args[2].as_str()
        } else {
            // Try src/main.flux in project
            "src/main.flux"
        };
        let resolved = match resolve_source_path(file) {
            Some(p) => p,
            None => {
                eprintln!("Flux error: source file '{}' not found", file);
                process::exit(1);
            }
        };
        let source = fs::read_to_string(&resolved).unwrap_or_else(|e| {
            eprintln!("Flux error: cannot read '{}': {}", resolved.display(), e);
            process::exit(1);
        });
        // Lex + Parse only (no execution)
        let lex_result = flux::lexer::Lexer::new(&source).tokenize();
        if !lex_result.errors.is_empty() {
            for err in &lex_result.errors {
                eprintln!(
                    "{}",
                    flux::diagnostic::render_lexer_error(
                        err,
                        &source,
                        &resolved.display().to_string()
                    )
                );
            }
            process::exit(1);
        }
        let parse_result = flux::parser::Parser::new(lex_result.tokens).parse();
        if !parse_result.errors.is_empty() {
            for err in &parse_result.errors {
                eprintln!(
                    "{}",
                    flux::diagnostic::render_parse_error(
                        err,
                        &source,
                        &resolved.display().to_string()
                    )
                );
            }
            process::exit(1);
        }
        println!("✓ {} — no errors", resolved.display());
        process::exit(0);
    } else if first == "fmt" {
        // Format a Flux file
        let file = if args.len() >= 3 {
            args[2].as_str()
        } else {
            "src/main.flux"
        };
        let check_mode = args.iter().any(|a| a == "--check");
        let resolved = match resolve_source_path(file) {
            Some(p) => p,
            None => {
                eprintln!("Flux error: source file '{}' not found", file);
                process::exit(1);
            }
        };
        let source = fs::read_to_string(&resolved).unwrap_or_else(|e| {
            eprintln!("Flux error: cannot read '{}': {}", resolved.display(), e);
            process::exit(1);
        });
        let formatted = flux::formatter::format_source(&source);
        if check_mode {
            if formatted != source {
                eprintln!("Would reformat {}", resolved.display());
                process::exit(1);
            } else {
                println!("✓ {} — already formatted", resolved.display());
                process::exit(0);
            }
        } else {
            fs::write(&resolved, &formatted).unwrap_or_else(|e| {
                eprintln!("Flux error: cannot write '{}': {}", resolved.display(), e);
                process::exit(1);
            });
            println!("Formatted {}", resolved.display());
            process::exit(0);
        }
    } else if first == "test" {
        // Run Flux tests
        let test_dir = if args.len() >= 3 {
            args[2].clone()
        } else {
            "tests".to_string()
        };
        let test_path = Path::new(&test_dir);
        if !test_path.exists() {
            println!("No tests directory found. Create tests/ with .flux test files.");
            process::exit(0);
        }
        let mut passed = 0;
        let mut failed = 0;
        let mut total = 0;
        if let Ok(entries) = fs::read_dir(test_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "flux") {
                    total += 1;
                    let source = fs::read_to_string(&path).unwrap_or_default();
                    let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let mut output = StdOutput;
                    let result = run_source(&source, &name, &base_dir, &mut output);
                    match result {
                        RunResult::Ok => {
                            passed += 1;
                            println!("  ✓ {}", name);
                        }
                        _ => {
                            failed += 1;
                            println!("  ✗ {}", name);
                        }
                    }
                }
            }
        }
        println!("\n{} passed, {} failed, {} total", passed, failed, total);
        if failed > 0 {
            process::exit(1);
        }
        process::exit(0);
    } else if first == "lint" {
        // Basic linting
        let file = if args.len() >= 3 {
            args[2].as_str()
        } else {
            "src/main.flux"
        };
        let resolved = match resolve_source_path(file) {
            Some(p) => p,
            None => {
                eprintln!("Flux error: source file '{}' not found", file);
                process::exit(1);
            }
        };
        let source = fs::read_to_string(&resolved).unwrap_or_else(|e| {
            eprintln!("Flux error: cannot read '{}': {}", resolved.display(), e);
            process::exit(1);
        });
        let lex_result = flux::lexer::Lexer::new(&source).tokenize();
        if !lex_result.errors.is_empty() {
            for err in &lex_result.errors {
                eprintln!(
                    "{}",
                    flux::diagnostic::render_lexer_error(
                        err,
                        &source,
                        &resolved.display().to_string()
                    )
                );
            }
            process::exit(1);
        }
        let parse_result = flux::parser::Parser::new(lex_result.tokens).parse();
        if !parse_result.errors.is_empty() {
            for err in &parse_result.errors {
                eprintln!(
                    "{}",
                    flux::diagnostic::render_parse_error(
                        err,
                        &source,
                        &resolved.display().to_string()
                    )
                );
            }
            process::exit(1);
        }
        // Basic lint: check for empty blocks
        let warnings = flux::lint::lint_program(&parse_result.program);
        if warnings.is_empty() {
            println!("✓ {} — no warnings", resolved.display());
        } else {
            for w in &warnings {
                println!(
                    "warning: {} at {}:{}",
                    w.message,
                    resolved.display(),
                    w.line
                );
            }
            println!("\n{} warning(s)", warnings.len());
        }
        process::exit(0);
    } else {
        // Check for unknown flags
        if first.starts_with('-') {
            eprintln!("Flux error: unknown option '{}'\n", first);
            eprintln!("{}", HELP);
            process::exit(2);
        }
        first.as_str()
    };

    // Resolve the source file
    let resolved = match resolve_source_path(file_arg) {
        Some(path) => path,
        None => {
            // Produce a helpful error showing what was tried
            let tried = if Path::new(file_arg).extension().is_none() {
                format!("'{}' or '{}.flux'", file_arg, file_arg)
            } else {
                format!("'{}'", file_arg)
            };
            eprintln!("Flux error: source file {} not found", tried);
            process::exit(1);
        }
    };

    // Read the source
    let source = fs::read_to_string(&resolved).unwrap_or_else(|err| {
        eprintln!("Flux error: cannot read '{}': {}", resolved.display(), err);
        process::exit(1);
    });

    let base_dir = resolved.parent().unwrap_or(Path::new(".")).to_path_buf();

    let display_name = resolved
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Run
    let mut output = StdOutput;
    let result = run_source(&source, &display_name, &base_dir, &mut output);

    match result {
        RunResult::Ok => {}
        RunResult::LexErrors(errs)
        | RunResult::ParseErrors(errs)
        | RunResult::RuntimeErrors(errs) => {
            for e in &errs {
                eprint!("{}", e);
            }
            process::exit(1);
        }
    }
}
