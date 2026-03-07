use fragile_build::{BuildConfig, TargetConfig, TargetType};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CargoBuildError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    StaticLibrary,
    SharedLibrary,
    Executable,
}

#[derive(Debug, Clone)]
pub struct BuiltArtifact {
    pub name: String,
    pub kind: ArtifactKind,
    pub path: PathBuf,
}

#[derive(Debug, Error)]
pub enum CargoBuildError {
    #[error(transparent)]
    BuildConfig(#[from] fragile_build::BuildError),

    #[error("OUT_DIR was not provided by Cargo: {0}")]
    OutDirMissing(#[source] std::env::VarError),

    #[error("target `{0}` not found in fragile.toml")]
    TargetNotFound(String),

    #[error("dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("project root does not exist: {0}")]
    MissingProjectRoot(PathBuf),

    #[error("failed to create directory `{path}`: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("source file for target `{target}` does not exist: {path}")]
    MissingSource { target: String, path: PathBuf },

    #[error("fragile compile failed for target `{target}` source `{path}`: {message}")]
    CompileFailed {
        target: String,
        path: PathBuf,
        message: String,
    },

    #[error("failed to remove stale archive `{path}`: {source}")]
    RemoveArchive {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to spawn `ar` when creating `{archive}`: {source}")]
    ArchiveSpawn {
        archive: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "archive creation failed for `{archive}` (status {status})\nstdout:\n{stdout}\nstderr:\n{stderr}"
    )]
    ArchiveFailed {
        archive: PathBuf,
        status: i32,
        stdout: String,
        stderr: String,
    },

    #[error("internal dependency `{dep}` for target `{target}` was not built")]
    MissingInternalArtifact { target: String, dep: String },

    #[error(
        "target `{target}` depends on executable target `{dep}`; executables cannot be link deps"
    )]
    ExecutableDependency { target: String, dep: String },

    #[error("failed to remove stale linked artifact `{path}`: {source}")]
    RemoveLinkedArtifact {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("unable to locate `fragilec`; set FRAGILEC_BIN or ensure fragilec is on PATH")]
    FragilecMissing,

    #[error("failed to spawn `{binary}` while linking target `{target}`: {source}")]
    LinkSpawn {
        binary: PathBuf,
        target: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "link failed via `{binary}` for target `{target}` (status {status})\nstdout:\n{stdout}\nstderr:\n{stderr}"
    )]
    LinkFailed {
        binary: PathBuf,
        target: String,
        status: i32,
        stdout: String,
        stderr: String,
    },
}

#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub project_root: PathBuf,
    pub object_dir: PathBuf,
    pub artifact_dir: PathBuf,
    pub built_artifacts: Vec<BuiltArtifact>,
    pub root_artifact: BuiltArtifact,
    pub linked_external_libs: Vec<String>,
    pub linked_external_paths: Vec<PathBuf>,
}

/// Build a target (and its internal deps) from `fragile.toml`.
///
/// Supported target types:
/// - `static_library`
/// - `shared_library`
/// - `executable`
///
/// Intended usage from `build.rs`:
///
/// ```ignore
/// fn main() {
///     fragile_cargo::build_target("fragile.toml", "cppcore").unwrap();
/// }
/// ```
pub fn build_target(config_path: impl AsRef<Path>, target_name: &str) -> Result<BuildOutput> {
    let config_path = config_path.as_ref();
    println!("cargo:rerun-if-changed={}", config_path.display());

    let config = BuildConfig::from_file(config_path)?;
    let config_dir = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let project_root = resolve_project_root(&config, &config_dir)?;

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").map_err(CargoBuildError::OutDirMissing)?);
    let object_dir = out_dir.join("fragile-obj");
    let artifact_dir = out_dir.join("fragile-artifacts");
    create_dir_all(&object_dir)?;
    create_dir_all(&artifact_dir)?;

    let build_order = dependency_postorder(&config, target_name)?;

    let mut artifacts_by_name: HashMap<String, BuiltArtifact> = HashMap::new();
    let mut built_artifacts = Vec::new();

    for name in &build_order {
        let target = config
            .find_target(name)
            .ok_or_else(|| CargoBuildError::TargetNotFound(name.clone()))?;

        let target_object_dir = object_dir.join(&target.name);
        let objects = compile_target_sources(&config, &project_root, target, &target_object_dir)?;

        let artifact = match target.target_type {
            TargetType::StaticLibrary => {
                build_static_library_target(target, &project_root, &artifact_dir, &objects)?
            }
            TargetType::SharedLibrary => build_linked_target(
                &config,
                &project_root,
                target,
                &artifact_dir,
                &objects,
                &artifacts_by_name,
                LinkOutputKind::SharedLibrary,
            )?,
            TargetType::Executable => build_linked_target(
                &config,
                &project_root,
                target,
                &artifact_dir,
                &objects,
                &artifacts_by_name,
                LinkOutputKind::Executable,
            )?,
        };

        artifacts_by_name.insert(target.name.clone(), artifact.clone());
        built_artifacts.push(artifact);
    }

    let root_artifact = artifacts_by_name
        .get(target_name)
        .cloned()
        .ok_or_else(|| CargoBuildError::TargetNotFound(target_name.to_string()))?;

    emit_cargo_link_directives(
        &config,
        &project_root,
        target_name,
        &root_artifact,
        &artifacts_by_name,
        &artifact_dir,
    )?;

    let (linked_external_paths, linked_external_libs) =
        collect_external_link_inputs_for_target(&config, &project_root, target_name)?;

    Ok(BuildOutput {
        project_root,
        object_dir,
        artifact_dir,
        built_artifacts,
        root_artifact,
        linked_external_libs,
        linked_external_paths,
    })
}

