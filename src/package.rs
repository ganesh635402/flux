// Flux package management — project structure, manifests, dependencies.

use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// The Flux project manifest filename.
pub const MANIFEST_NAME: &str = "flux.toml";

/// The Flux lock filename.
pub const LOCK_NAME: &str = "flux.lock";

/// A parsed Flux project manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct FluxManifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
}

/// Package identity and metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// A dependency specification.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// A simple version string: `"1.0.0"`
    Version(String),
    /// A path dependency: `{ path = "../utils" }`
    Detailed(DetailedDep),
}

/// A detailed dependency with path and optional version.
#[derive(Debug, Clone, Deserialize)]
pub struct DetailedDep {
    pub path: Option<String>,
    pub version: Option<String>,
}

/// A semantic version (simplified).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    pub fn parse(s: &str) -> Result<SemVer, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "invalid version '{}': expected MAJOR.MINOR.PATCH",
                s
            ));
        }
        let major = parts[0]
            .parse()
            .map_err(|_| format!("invalid major version in '{}'", s))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| format!("invalid minor version in '{}'", s))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| format!("invalid patch version in '{}'", s))?;
        Ok(SemVer {
            major,
            minor,
            patch,
        })
    }

    /// Check if this version satisfies a version requirement string.
    pub fn satisfies(&self, requirement: &str) -> bool {
        let req = requirement.trim();
        if req.starts_with('^') {
            // Caret: compatible with major version
            if let Ok(base) = SemVer::parse(&req[1..]) {
                return self.major == base.major
                    && (self.minor > base.minor
                        || (self.minor == base.minor && self.patch >= base.patch));
            }
            false
        } else if req.starts_with(">=") {
            if let Ok(base) = SemVer::parse(req.trim_start_matches(">=").trim()) {
                return *self >= base;
            }
            false
        } else {
            // Exact match
            if let Ok(base) = SemVer::parse(req) {
                return *self == base;
            }
            false
        }
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A resolved dependency in the dependency graph.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: SemVer,
    pub path: PathBuf,
    pub manifest: FluxManifest,
}

/// A dependency graph for cycle detection.
#[derive(Debug, Default)]
pub struct DepGraph {
    edges: HashMap<String, Vec<String>>,
}

impl DepGraph {
    pub fn new() -> Self {
        DepGraph {
            edges: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    /// Detect cycles in the dependency graph. Returns the cycle chain if found.
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashMap::new();
        for node in self.edges.keys() {
            if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut vec![]) {
                return Some(cycle);
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashMap<String, bool>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if let Some(&in_stack) = visited.get(node) {
            if in_stack {
                // Found cycle
                let start = path.iter().position(|n| n == node).unwrap();
                let mut cycle = path[start..].to_vec();
                cycle.push(node.to_string());
                return Some(cycle);
            }
            return None; // Already fully visited
        }
        visited.insert(node.to_string(), true);
        path.push(node.to_string());
        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if let Some(cycle) = self.dfs_cycle(dep, visited, path) {
                    return Some(cycle);
                }
            }
        }
        path.pop();
        visited.insert(node.to_string(), false);
        None
    }
}

/// Errors from package operations.
#[derive(Debug)]
pub enum PackageError {
    ManifestNotFound(PathBuf),
    ManifestReadError(PathBuf, String),
    ManifestParseError(PathBuf, String),
    InvalidVersion(String),
    DependencyNotFound(String, PathBuf),
    DependencyManifestError(String, String),
    CircularDependency(Vec<String>),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageError::ManifestNotFound(path) => {
                write!(f, "manifest not found: {}", path.display())
            }
            PackageError::ManifestReadError(path, err) => {
                write!(f, "failed to read manifest {}: {}", path.display(), err)
            }
            PackageError::ManifestParseError(path, err) => {
                write!(f, "invalid manifest {}: {}", path.display(), err)
            }
            PackageError::InvalidVersion(v) => write!(f, "invalid version: {}", v),
            PackageError::DependencyNotFound(name, path) => {
                write!(f, "dependency '{}' not found at {}", name, path.display())
            }
            PackageError::DependencyManifestError(name, err) => {
                write!(f, "dependency '{}': {}", name, err)
            }
            PackageError::CircularDependency(chain) => {
                write!(f, "circular dependency detected: {}", chain.join(" -> "))
            }
        }
    }
}

