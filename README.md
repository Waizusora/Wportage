# Wportage

**Wicofe Portage** is a Rust-native build and package infrastructure project for the Wicofe ecosystem.

Wportage is designed around a small, memorable CLI and an independent build engine.

## Status

Early development. The current engine validates target dependency graphs, executes typed process actions without a shell, retains shell commands as an explicit escape hatch, and schedules independent targets concurrently. Toolchains, incremental caching, package resolution, sandboxing, and remote execution remain planned work.

## Commands

```sh
wportage init [path]
wportage build [target] [-j JOBS]
wportage test [-j JOBS]
wportage clean
wportage graph
wportage doctor
```

The default job limit is the host's available parallelism. Set `-j 1` for serial execution.

## Manifest

- `Wportage.toml` — project and target definitions.
- `Wportage.lock` — reserved for deterministic dependency locking as package resolution is implemented.
- `target/` — generated build outputs.

### Typed executable target

Prefer `type = "exec"` for process actions. Wportage invokes the program directly and does not pass arguments through a shell.

```toml
[target.compile]
type = "exec"
program = "clang"
args = ["-c", "src/main.c", "-o", "target/main.o"]
env = { LC_ALL = "C" }
deps = ["generate"]

[target.generate]
type = "exec"
program = "python3"
args = ["tools/generate.py"]
```

### Shell command target

`type = "command"` remains available as an escape hatch for scripts and compound shell expressions.

```toml
[target.custom]
type = "command"
commands = ["mkdir -p target && echo ready"]
```

## Quick start

```sh
cargo install --path crates/cli
wportage init hello
cd hello
wportage build
```

## Execution model

Wportage validates the reachable dependency graph before starting actions. Ready targets are selected deterministically and run concurrently up to the configured job limit. A failed target stops further scheduling; already-running peers in the same batch are joined before the failure is returned. This release does not yet provide incremental rebuilds, artifact caching, process-tree cancellation, or hermetic execution.

## License

Apache-2.0. See [LICENSE](LICENSE).
