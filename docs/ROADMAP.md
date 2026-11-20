# Wportage Roadmap

## Milestone 0 — Repository foundation
- [x] Rust workspace and CLI crate
- [x] Initial manifest parser
- [x] Target dependency ordering and cycle detection
- [x] Native command execution without external executor
- [x] Basic init/build/test/clean/graph/doctor commands
- [x] CI workflow

## Milestone 1 — Build engine correctness
- [ ] Typed action model (no shell required for native actions)
- [ ] Artifact declarations and output validation
- [ ] Deterministic DAG validation and diagnostics
- [ ] Parallel scheduler with resource limits
- [ ] Process cancellation and signal propagation
- [ ] Unit and integration test coverage

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