/// Load and parse a flux.toml manifest from a directory.
pub fn load_manifest(dir: &Path) -> Result<FluxManifest, PackageError> {
    let manifest_path = dir.join(MANIFEST_NAME);
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(manifest_path));
    }
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| PackageError::ManifestReadError(manifest_path.clone(), e.to_string()))?;
    let manifest: FluxManifest = toml::from_str(&content)
        .map_err(|e| PackageError::ManifestParseError(manifest_path, e.to_string()))?;

    // Validate version
    SemVer::parse(&manifest.package.version).map_err(|e| PackageError::InvalidVersion(e))?;

    Ok(manifest)
}

/// Find the project root by searching upward for flux.toml.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(MANIFEST_NAME).exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Resolve all local/path dependencies for a project.
pub fn resolve_dependencies(
    project_dir: &Path,
    manifest: &FluxManifest,
) -> Result<(Vec<ResolvedDep>, DepGraph), PackageError> {
    let mut resolved = Vec::new();
    let mut graph = DepGraph::new();
    let pkg_name = &manifest.package.name;

    for (dep_name, dep_spec) in &manifest.dependencies {
        graph.add_edge(pkg_name, dep_name);

        let dep_path = match dep_spec {
            DependencySpec::Detailed(d) => {
                if let Some(ref p) = d.path {
                    project_dir.join(p)
                } else {
                    return Err(PackageError::DependencyNotFound(
                        dep_name.clone(),
                        project_dir.to_path_buf(),
                    ));
                }
            }
            DependencySpec::Version(_) => {
                // Version-only deps require a registry — not yet supported
                return Err(PackageError::DependencyNotFound(
                    dep_name.clone(),
                    project_dir.to_path_buf(),
                ));
            }
        };

        if !dep_path.exists() {
            return Err(PackageError::DependencyNotFound(dep_name.clone(), dep_path));
        }

        let dep_manifest = load_manifest(&dep_path).map_err(|e| {
            PackageError::DependencyManifestError(dep_name.clone(), format!("{}", e))
        })?;

        // Check version if specified
        if let DependencySpec::Detailed(d) = dep_spec {
            if let Some(ref req) = d.version {
                let dep_ver = SemVer::parse(&dep_manifest.package.version)
                    .map_err(|e| PackageError::InvalidVersion(e))?;
                if !dep_ver.satisfies(req) {
                    return Err(PackageError::DependencyManifestError(
                        dep_name.clone(),
                        format!("version {} does not satisfy requirement {}", dep_ver, req),
                    ));
                }
            }
        }

        // Recursively resolve transitive dependencies
        for (trans_name, _) in &dep_manifest.dependencies {
            graph.add_edge(dep_name, trans_name);
        }

        resolved.push(ResolvedDep {
            name: dep_name.clone(),
            version: SemVer::parse(&dep_manifest.package.version).unwrap(),
            path: dep_path,
            manifest: dep_manifest,
        });
    }

    // Check for cycles
    if let Some(cycle) = graph.find_cycle() {
        return Err(PackageError::CircularDependency(cycle));
    }

    Ok((resolved, graph))
}

