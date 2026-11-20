use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(name = "wportage", version, about = "Wicofe Portage — native build and package infrastructure")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a starter Wportage project.
    Init { path: Option<PathBuf> },
    /// Build a target (defaults to the manifest's default target).
    Build {
        target: Option<String>,
        /// Maximum number of independent targets to execute concurrently.
        #[arg(short = 'j', long = "jobs", default_value_t = default_jobs())]
        jobs: usize,
    },
    /// Run the manifest's test target, or all targets marked as tests.
    Test {
        /// Maximum number of independent targets to execute concurrently.
        #[arg(short = 'j', long = "jobs", default_value_t = default_jobs())]
        jobs: usize,
    },
    /// Remove generated build artifacts.
    Clean,
    /// Print the target dependency graph.
    Graph,
    /// Check the project manifest and required host tools.
    Doctor,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    project: Project,
    #[serde(default)]
    build: BuildConfig,
    #[serde(default)]
    target: BTreeMap<String, Target>,
}

#[derive(Debug, Deserialize)]
struct Project {
    name: String,
    version: String,
    #[serde(default = "default_target")]
    default_target: String,
}

#[derive(Debug, Deserialize)]
struct BuildConfig {
    #[serde(default = "default_out")]
    out_dir: String,
}

impl Default for BuildConfig {
    fn default() -> Self { Self { out_dir: default_out() } }
}

#[derive(Debug, Deserialize)]
struct Target {
    #[serde(rename = "type", default = "default_type")]
    kind: String,
    #[serde(default)]
    deps: Vec<String>,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    test: bool,
}

fn default_target() -> String { "default".into() }
fn default_out() -> String { "target/wportage".into() }
fn default_type() -> String { "command".into() }
fn default_jobs() -> usize {
    std::thread::available_parallelism().map(usize::from).unwrap_or(1)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("wportage: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path } => init(path.unwrap_or_else(|| PathBuf::from("."))),
        Commands::Build { target, jobs } => {
            let (manifest, root) = load_manifest()?;
            let selected = target.unwrap_or_else(|| manifest.project.default_target.clone());
            build(&manifest, &root, &selected, jobs)
        }
        Commands::Test { jobs } => {
            let (manifest, root) = load_manifest()?;
            let tests: Vec<String> = manifest.target.iter().filter(|(_, t)| t.test).map(|(n, _)| n.clone()).collect();
            if tests.is_empty() {
                if manifest.target.contains_key("test") { build(&manifest, &root, "test", jobs) }
                else { bail!("no test targets found; mark a target with test = true") }
            } else {
                for name in tests { build(&manifest, &root, &name, jobs)?; }
                Ok(())
            }
        }
        Commands::Clean => {
            let (manifest, root) = load_manifest()?;
            let out = root.join(&manifest.build.out_dir);
            if out.exists() { fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?; }
            println!("cleaned {}", out.display());
            Ok(())
        }
        Commands::Graph => {
            let (manifest, _) = load_manifest()?;
            for (name, target) in &manifest.target {
                if target.deps.is_empty() { println!("{name}"); }
                else { println!("{name} -> {}", target.deps.join(", ")); }
            }
            Ok(())
        }
        Commands::Doctor => doctor(),
    }
}

