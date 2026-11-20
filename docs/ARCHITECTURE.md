# Wportage Architecture

Wportage is a Rust-native build and package infrastructure. The first implementation intentionally keeps the user interface small while separating the long-term engine into clear subsystems.

## Design rules

1. The native Wportage engine must not require Another engine.
2. Build descriptions are declarative and versioned.
3. The build graph is validated before actions execute.
4. Execution must be deterministic where inputs and toolchains are fixed.
5. Cache entries are keyed by declared inputs, command, environment, and toolchain identity.
6. Package resolution must produce a committed lockfile before reproducible package builds are considered stable.
7. Shell command targets are an escape hatch, not the long-term typed action interface.

## Initial implementation

The current CLI parses `Wportage.toml`, validates target references and cycles, topologically orders dependencies, then executes command actions directly using the host command interpreter. This is a bootstrap executor, not yet a hermetic or parallel build engine.

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