/// Initialize a new Flux project in a directory.
pub fn init_project(dir: &Path, name: &str) -> Result<(), PackageError> {
    let manifest_path = dir.join(MANIFEST_NAME);
    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[dependencies]
"#,
        name
    );
    fs::write(&manifest_path, content)
        .map_err(|e| PackageError::ManifestReadError(manifest_path, e.to_string()))?;

    // Create src directory and main.flux
    let src_dir = dir.join("src");
    let _ = fs::create_dir_all(&src_dir);
    let main_path = src_dir.join("main.flux");
    if !main_path.exists() {
        let _ = fs::write(&main_path, "print(\"Hello, Flux!\")\n");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn semver_parse_valid() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn semver_parse_invalid() {
        assert!(SemVer::parse("1.2").is_err());
        assert!(SemVer::parse("abc").is_err());
    }

    #[test]
    fn semver_display() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(format!("{}", v), "1.2.3");
    }

    #[test]
    fn semver_satisfies_exact() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert!(v.satisfies("1.2.3"));
        assert!(!v.satisfies("1.2.4"));
    }

    #[test]
    fn semver_satisfies_caret() {
        let v = SemVer::parse("1.3.0").unwrap();
        assert!(v.satisfies("^1.2.0"));
        assert!(!v.satisfies("^2.0.0"));
    }

    #[test]
    fn semver_satisfies_gte() {
        let v = SemVer::parse("1.5.0").unwrap();
        assert!(v.satisfies(">=1.0.0"));
        assert!(!v.satisfies(">=2.0.0"));
    }

    #[test]
    fn semver_ordering() {
        let v1 = SemVer::parse("1.0.0").unwrap();
        let v2 = SemVer::parse("1.1.0").unwrap();
        let v3 = SemVer::parse("2.0.0").unwrap();
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn dep_graph_no_cycle() {
        let mut g = DepGraph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        assert!(g.find_cycle().is_none());
    }

    #[test]
    fn dep_graph_cycle() {
        let mut g = DepGraph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        g.add_edge("C", "A");
        let cycle = g.find_cycle().unwrap();
        assert!(cycle.contains(&"A".to_string()));
    }

    #[test]
    fn manifest_load_missing() {
        let tmp = std::env::temp_dir().join("flux_test_missing_manifest");
        let _ = fs::create_dir_all(&tmp);
        let result = load_manifest(&tmp);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn manifest_load_valid() {
        let tmp = std::env::temp_dir().join("flux_test_valid_manifest");
        let _ = fs::create_dir_all(&tmp);
        fs::write(
            tmp.join("flux.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
        )
        .unwrap();
        let manifest = load_manifest(&tmp).unwrap();
        assert_eq!(manifest.package.name, "test");
        assert_eq!(manifest.package.version, "0.1.0");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn manifest_load_invalid_toml() {
        let tmp = std::env::temp_dir().join("flux_test_invalid_toml");
        let _ = fs::create_dir_all(&tmp);
        fs::write(tmp.join("flux.toml"), "not valid toml {{{{").unwrap();
        let result = load_manifest(&tmp);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn init_project_creates_files() {
        let tmp = std::env::temp_dir().join("flux_test_init_project");
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);
        init_project(&tmp, "myproject").unwrap();
        assert!(tmp.join("flux.toml").exists());
        assert!(tmp.join("src").join("main.flux").exists());
        let manifest = load_manifest(&tmp).unwrap();
        assert_eq!(manifest.package.name, "myproject");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_project_root_found() {
        let tmp = std::env::temp_dir().join("flux_test_find_root");
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(tmp.join("src"));
        fs::write(
            tmp.join("flux.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
        )
        .unwrap();
        let root = find_project_root(&tmp.join("src"));
        assert!(root.is_some());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_path_dependency() {
        let tmp = std::env::temp_dir().join("flux_test_resolve_dep");
        let _ = fs::remove_dir_all(&tmp);
        let main_dir = tmp.join("main_pkg");
        let dep_dir = tmp.join("dep_pkg");
        let _ = fs::create_dir_all(&main_dir);
        let _ = fs::create_dir_all(&dep_dir);

        fs::write(
            main_dir.join("flux.toml"),
            "[package]\nname = \"main\"\nversion = \"0.1.0\"\n\n[dependencies]\nmylib = { path = \"../dep_pkg\" }\n",
        )
        .unwrap();
        fs::write(
            dep_dir.join("flux.toml"),
            "[package]\nname = \"mylib\"\nversion = \"0.2.0\"\n\n[dependencies]\n",
        )
        .unwrap();

        let manifest = load_manifest(&main_dir).unwrap();
        let (deps, _graph) = resolve_dependencies(&main_dir, &manifest).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "mylib");
        assert_eq!(deps[0].version, SemVer::parse("0.2.0").unwrap());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_circular_dependency() {
        let tmp = std::env::temp_dir().join("flux_test_circular_dep");
        let _ = fs::remove_dir_all(&tmp);
        let pkg_a = tmp.join("pkg_a");
        let pkg_b = tmp.join("pkg_b");
        let _ = fs::create_dir_all(&pkg_a);
        let _ = fs::create_dir_all(&pkg_b);

        fs::write(
            pkg_a.join("flux.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nb = { path = \"../pkg_b\" }\n",
        )
        .unwrap();
        fs::write(
            pkg_b.join("flux.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n[dependencies]\na = { path = \"../pkg_a\" }\n",
        )
        .unwrap();

        let manifest = load_manifest(&pkg_a).unwrap();
        let result = resolve_dependencies(&pkg_a, &manifest);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_missing_dependency() {
        let tmp = std::env::temp_dir().join("flux_test_missing_dep");
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);

        fs::write(
            tmp.join("flux.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = { path = \"../nonexistent\" }\n",
        )
        .unwrap();

        let manifest = load_manifest(&tmp).unwrap();
        let result = resolve_dependencies(&tmp, &manifest);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&tmp);
    }
}
