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
- [Cargo.toml](./Cargo.toml): workspace root

Chosen stack:

- Rust
- Qt
- Skia

Near-term goal:

Build an MVP desktop editor where AI generates a structured visual document that users can inspect, tweak, and export.
