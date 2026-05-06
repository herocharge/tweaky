# tweaky

An experiment in AI-assisted image creation through editable visual scene documents instead of direct bitmap generation.

Current project docs:

- [CONTEXT.md](./CONTEXT.md): reload and handoff context for future sessions
- [thoughts.md](./thoughts.md): product idea and conceptual framing
- [spec.md](./spec.md): MVP specification and architecture direction
- [roadmap.md](./roadmap.md): milestone-by-milestone execution plan
- [docs/editor-shell.md](./docs/editor-shell.md): first editor-shell boundary for Milestone 3
- [docs/ai-contract.md](./docs/ai-contract.md): first AI generation and revision contract

Current implementation artifacts:

- [schemas/scene-document.schema.json](./schemas/scene-document.schema.json): JSON Schema for scene document version `0.1`
- [examples](./examples): hand-authored example `.vsd.json` scene documents
- [examples/pelican_bicycle.vsd.json](./examples/pelican_bicycle.vsd.json): first AI benchmark scene around "a drawing of a pelican riding a bicycle"
- [crates/scene_schema](./crates/scene_schema): Rust parsing and validation crate
- [crates/scene_runtime](./crates/scene_runtime): runtime registry, traversal, mutation commands, and shared geometry/bounds helpers
- [crates/renderer](./crates/renderer): backend-agnostic render plan and optional Skia CPU raster backend, now consuming runtime geometry semantics
- [crates/ai_adapter](./crates/ai_adapter): provider abstraction for AI generation, with a mock backend plus Gemini/openai-compatible configuration seams
- [Cargo.toml](./Cargo.toml): workspace root
- [apps/editor/qt_shell](./apps/editor/qt_shell): Qt Widgets desktop shell prototype

Current rendering coverage:

- `Rectangle`
- `Ellipse`
- `Text`
- `Path` with point-list geometry
- `ImageLayer` planning support
- `ImageLayer` resource-path rendering in Skia export and editor view models
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
- `cargo run -p editor -- examples/hybrid_scene.vsd.json --export /tmp/tweaky-hybrid-smoke.png`
- `cargo run -p editor -- examples/basic_poster.vsd.json --dump-view-model`
- `cargo run -p editor -- --prompt "a drawing of a pelican riding a bicycle" --ai-provider mock --write-generated /tmp/mock-pelican.vsd.json --dump-view-model`
- `cargo run -p editor -- --prompt "a drawing of a pelican riding a bicycle" --ai-provider gemini --ai-model gemini-2.5-flash --ai-api-key-env GEMINI_API_KEY --write-generated /tmp/pelican.vsd.json`
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
- a provider abstraction for model backends; OpenAI-compatible APIs are a useful extension target, but not treated as the native contract

AI provider notes:

- put secrets in local env vars or a local `.env`, never in committed config
- `.env` files are ignored, and [.env.example](./.env.example) shows the expected shape
- current CLI/env provider knobs are `TWEAKY_AI_PROVIDER`, `TWEAKY_AI_MODEL`, `TWEAKY_AI_FALLBACK_MODELS`, `TWEAKY_AI_API_KEY_ENV`, and `TWEAKY_AI_BASE_URL`
- the Gemini path now falls back from `gemini-2.5-flash` to `gemini-2.5-flash-lite` on transient capacity errors like `UNAVAILABLE`
- the Gemini path now renders the generated scene and can run one image-based critique/revision pass
- the Gemini path now also uses repo-native few-shot examples and one retry-with-feedback pass when the model returns malformed scene output
- the Gemini path now prefers a two-pass flow: scene plan first, final scene JSON second
- the Gemini path now chooses a repo-native scaffold family before planning, so generation starts from a structural template instead of from zero
- the next AI direction is typed scene operations / tool-call style document edits instead of relying only on raw subtree emission

Chosen stack:

- Rust
- Qt
- Skia

Near-term goal:

Build an MVP desktop editor where AI generates a structured visual document that users can inspect, tweak, and export.

Current project phase:

- Milestone 1 complete
- Milestone 2 functionally complete for the current MVP scope
- Milestone 3 active with a real editor app scaffold, a compiled Qt shell prototype, a Rust-owned view-model boundary for hierarchy/inspector/canvas preview data, working Qt open/reload/save/save-as/export document actions, a temp working-copy edit loop with dirty-state tracking and undo/redo, keyboard nudging plus corner-handle resizing for simple bound-based nodes, direct path-point editing, and text layout support that now reaches preview/export via `lineHeight`, `maxWidth`, and `align`
- Milestone 4 now has a live Gemini prompt-to-scene path with fallback, few-shot prompting, repair retries, and a mock path through the same provider interface