fn init(path: PathBuf) -> Result<()> {
    fs::create_dir_all(&path)?;
    let manifest = path.join("Wportage.toml");
    if manifest.exists() { bail!("{} already exists", manifest.display()); }
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("wportage-project");
    let content = format!(r#"[project]
name = "{name}"
version = "0.1.0"
default_target = "hello"

[build]
out_dir = "target/wportage"

[target.prepare]
type = "exec"
program = "echo"
args = ["preparing project"]

[target.hello]
type = "exec"
deps = ["prepare"]
program = "echo"
args = ["Wportage build is working"]

[target.test]
type = "exec"
test = true
program = "echo"
args = ["Wportage test target is working"]
"#);
    fs::write(&manifest, content)?;
    println!("created {}", manifest.display());
    Ok(())
}

fn load_manifest() -> Result<(Manifest, PathBuf)> {
    let root = std::env::current_dir()?;
    let path = root.join("Wportage.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let manifest: Manifest = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if manifest.project.name.trim().is_empty() || manifest.project.version.trim().is_empty() {
        bail!("project name and version must not be empty");
    }
    if manifest.build.out_dir.trim().is_empty() {
        bail!("build.out_dir must not be empty");
    }
    Ok((manifest, root))
}

fn build(manifest: &Manifest, root: &Path, selected: &str, jobs: usize) -> Result<()> {
    if jobs == 0 { bail!("jobs must be greater than zero"); }
    let mut ordered = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    visit(selected, manifest, &mut visiting, &mut visited, &mut ordered)?;

    let mut declared_outputs: Vec<(String, PathBuf)> = Vec::new();
    for name in &ordered {
        let target = &manifest.target[name];
        validate_action(name, target)?;
        for output in &target.outputs {
            let path = validate_output_path(name, output)?;
            for (owner, existing) in &declared_outputs {
                if path == *existing || path.starts_with(existing) || existing.starts_with(&path) {
                    bail!("output conflict: target '{name}' declares '{output}', overlapping output of target '{owner}' ({} )", existing.display());
                }
            }
            declared_outputs.push((name.clone(), path));
        }
    }

    let mut completed = BTreeSet::new();
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut pending = ordered;
    while !pending.is_empty() {
        let ready: Vec<String> = pending.iter()
            .filter(|name| manifest.target[*name].deps.iter().all(|dep| completed.contains(dep)))
            .take(jobs)
            .cloned()
            .collect();
        if ready.is_empty() {
            bail!("build graph cannot make progress; check dependency relationships");
        }

        println!("==> scheduling {}", ready.join(", "));
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = ready.iter().map(|name| {
                let name = name.clone();
                let cancellation = Arc::clone(&cancellation);
                scope.spawn(move || {
                    let result = execute_target(&name, &manifest.target[&name], root, &cancellation);
                    if result.is_err() {
                        cancellation.store(true, Ordering::SeqCst);
                    }
                    (name, result)
                })
            }).collect();

            handles.into_iter().map(|handle| {
                handle.join().unwrap_or_else(|_| {
                    ("<worker>".to_string(), Err(anyhow!("target worker panicked")))
                })
            }).collect::<Vec<_>>()
        });

        for (name, result) in results {
            result.with_context(|| format!("target '{name}' failed"))?;
            completed.insert(name);
        }
        let finished: BTreeSet<String> = ready.into_iter().collect();
        pending.retain(|name| !finished.contains(name));
    }
    Ok(())
}

fn validate_output_path(target: &str, output: &str) -> Result<PathBuf> {
    let path = Path::new(output);
    if output.trim().is_empty() || path.is_absolute() || path.components().any(|component| {
        matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))
    }) {
        bail!("target '{target}' declares unsafe output path '{output}'; outputs must stay inside the project directory");
    }
    Ok(path.to_path_buf())
}

fn validate_action(name: &str, target: &Target) -> Result<()> {
    match target.kind.as_str() {
        "command" => {
            if target.commands.is_empty() {
                bail!("target '{name}' has type = \"command\" but no commands");
            }
            if target.program.is_some() || !target.args.is_empty() {
                bail!("target '{name}' mixes command and exec action fields");
            }
        }
        "exec" => {
            let program = target.program.as_deref().unwrap_or_default();
            if program.trim().is_empty() {
                bail!("target '{name}' has type = \"exec\" but no program");
            }
            if !target.commands.is_empty() {
                bail!("target '{name}' mixes command and exec action fields");
            }
        }
        other => bail!("unsupported target type for {name}: {other}; supported types are command and exec"),
    }
    for output in &target.outputs {
        validate_output_path(name, output)?;
    }
    Ok(())
}

fn execute_target(name: &str, target: &Target, root: &Path, cancellation: &AtomicBool) -> Result<()> {
    println!("==> {name}");
    match target.kind.as_str() {
        "command" => {
            for command in &target.commands {
                let mut process = if cfg!(target_os = "windows") {
                    Command::new("cmd").args(["/C", command]).current_dir(root).stdin(Stdio::null()).spawn()?
                } else {
                    Command::new("sh").args(["-c", command]).current_dir(root).stdin(Stdio::null()).spawn()?
                };
                wait_for_process(&mut process, target.timeout_ms, cancellation)
                    .with_context(|| format!("command failed: {command}"))?;
            }
        }
        "exec" => {
            let program = target.program.as_deref().context("missing executable program")?;
            let mut process = Command::new(program)
                .args(&target.args)
                .envs(&target.env)
                .current_dir(root)
                .stdin(Stdio::null())
                .spawn()
                .with_context(|| format!("starting executable '{program}'"))?;
            wait_for_process(&mut process, target.timeout_ms, cancellation)
                .with_context(|| format!("executable '{program}' failed"))?;
        }
        _ => unreachable!("actions are validated before execution"),
    }
    for output in &target.outputs {
        let path = root.join(output);
        if !path.exists() {
            bail!("declared output '{output}' was not produced");
        }
    }
    Ok(())
}

