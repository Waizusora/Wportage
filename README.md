# Wportage

**Wicofe Portage** is a native Rust build and package infrastructure project for the Wicofe ecosystem.

Wportage is designed around a small, memorable CLI and an independent build engine. It does not require GNU Make, Ninja, or CMake to execute native Wportage build tasks.

## Status

Early development. The initial milestone establishes the Rust workspace, CLI, manifest format, and CI. Package registry, dependency resolution, sandboxing, remote execution, and advanced caching are planned follow-up milestones.

## Commands

```sh
wportage init [path]
wportage build [target] [-j JOBS]
wportage test
wportage clean
wportage graph
wportage doctor
```

## Project files

- `Wportage.toml` — project and target definitions.
- `Wportage.lock` — reserved for deterministic dependency locking as package resolution is implemented.
- `target/` — generated build outputs.

## Quick start

```sh
cargo install --path crates/cli
wportage init hello
cd hello
wportage build
```

The generated example is a command-based target, so the compiler or toolchain is selected by the project author. Wportage schedules and executes declared commands itself.

## License

Apache-2.0. See [LICENSE](LICENSE).
