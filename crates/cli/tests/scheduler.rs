#[cfg(unix)]
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
#[cfg(unix)]
use std::{path::Path, time::{Duration, Instant}};

#[cfg(unix)]
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[cfg(unix)]
struct Fixture {
    root: PathBuf,
}

#[cfg(unix)]
impl Fixture {
    fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "wportage-scheduler-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn manifest(&self, contents: &str) {
        fs::write(self.root.join("Wportage.toml"), contents).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wportage"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn path(&self, name: &str) -> String {
        self.root.join(name).to_string_lossy().into_owned()
    }
}

#[cfg(unix)]
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
#[test]
fn independent_targets_execute_concurrently() {
    let fixture = Fixture::new();
    let first = fixture.path("first.started");
    let second = fixture.path("second.started");
    let manifest = format!(
        r#"[project]
name = "parallel-fixture"
version = "0.1.0"
default_target = "app"

[target.first]
type = "exec"
program = "sh"
args = ["-c", "touch '{first}'; for i in $(seq 1 300); do test -f '{second}' && exit 0; sleep 0.01; done; exit 9"]

[target.second]
type = "exec"
program = "sh"
args = ["-c", "touch '{second}'; for i in $(seq 1 300); do test -f '{first}' && exit 0; sleep 0.01; done; exit 9"]

[target.app]
type = "exec"
deps = ["first", "second"]
program = "true"
"#,
    );
    fixture.manifest(&manifest);

    let output = fixture.run(&["build", "app", "-j", "2"]);
    assert!(
        output.status.success(),
        "parallel build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Path::new(&first).exists());
    assert!(Path::new(&second).exists());
}

#[cfg(unix)]
#[test]
fn dependent_target_runs_after_its_dependency() {
    let fixture = Fixture::new();
    let trace = fixture.path("order.log");
    let manifest = format!(
        r#"[project]
name = "order-fixture"
version = "0.1.0"
default_target = "app"

[target.prepare]
type = "exec"
program = "sh"
args = ["-c", "printf prepare >> '{trace}'"]

[target.compile]
type = "exec"
deps = ["prepare"]
program = "sh"
args = ["-c", "printf compile >> '{trace}'"]

[target.app]
type = "exec"
deps = ["compile"]
program = "sh"
args = ["-c", "printf app >> '{trace}'"]
"#,
    );
    fixture.manifest(&manifest);

    let output = fixture.run(&["build", "app", "-j", "3"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fs::read_to_string(trace).unwrap(), "preparecompileapp");
}

#[cfg(unix)]
#[test]
fn failed_target_cancels_a_running_peer() {
    let fixture = Fixture::new();
    fixture.manifest(
        r#"[project]
name = "cancel-fixture"
version = "0.1.0"
default_target = "app"

[target.slow]
type = "exec"
program = "sleep"
args = ["30"]

[target.fail]
type = "exec"
program = "sh"
args = ["-c", "sleep 0.15; exit 7"]

[target.app]
type = "exec"
deps = ["slow", "fail"]
program = "true"
"#,
    );

    let started = Instant::now();
    let output = fixture.run(&["build", "app", "-j", "2"]);
    assert!(!output.status.success());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "peer process was not cancelled promptly"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("failed"),
        "unexpected diagnostic: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn build_rejects_a_declared_output_that_was_not_created() {
    let fixture = Fixture::new();
    fixture.manifest(
        r#"[project]
name = "output-fixture"
version = "0.1.0"
default_target = "app"

[target.app]
type = "exec"
program = "true"
outputs = ["build/result.bin"]
"#,
    );

    let output = fixture.run(&["build", "app", "-j", "1"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("was not produced"),
        "unexpected diagnostic: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