fn wait_for_process(child: &mut Child, timeout_ms: Option<u64>, cancellation: &AtomicBool) -> Result<()> {
    let started = Instant::now();
    loop {
        if cancellation.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("process cancelled because another build action failed");
        }
        if let Some(status) = child.try_wait()? {
            if !status.success() { bail!("process exited with status {status}"); }
            return Ok(());
        }
        if timeout_ms.is_some_and(|limit| started.elapsed() >= Duration::from_millis(limit)) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("process exceeded timeout of {} ms", timeout_ms.unwrap());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn visit(
    name: &str,
    manifest: &Manifest,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(name) { return Ok(()); }
    if !visiting.insert(name.to_owned()) { bail!("dependency cycle detected at target '{name}'"); }
    let target = manifest.target.get(name).with_context(|| format!("unknown target or dependency '{name}'"))?;
    for dep in &target.deps { visit(dep, manifest, visiting, visited, ordered)?; }
    visiting.remove(name);
    visited.insert(name.to_owned());
    ordered.push(name.to_owned());
    Ok(())
}

fn doctor() -> Result<()> {
    let (manifest, _) = load_manifest()?;
    println!("Wportage project: {} {}", manifest.project.name, manifest.project.version);
    println!("targets: {}", manifest.target.len());
    println!("host: {}-{}", std::env::consts::ARCH, std::env::consts::OS);
    println!("parallel jobs: {}", default_jobs());
    println!("manifest: OK");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(deps: &[&str]) -> Target {
        Target {
            kind: "exec".into(),
            deps: deps.iter().map(|value| (*value).to_string()).collect(),
            commands: Vec::new(),
            program: Some("true".into()),
            args: Vec::new(),
            env: BTreeMap::new(),
            outputs: Vec::new(),
            timeout_ms: None,
            test: false,
        }
    }

    fn manifest(targets: &[(&str, &[&str])]) -> Manifest {
        Manifest {
            project: Project {
                name: "fixture".into(),
                version: "0.1.0".into(),
                default_target: "app".into(),
            },
            build: BuildConfig::default(),
            target: targets.iter().map(|(name, deps)| ((*name).to_string(), target(deps))).collect(),
        }
    }

    #[test]
    fn visits_dependencies_before_dependents() {
        let manifest = manifest(&[("app", &["lib"]), ("lib", &["base"]), ("base", &[])]);
        let mut ordered = Vec::new();
        visit("app", &manifest, &mut BTreeSet::new(), &mut BTreeSet::new(), &mut ordered).unwrap();
        assert_eq!(ordered, ["base", "lib", "app"]);
    }

    #[test]
    fn rejects_dependency_cycles() {
        let manifest = manifest(&[("app", &["lib"]), ("lib", &["app"])]);
        let error = visit("app", &manifest, &mut BTreeSet::new(), &mut BTreeSet::new(), &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("cycle"));
    }

    #[test]
    fn rejects_unknown_dependencies() {
        let manifest = manifest(&[("app", &["missing"])]);
        let error = visit("app", &manifest, &mut BTreeSet::new(), &mut BTreeSet::new(), &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("unknown"));
    }

    #[test]
    fn rejects_zero_jobs() {
        let manifest = manifest(&[("app", &[])]);
        let error = build(&manifest, Path::new("."), "app", 0).unwrap_err();
        assert!(error.to_string().contains("jobs"));
    }

    #[test]
    fn default_build_directory_is_defined() {
        assert_eq!(BuildConfig::default().out_dir, "target/wportage");
    }

    #[test]
    fn exec_action_requires_a_program() {
        let mut action = target(&[]);
        action.program = None;
        let error = validate_action("broken", &action).unwrap_err();
        assert!(error.to_string().contains("no program"));
    }

    #[test]
    fn rejects_absolute_and_parent_output_paths() {
        assert!(validate_output_path("compile", "/tmp/out.o").is_err());
        assert!(validate_output_path("compile", "../out.o").is_err());
    }

    #[test]
    fn accepts_workspace_relative_output_paths() {
        assert_eq!(validate_output_path("compile", "obj/main.o").unwrap(), PathBuf::from("obj/main.o"));
    }

    #[cfg(unix)]
    #[test]
    fn process_timeout_kills_child() {
        let mut child = Command::new("sleep").arg("2").spawn().unwrap();
        let error = wait_for_process(&mut child, Some(10), &AtomicBool::new(false)).unwrap_err();
        assert!(error.to_string().contains("timeout"));
        assert!(child.try_wait().unwrap().is_some());
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_stops_a_running_child() {
        let mut child = Command::new("sleep").arg("2").spawn().unwrap();
        let cancellation = AtomicBool::new(true);
        let error = wait_for_process(&mut child, None, &cancellation).unwrap_err();
        assert!(error.to_string().contains("cancelled"));
        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn shell_action_requires_commands() {
        let action = Target {
            kind: "command".into(),
            deps: Vec::new(),
            commands: Vec::new(),
            program: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            outputs: Vec::new(),
            timeout_ms: None,
            test: false,
        };
        let error = validate_action("broken", &action).unwrap_err();
        assert!(error.to_string().contains("no commands"));
    }
}
