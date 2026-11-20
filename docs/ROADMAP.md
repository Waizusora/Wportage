# Wportage Roadmap

## Milestone 0 — Repository foundation
- [x] Rust workspace and CLI crate
- [x] Initial manifest parser
- [x] Target dependency ordering and cycle detection
- [x] Native command execution without external executor
- [x] Basic init/build/test/clean/graph/doctor commands
- [x] CI workflow

## Milestone 1 — Build engine correctness
- [x] Typed executable actions that do not require a shell
- [x] Deterministic dependency validation and traversal
- [x] Bounded parallel scheduling of independent targets
- [x] Failure propagation between scheduler batches
- [x] Unit coverage for dependency ordering, cycles, missing targets, and job limits
- [x] Declared artifact outputs and output validation
- [x] Direct-child timeout, cancellation, and reaping
- [ ] Process-tree cancellation and signal propagation (Unix process groups / Windows job objects)
- [ ] Integration tests for parallel execution and failure behavior

## Milestone 2 — Toolchains
- [ ] C and C++ compiler toolchains
- [ ] Rust toolchain integration
- [ ] Assembly and linker integration
- [ ] Cross-compilation profiles and sysroot handling
- [ ] Kernel-oriented targets and linker scripts

## Milestone 3 — Reproducibility and performance
- [ ] Input hashing and action keys
- [ ] Local content-addressable cache
- [ ] Incremental rebuilds
- [ ] Environment allowlists and hermetic execution
- [ ] Build provenance and artifact manifests

## Milestone 4 — Packages and dependencies
- [ ] Version constraints and lockfile
- [ ] Dependency resolver
- [ ] Package metadata and local store
- [ ] Registry client and package verification
- [ ] Install, remove, search, info, update, sync

## Milestone 5 — Advanced execution
- [ ] Sandboxing
- [ ] Remote cache
- [ ] Remote execution protocol
- [ ] Multi-platform workers
- [ ] Security review and reproducible release pipeline