fn compile_target_sources(
    config: &BuildConfig,
    project_root: &Path,
    target: &TargetConfig,
    object_dir: &Path,
) -> Result<Vec<PathBuf>> {
    create_dir_all(object_dir)?;

    let compile_flags = compile_flags(config, target, project_root);
    let compile_flag_refs: Vec<&str> = compile_flags.iter().map(String::as_str).collect();

    let mut objects = Vec::new();
    for (idx, source_token) in target.sources.iter().enumerate() {
        let source_path = resolve_project_path(source_token, project_root);
        if !source_path.is_file() {
            return Err(CargoBuildError::MissingSource {
                target: target.name.clone(),
                path: source_path,
            });
        }

        println!("cargo:rerun-if-changed={}", source_path.display());

        let out_obj = object_path(object_dir, idx, &source_path);
        if let Some(parent) = out_obj.parent() {
            create_dir_all(parent)?;
        }

        fragile_driver::compile_unit_with_fragilec_in_dir(
            &source_path,
            &out_obj,
            &compile_flag_refs,
            project_root,
        )
        .map_err(|message| CargoBuildError::CompileFailed {
            target: target.name.clone(),
            path: source_path.clone(),
            message,
        })?;

        objects.push(out_obj);
    }

    Ok(objects)
}

fn build_static_library_target(
    target: &TargetConfig,
    project_root: &Path,
    artifact_dir: &Path,
    objects: &[PathBuf],
) -> Result<BuiltArtifact> {
    create_dir_all(artifact_dir)?;

    let archive_path = artifact_path_for_target(artifact_dir, target);
    if archive_path.exists() {
        std::fs::remove_file(&archive_path).map_err(|source| CargoBuildError::RemoveArchive {
            path: archive_path.clone(),
            source,
        })?;
    }
    archive_objects(project_root, &archive_path, objects)?;

    Ok(BuiltArtifact {
        name: target.name.clone(),
        kind: ArtifactKind::StaticLibrary,
        path: archive_path,
    })
}

enum LinkOutputKind {
    SharedLibrary,
    Executable,
}

