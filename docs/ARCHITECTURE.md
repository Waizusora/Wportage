# Wportage Architecture

Wportage is a Rust-native build and package infrastructure. The user interface stays small while the engine is divided into explicit subsystems.

## Design rules

1. The native Wportage engine must not require another build engine.
2. Build descriptions are declarative and versioned.
3. The reachable build graph is validated before actions execute.
4. Ready actions are scheduled deterministically within a bounded concurrency limit.
5. Cache entries will be keyed by declared inputs, action, environment, and toolchain identity.
6. Package resolution must produce a committed lockfile before reproducible package builds are considered stable.
7. Shell command targets are an escape hatch; typed executable actions are preferred.

## Current execution model

The CLI parses `Wportage.toml`, validates target references and cycles, and computes a deterministic dependency-first order. The scheduler selects ready targets in stable name order and runs up to `jobs` targets concurrently. It waits for the current batch before scheduling dependents. A failure prevents later batches from starting; peers already running in the failed batch are joined.

Two action types are currently supported:

- `exec`: starts a program directly with an argument vector and declared environment variables. No shell parsing is involved.
- `command`: executes shell command strings through the host command interpreter. This is intentionally retained as an escape hatch.

The executor now accepts optional `outputs = ["relative/path"]` declarations per target. Output paths must be workspace-relative and cannot traverse outside the project. Before execution, the reachable graph is checked for overlapping output paths; after each action, every declared output must exist. This prevents concurrent targets from claiming the same artifact path. Each target may set `timeout_ms` to bound its execution. On Unix, every action is started in a fresh process group; cancellation and timeout send SIGKILL to the complete group, then reap the direct child. This also terminates descendants spawned by shell actions. On non-Unix platforms, the current fallback only terminates and reaps the direct child; Windows Job Object support remains future work. Process-tree cancellation does not yet provide graceful signal propagation or configurable shutdown grace periods. The current executor still does not model declared inputs, cache keys, incremental state, sandbox policies, or toolchain identities. Parallel scheduling is not a claim of hermetic or reproducible execution.

## Planned subsystem boundaries

- CLI: argument parsing and user-facing diagnostics.
- Manifest: schema, validation, and compatibility.
- Graph: target graph and deterministic traversal.
- Scheduler: ready-queue scheduling and resource limits.
- Executor: process lifecycle and action execution.
- Toolchain: compiler, linker, assembler, sysroot, and target triples.
- Cache: content-addressed artifacts and action metadata.
- Sandbox: filesystem and environment isolation.
- Package: package metadata, store, and installation transactions.
- Resolver: version solving and lockfile generation.
- Remote: authenticated remote cache and execution.

## CLI principles

Prefer short verbs: `init`, `build`, `test`, `clean`, `add`, `remove`, `sync`, `update`, `install`, `uninstall`, `search`, `info`, `graph`, and `doctor`. Commands are introduced only when their behavior is implemented and tested.
