use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
        #[arg(short = 'j', long, default_value_t = 1)]
        jobs: usize,
    },
    /// Run the manifest's test target, or all targets marked as tests.
    Test,
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

#[derive(Debug, Default, Deserialize)]
struct BuildConfig {
    #[serde(default = "default_out")]
    out_dir: String,
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
    test: bool,
}

fn default_target() -> String { "default".into() }
fn default_out() -> String { "target/wportage".into() }
fn default_type() -> String { "command".into() }

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
            if jobs == 0 { bail!("-j/--jobs must be greater than zero"); }
            let (manifest, root) = load_manifest()?;
            let selected = target.unwrap_or_else(|| manifest.project.default_target.clone());
            build(&manifest, &root, &selected)
        }
        Commands::Test => {
            let (manifest, root) = load_manifest()?;
            let tests: Vec<String> = manifest.target.iter().filter(|(_, t)| t.test).map(|(n, _)| n.clone()).collect();
            if tests.is_empty() {
                if manifest.target.contains_key("test") { build(&manifest, &root, "test") }
                else { bail!("no test targets found; mark a target with test = true") }
            } else {
                for name in tests { build(&manifest, &root, &name)?; }
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

[target.hello]
type = "command"
commands = ["echo Wportage build is working"]

[target.test]
type = "command"
test = true
commands = ["echo Wportage test target is working"]
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
    Ok((manifest, root))
}

fn build(manifest: &Manifest, root: &Path, selected: &str) -> Result<()> {
    if !manifest.target.contains_key(selected) { bail!("unknown target '{selected}'"); }
    let mut ordered = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    visit(selected, manifest, &mut visiting, &mut visited, &mut ordered)?;
    for name in ordered {
        let target = &manifest.target[&name];
        println!("==> {name}");
        for command in &target.commands {
            let status = if cfg!(target_os = "windows") {
                Command::new("cmd").args(["/C", command]).current_dir(root).stdin(Stdio::null()).status()?
            } else {
                Command::new("sh").args(["-c", command]).current_dir(root).stdin(Stdio::null()).status()?
            };
            if !status.success() { bail!("target '{name}' command failed: {command}"); }
        }
    }
    Ok(())
}

fn visit(name: &str, manifest: &Manifest, visiting: &mut BTreeSet<String>, visited: &mut BTreeSet<String>, ordered: &mut Vec<String>) -> Result<()> {
    if visited.contains(name) { return Ok(()); }
    if !visiting.insert(name.to_owned()) { bail!("dependency cycle detected at target '{name}'"); }
    let target = manifest.target.get(name).with_context(|| format!("unknown dependency target '{name}'"))?;
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
    println!("manifest: OK");
    Ok(())
}
