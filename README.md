# tweaky

An experiment in AI-assisted image creation through editable visual scene documents instead of direct bitmap generation.

Current project docs:

- [CONTEXT.md](./CONTEXT.md): reload and handoff context for future sessions
- [thoughts.md](./thoughts.md): product idea and conceptual framing
- [spec.md](./spec.md): MVP specification and architecture direction
- [roadmap.md](./roadmap.md): milestone-by-milestone execution plan
- [docs/editor-shell.md](./docs/editor-shell.md): first editor-shell boundary for Milestone 3

Current implementation artifacts:

- [schemas/scene-document.schema.json](./schemas/scene-document.schema.json): JSON Schema for scene document version `0.1`
- [examples](./examples): hand-authored example `.vsd.json` scene documents
- [crates/scene_schema](./crates/scene_schema): Rust parsing and validation crate
- [crates/scene_runtime](./crates/scene_runtime): runtime registry, traversal, mutation commands, and shared geometry/bounds helpers
- [crates/renderer](./crates/renderer): backend-agnostic render plan and optional Skia CPU raster backend, now consuming runtime geometry semantics
- [Cargo.toml](./Cargo.toml): workspace root
- [apps/editor/qt_shell](./apps/editor/qt_shell): Qt Widgets desktop shell prototype

Current rendering coverage:

- `Rectangle`
- `Ellipse`
- `Text`
- `Path` with point-list geometry
- `ImageLayer` planning support
- style-level `blur` and `shadow` effects on drawable nodes

Current interaction coverage:

- shared bounds computation in `scene_runtime`
- topmost-first scene hit testing
- polygon-aware hit testing for closed `Path` nodes

Useful renderer commands:

- `cargo test`
- `cargo test -p renderer --features skia-safe-backend`

Useful editor commands:

- `cargo run -p editor -- examples/basic_poster.vsd.json --export /tmp/tweaky-editor-smoke.png`
- `cargo run -p editor -- examples/basic_poster.vsd.json --dump-view-model`
- `cargo run -p editor -- examples/basic_poster.vsd.json --rename-node headline "Title Block"`
- `cargo run -p editor -- examples/basic_poster.vsd.json --set-position headline 320 360 --set-params-json headline '{"text":"JSON MODE","fontFamily":"Inter","fontSize":72,"lineHeight":1.0}' --set-style-json headline '{"fill":"#445566"}'`
- `cmake -S apps/editor/qt_shell -B build/qt_shell -DCMAKE_PREFIX_PATH=$(brew --prefix qt)`
- `cmake --build build/qt_shell`
- `./build/qt_shell/tweaky-editor-qt examples/basic_poster.vsd.json`

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

Current project phase:

- Milestone 1 complete
- Milestone 2 functionally complete for the current MVP scope
- Milestone 3 active with a real editor app scaffold, a compiled Qt shell prototype, a Rust-owned view-model boundary for hierarchy/inspector/canvas preview data, working Qt open/reload/save/save-as/export document actions, a temp working-copy edit loop with dirty-state tracking, keyboard nudging for selected nodes, and a JSON-first inspector with direct position controls
