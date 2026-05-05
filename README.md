# tweaky

An experiment in AI-assisted image creation through editable visual scene documents instead of direct bitmap generation.

Current project docs:

- [CONTEXT.md](./CONTEXT.md): reload and handoff context for future sessions
- [thoughts.md](./thoughts.md): product idea and conceptual framing
- [spec.md](./spec.md): MVP specification and architecture direction
- [roadmap.md](./roadmap.md): milestone-by-milestone execution plan

Current implementation artifacts:

- [schemas/scene-document.schema.json](./schemas/scene-document.schema.json): JSON Schema for scene document version `0.1`
- [examples](./examples): hand-authored example `.vsd.json` scene documents
- [crates/scene_schema](./crates/scene_schema): Rust parsing and validation crate
- [crates/scene_runtime](./crates/scene_runtime): runtime registry, traversal, mutation commands, and shared geometry/bounds helpers
- [crates/renderer](./crates/renderer): backend-agnostic render plan, bounds layer, and optional Skia CPU raster backend
- [Cargo.toml](./Cargo.toml): workspace root

Useful renderer commands:

- `cargo test`
- `cargo test -p renderer --features skia-safe-backend`

Architecture direction:

- declarative scene IR
- built-in component library
- pluggable renderer backends
- external generators/adapters that target the same IR

Chosen stack:

- Rust
- Qt
- Skia

Near-term goal:

Build an MVP desktop editor where AI generates a structured visual document that users can inspect, tweak, and export.
