// Flux module loader — resolves, loads, parses, and caches modules.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{FunctionDecl, Program, Statement};
use crate::interpreter::RuntimeError;
use crate::lexer::{Lexer, Span};
use crate::parser::Parser;

/// A parsed but not-yet-executed module.
pub struct ParsedModule {
    pub name: String,
    pub program: Program,
    pub functions: HashMap<String, FunctionDecl>,
    pub module_dir: PathBuf,
}

/// Module loading state for circular import detection.
#[derive(Debug, Clone, PartialEq)]
enum LoadState {
    Loading,
    Loaded,
}

/// The module loader: resolves paths, reads files, parses, and caches.
pub struct ModuleLoader {
    loaded: HashMap<PathBuf, bool>,
    state: HashMap<PathBuf, LoadState>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        ModuleLoader {
            loaded: HashMap::new(),
            state: HashMap::new(),
        }
    }

    /// Check if a module is already loaded.
    pub fn is_loaded(&self, module_path: &[String], base_dir: &Path) -> bool {
        let file_path = Self::resolve_path(module_path, base_dir);
        let canonical = file_path.canonicalize().unwrap_or(file_path);
        self.loaded.contains_key(&canonical)
    }

    /// Mark a module as loaded.
    pub fn mark_loaded(&mut self, module_path: &[String], base_dir: &Path) {
        let file_path = Self::resolve_path(module_path, base_dir);
        let canonical = file_path.canonicalize().unwrap_or(file_path);
        self.state.insert(canonical.clone(), LoadState::Loaded);
        self.loaded.insert(canonical, true);
    }

    /// Resolve a module path to a filesystem path.
    fn resolve_path(module_path: &[String], base_dir: &Path) -> PathBuf {
        if module_path.len() == 1 {
            let name = &module_path[0];
            // Support explicit .flux extension
            if name.ends_with(".flux") {
                base_dir.join(name)
            } else {
                base_dir.join(format!("{}.flux", name))
            }
        } else {
            // Nested: utils.math → utils/math.flux
            let mut path = base_dir.to_path_buf();
            for (i, segment) in module_path.iter().enumerate() {
                if i == module_path.len() - 1 {
                    path = path.join(format!("{}.flux", segment));
                } else {
                    path = path.join(segment);
                }
            }
            path
        }
    }

    /// Load and parse a module. Returns the parsed module for the interpreter to execute.
    pub fn load(
        &mut self,
        module_path: &[String],
        base_dir: &Path,
        span: &Span,
    ) -> Result<ParsedModule, RuntimeError> {
        let module_name = module_path.join(".");
        let file_path = Self::resolve_path(module_path, base_dir);
        let canonical = file_path.canonicalize().unwrap_or(file_path.clone());

        if self.loaded.contains_key(&canonical) {
            return Err(RuntimeError {
                call_stack: Vec::new(),
                message: format!("module '{}' already loaded", module_name),
                span: span.clone(),
            });
        }

        if self.state.get(&canonical) == Some(&LoadState::Loading) {
            return Err(RuntimeError {
                call_stack: Vec::new(),
                message: format!("circular module import detected: {}", module_name),
                span: span.clone(),
            });
        }

        if !file_path.exists() {
            return Err(RuntimeError {
                call_stack: Vec::new(),
                message: format!(
                    "module '{}' not found (looked for '{}')",
                    module_name,
                    file_path.display()
                ),
                span: span.clone(),
            });
        }

        let source = std::fs::read_to_string(&file_path).map_err(|err| RuntimeError {
            call_stack: Vec::new(),
            message: format!("failed to read module '{}': {}", module_name, err),
            span: span.clone(),
        })?;

        self.state.insert(canonical.clone(), LoadState::Loading);

        let lex_result = Lexer::new(&source).tokenize();
        if !lex_result.errors.is_empty() {
            self.state.remove(&canonical);
            return Err(RuntimeError {
                call_stack: Vec::new(),
                message: format!(
                    "lexer error in module '{}': {}",
                    module_name, lex_result.errors[0].message
                ),
                span: lex_result.errors[0].span.clone(),
            });
        }

        let parse_result = Parser::new(lex_result.tokens).parse();
        if !parse_result.errors.is_empty() {
            self.state.remove(&canonical);
            return Err(RuntimeError {
                call_stack: Vec::new(),
                message: format!(
                    "parse error in module '{}': {}",
                    module_name, parse_result.errors[0].message
                ),
                span: parse_result.errors[0].span.clone(),
            });
        }

        let mut functions = HashMap::new();
        for stmt in &parse_result.program.statements {
            if let Statement::Function(func) = stmt {
                functions.insert(func.name.clone(), func.clone());
            }
        }

        let module_dir = file_path.parent().unwrap_or(base_dir).to_path_buf();

        Ok(ParsedModule {
            name: module_name.to_string(),
            program: parse_result.program,
            functions,
            module_dir,
        })
    }
}