fn build_linked_target(
    config: &BuildConfig,
    project_root: &Path,
    target: &TargetConfig,
    artifact_dir: &Path,
    objects: &[PathBuf],
    artifacts_by_name: &HashMap<String, BuiltArtifact>,
    output_kind: LinkOutputKind,
) -> Result<BuiltArtifact> {
    create_dir_all(artifact_dir)?;

    let output_path = artifact_path_for_target(artifact_dir, target);
    if output_path.exists() {
        std::fs::remove_file(&output_path).map_err(|source| {
            CargoBuildError::RemoveLinkedArtifact {
                path: output_path.clone(),
                source,
            }
        })?;
    }

    let mut args: Vec<String> = Vec::new();

    for obj in objects {
        args.push(obj.display().to_string());
    }

    let mut dep_order = dependency_postorder(config, &target.name)?;
    dep_order.pop(); // remove self
    dep_order.reverse();

    for dep_name in &dep_order {
        let dep_artifact = artifacts_by_name.get(dep_name).ok_or_else(|| {
            CargoBuildError::MissingInternalArtifact {
                target: target.name.clone(),
                dep: dep_name.clone(),
            }
        })?;
        if dep_artifact.kind == ArtifactKind::Executable {
            return Err(CargoBuildError::ExecutableDependency {
                target: target.name.clone(),
                dep: dep_name.clone(),
            });
        }
        args.push(dep_artifact.path.display().to_string());
    }

    let (external_paths, external_libs) =
        collect_external_link_inputs_for_target(config, project_root, &target.name)?;

    for path in external_paths {
        args.push("-L".to_string());
        args.push(path.display().to_string());
    }
    for lib in external_libs {
        if lib.starts_with('-') {
            args.push(lib);
        } else {
            args.push(format!("-l{}", lib));
        }
    }

    if matches!(output_kind, LinkOutputKind::SharedLibrary) {
        args.push("-shared".to_string());
    }
    args.push("-o".to_string());
    args.push(output_path.display().to_string());

    let fragilec = find_fragilec_binary().ok_or(CargoBuildError::FragilecMissing)?;
    let mut cmd = Command::new(&fragilec);
    cmd.current_dir(project_root);
    if std::env::var_os("FRAGILEC_MODE").is_none() {
        cmd.env("FRAGILEC_MODE", "strict");
    }
    cmd.args(&args);

    let output = cmd.output().map_err(|source| CargoBuildError::LinkSpawn {
        binary: fragilec.clone(),
        target: target.name.clone(),
        source,
    })?;

    if !output.status.success() {
        return Err(CargoBuildError::LinkFailed {
            binary: fragilec,
            target: target.name.clone(),
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let kind = match output_kind {
        LinkOutputKind::SharedLibrary => ArtifactKind::SharedLibrary,
        LinkOutputKind::Executable => ArtifactKind::Executable,
    };

    Ok(BuiltArtifact {
        name: target.name.clone(),
        kind,
        path: output_path,
    })
}

fn emit_cargo_link_directives(
    config: &BuildConfig,
    project_root: &Path,
    root_target_name: &str,
    root_artifact: &BuiltArtifact,
    artifacts_by_name: &HashMap<String, BuiltArtifact>,
    artifact_dir: &Path,
) -> Result<()> {
    match root_artifact.kind {
        ArtifactKind::Executable => {
            println!(
                "cargo:warning=fragile-cargo built executable target `{}` at {}",
                root_target_name,
                root_artifact.path.display()
            );
            return Ok(());
        }
        ArtifactKind::StaticLibrary | ArtifactKind::SharedLibrary => {
            println!("cargo:rustc-link-search=native={}", artifact_dir.display());
        }
    }

    let mut dep_order = dependency_postorder(config, root_target_name)?;
    dep_order.pop();
    dep_order.reverse();

    match root_artifact.kind {
        ArtifactKind::StaticLibrary => {
            print_link_lib_for_artifact(root_artifact);
            for dep in &dep_order {
                let artifact = artifacts_by_name.get(dep).ok_or_else(|| {
                    CargoBuildError::MissingInternalArtifact {
                        target: root_target_name.to_string(),
                        dep: dep.clone(),
                    }
                })?;
                if artifact.kind == ArtifactKind::Executable {
                    return Err(CargoBuildError::ExecutableDependency {
                        target: root_target_name.to_string(),
                        dep: dep.clone(),
                    });
                }
                print_link_lib_for_artifact(artifact);
            }
        }
        ArtifactKind::SharedLibrary => {
            print_link_lib_for_artifact(root_artifact);
            for dep in &dep_order {
                let artifact = artifacts_by_name.get(dep).ok_or_else(|| {
                    CargoBuildError::MissingInternalArtifact {
                        target: root_target_name.to_string(),
                        dep: dep.clone(),
                    }
                })?;
                if artifact.kind == ArtifactKind::SharedLibrary {
                    print_link_lib_for_artifact(artifact);
                }
            }
        }
        ArtifactKind::Executable => {}
    }

    let (external_paths, external_libs) =
        collect_external_link_inputs_for_target(config, project_root, root_target_name)?;
    for path in &external_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for lib in &external_libs {
        println!("cargo:rustc-link-lib={}", lib);
    }

    Ok(())
}

fn print_link_lib_for_artifact(artifact: &BuiltArtifact) {
    match artifact.kind {
        ArtifactKind::StaticLibrary => {
            println!("cargo:rustc-link-lib=static={}", artifact.name);
        }
        ArtifactKind::SharedLibrary => {
            println!("cargo:rustc-link-lib=dylib={}", artifact.name);
        }
        ArtifactKind::Executable => {}
    }
}

fn collect_external_link_inputs_for_target(
    config: &BuildConfig,
    project_root: &Path,
    target_name: &str,
) -> Result<(Vec<PathBuf>, Vec<String>)> {
    let mut all_names = vec![target_name.to_string()];
    let mut deps = dependency_postorder(config, target_name)?;
    deps.pop();
    deps.reverse();
    all_names.extend(deps);

    let mut external_paths = Vec::new();
    let mut external_libs = Vec::new();

    for name in &all_names {
        let target = config
            .find_target(name)
            .ok_or_else(|| CargoBuildError::TargetNotFound(name.clone()))?;
        for raw_path in &target.lib_paths {
            let resolved = resolve_project_path(raw_path, project_root);
            push_unique_path(&mut external_paths, resolved);
        }
        for lib in &target.libs {
            let normalized = normalize_link_lib(lib);
            push_unique_string(&mut external_libs, normalized);
        }
    }

    Ok((external_paths, external_libs))
}

fn compile_flags(config: &BuildConfig, target: &TargetConfig, project_root: &Path) -> Vec<String> {
    let mut flags = Vec::new();
    if let Some(std_flag) = config.get_std(target) {
        flags.push(format!("-std={}", std_flag));
    }

    for include in config.get_includes(target) {
        let include_path = resolve_project_path(&include, project_root);
        flags.push(format!("-I{}", include_path.display()));
    }
    for define in config.get_defines(target) {
        flags.push(format!("-D{}", define));
    }

    for cflag in &config.compiler.cflags {
        flags.push(cflag.clone());
    }
    for cflag in &target.cflags {
        flags.push(cflag.clone());
    }

    flags
}

fn archive_objects(cwd: &Path, archive_path: &Path, objects: &[PathBuf]) -> Result<()> {
    let mut cmd = Command::new("ar");
    cmd.current_dir(cwd).arg("crs").arg(archive_path);
    for obj in objects {
        cmd.arg(obj);
    }

    let output = cmd
        .output()
        .map_err(|source| CargoBuildError::ArchiveSpawn {
            archive: archive_path.to_path_buf(),
            source,
        })?;
    if output.status.success() {
        return Ok(());
    }

    Err(CargoBuildError::ArchiveFailed {
        archive: archive_path.to_path_buf(),
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn resolve_project_root(config: &BuildConfig, config_dir: &Path) -> Result<PathBuf> {
    let root = match &config.project.root {
        Some(root) if root.is_absolute() => root.clone(),
        Some(root) => config_dir.join(root),
        None => config_dir.to_path_buf(),
    };
    if !root.exists() {
        return Err(CargoBuildError::MissingProjectRoot(root));
    }
    Ok(root.canonicalize().unwrap_or(root))
}

fn resolve_project_path(raw: &str, project_root: &Path) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn dependency_postorder(config: &BuildConfig, root_target: &str) -> Result<Vec<String>> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut active_stack: Vec<String> = Vec::new();
    let mut order = Vec::new();

    visit_target(
        config,
        root_target,
        &mut visited,
        &mut active_stack,
        &mut order,
    )?;
    Ok(order)
}

fn visit_target(
    config: &BuildConfig,
    name: &str,
    visited: &mut HashSet<String>,
    active_stack: &mut Vec<String>,
    order: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if let Some(position) = active_stack.iter().position(|n| n == name) {
        let mut cycle = active_stack[position..].join(" -> ");
        cycle.push_str(" -> ");
        cycle.push_str(name);
        return Err(CargoBuildError::DependencyCycle(cycle));
    }

    let target = config
        .find_target(name)
        .ok_or_else(|| CargoBuildError::TargetNotFound(name.to_string()))?;

    active_stack.push(name.to_string());
    for dep in &target.deps {
        visit_target(config, dep, visited, active_stack, order)?;
    }
    active_stack.pop();

    visited.insert(name.to_string());
    order.push(name.to_string());

    Ok(())
}

fn create_dir_all(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|source| CargoBuildError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

fn normalize_link_lib(lib: &str) -> String {
    if let Some(stripped) = lib.strip_prefix("-l") {
        stripped.to_string()
    } else {
        lib.to_string()
    }
}

fn object_path(object_dir: &Path, index: usize, source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unit")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    object_dir.join(format!("{index:04}_{stem}.o"))
}

fn artifact_path_for_target(artifact_dir: &Path, target: &TargetConfig) -> PathBuf {
    match target.target_type {
        TargetType::StaticLibrary => artifact_dir.join(format!("lib{}.a", target.name)),
        TargetType::SharedLibrary => artifact_dir.join(format!("lib{}.so", target.name)),
        TargetType::Executable => artifact_dir.join(target.name.clone()),
    }
}

fn push_unique_path(items: &mut Vec<PathBuf>, value: PathBuf) {
    if !items.iter().any(|existing| existing == &value) {
        items.push(value);
    }
}

fn push_unique_string(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|existing| existing == &value) {
        items.push(value);
    }
}

fn find_fragilec_binary() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("FRAGILEC_BIN") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if is_executable_candidate(&path) {
                return Some(path);
            }
            return None;
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().and_then(|p| p.parent());

    let mut candidates = vec![PathBuf::from("fragilec")];
    if let Some(repo_root) = repo_root {
        candidates.push(repo_root.join("target/release/fragilec"));
        candidates.push(repo_root.join("target/debug/fragilec"));
    }

    candidates
        .into_iter()
        .find(|candidate| is_executable_candidate(candidate))
}

fn is_executable_candidate(path: &Path) -> bool {
    if path == Path::new("fragilec") {
        return true;
    }
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn config_from_toml(toml: &str) -> BuildConfig {
        let mut file = NamedTempFile::new().expect("create temp fragile config");
        write!(file, "{}", toml).expect("write config");
        BuildConfig::from_file(file.path()).expect("parse config")
    }

    #[test]
    fn dependency_postorder_visits_dependencies_first() {
        let config = config_from_toml(
            r#"
[project]
name = "demo"

[[target]]
name = "c"
type = "static_library"
sources = []

[[target]]
name = "b"
type = "shared_library"
sources = []
deps = ["c"]

[[target]]
name = "a"
type = "executable"
sources = []
deps = ["b"]
"#,
        );
        let order = dependency_postorder(&config, "a").expect("order");
        assert_eq!(order, vec!["c", "b", "a"]);
    }

    #[test]
    fn dependency_postorder_rejects_cycles() {
        let config = config_from_toml(
            r#"
[project]
name = "demo"

[[target]]
name = "a"
type = "static_library"
sources = []
deps = ["b"]

[[target]]
name = "b"
type = "static_library"
sources = []
deps = ["a"]
"#,
        );
        let err = dependency_postorder(&config, "a").expect_err("should fail on cycle");
        let message = err.to_string();
        assert!(message.contains("dependency cycle"));
        assert!(message.contains("a -> b -> a"));
    }

    #[test]
    fn normalize_link_lib_strips_dash_l_prefix() {
        assert_eq!(normalize_link_lib("-lpthread"), "pthread");
        assert_eq!(normalize_link_lib("numa"), "numa");
    }

    #[test]
    fn artifact_path_names_match_target_type() {
        let static_target = TargetConfig {
            name: "core".to_string(),
            target_type: TargetType::StaticLibrary,
            sources: vec![],
            includes: vec![],
            defines: vec![],
            std: None,
            cflags: vec![],
            libs: vec![],
            lib_paths: vec![],
            deps: vec![],
            inherit_includes: true,
        };
        let shared_target = TargetConfig {
            name: "bridge".to_string(),
            target_type: TargetType::SharedLibrary,
            ..static_target.clone()
        };
        let exe_target = TargetConfig {
            name: "tool".to_string(),
            target_type: TargetType::Executable,
            ..static_target.clone()
        };

        let out = Path::new("/tmp/out");
        assert_eq!(
            artifact_path_for_target(out, &static_target),
            PathBuf::from("/tmp/out/libcore.a")
        );
        assert_eq!(
            artifact_path_for_target(out, &shared_target),
            PathBuf::from("/tmp/out/libbridge.so")
        );
        assert_eq!(
            artifact_path_for_target(out, &exe_target),
            PathBuf::from("/tmp/out/tool")
        );
    }
}
