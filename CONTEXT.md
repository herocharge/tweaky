# CONTEXT

This file is a reload and handoff document for the project.

If the session is lost, read this file first, then `README.md`, `spec.md`, and `roadmap.md`.

## Project Name

`tweaky`

## One-Sentence Summary

`tweaky` is a desktop editor where AI generates an editable visual scene document instead of a final bitmap, so users can tweak the result directly through a hierarchy, canvas, and inspector.

## Current Product Thesis

The core idea is that image generation should target a structured scene document rather than directly emitting `png` or `jpeg`.

That document is:

- The source of truth
- Editable by humans
- Readable and writable by AI
- Renderable to raster exports

The user-facing experience should feel like a serious desktop editor, closer to Unity's hierarchy model than a pure node graph.

## Locked Decisions

These are current project decisions and should be treated as default assumptions unless explicitly changed.

### Product and UX

- Users primarily interact with a visual editor, not raw document JSON
- The editor should center on hierarchy + canvas + inspector
- The native format should start declarative
- The system should support hybrid scenes with both structured nodes and raster-backed fallback nodes

### Tech stack

- Chosen stack: `Rust + Qt + Skia`
- This corresponds to Option A from `spec.md`
- The app should be desktop-first
- Avoid toy frameworks for the core editor

### MVP node vocabulary

- `Group`
- `Rectangle`
- `Ellipse`
- `Path`
- `Text`
- `ImageLayer`
- `Shadow`
- `Blur`

### Raster fallback

- Painterly or hard-to-parameterize content can be represented with `ImageLayer`
- `ImageLayer` is a first-class scene node, not a hack

### Parameter modeling

- The wire format remains generic JSON under `params`
- Rust code is starting to use typed parameter accessors on top of that generic structure
- Current typed accessors exist for rectangle, ellipse, text, and image-layer nodes

### Extensibility model

- The saved document should stay declarative
- Built-in node types act as a standard component library
- Alternate renderers and external generators should target the scene IR rather than embed arbitrary runtime code in documents

## Key Docs

- [README.md](/Users/herocharge/fun/draw/README.md)
- [thoughts.md](/Users/herocharge/fun/draw/thoughts.md)
- [spec.md](/Users/herocharge/fun/draw/spec.md)
- [roadmap.md](/Users/herocharge/fun/draw/roadmap.md)
- [docs/ai-contract.md](/Users/herocharge/fun/draw/docs/ai-contract.md)

Recommended reading order for reload:

1. `CONTEXT.md`
2. `README.md`
3. `spec.md`
4. `roadmap.md`
5. `thoughts.md`

## Current Repo State

The repo now has an initial Rust workspace scaffold and the first schema implementation.

Currently implemented:

- Root Cargo workspace
- `scene_schema` crate with parsing and validation
- `scene_runtime` crate with component registry, depth-first traversal, command-based mutation, bounds calculation, and hit testing
- `renderer` crate with render plan generation, primitive extraction, and optional Skia CPU PNG export
- `renderer` now uses `scene_runtime` as the source of truth for shared bounds semantics
- Basic `Path` support now exists end-to-end through typed params, runtime bounds, render planning, and Skia drawing
- Closed `Path` nodes now participate in polygon-aware hit testing instead of only bounding-box selection
- Style-level `blur` and `shadow` effects now affect bounds and Skia rendering
- `editor` now has a real app-state scaffold, CLI loading path, hierarchy summary, and PNG export workflow
- `editor` now emits a serialized view-model for the Qt shell, including hierarchy, node bounds, and render item data
- `apps/editor/qt_shell` now provides a compiled Qt Widgets shell prototype with hierarchy, inspector, a Rust-fed canvas preview, and working menu actions for open, reload, and export PNG
- the current inspector is intentionally JSON-first: selected-node name, `x`/`y`, raw `params`, and raw `style` edits flow from the Qt shell into the Rust editor app
- valid inspector edits now auto-apply without requiring the button, and scene refreshes preserve the current node selection by reusing the edited view model
- the Qt shell now edits a temp working copy instead of the original file directly; `Save` and `Save As` persist that working copy intentionally, export uses the working copy, and open/reload/close prompt on unsaved changes
- the Qt shell supports arrow-key nudging for the selected node, with `Shift` for larger 10-unit steps, while intentionally ignoring arrow keys when text fields are focused
- the Qt shell now supports corner-handle resizing for `Rectangle`, `Ellipse`, and `ImageLayer` nodes by rewriting `x`/`y` plus the corresponding size params through the Rust edit path
- the Qt shell now keeps lightweight snapshot history for undo/redo over the working copy, and selected `Text` nodes support double-click content editing plus keyboard font-size stepping
- selected `Path` nodes now expose draggable point handles on the canvas, and selected text nodes support line-height stepping in addition to content/font-size editing
- shared text layout now supports `lineHeight`, `maxWidth`, and `align` end-to-end across schema parsing, runtime bounds, Qt preview, Rust view models, and Skia PNG export
- `docs/ai-contract.md` now defines the first AI prompt/document/patch contract
- `examples/pelican_bicycle.vsd.json` now serves as the first funny benchmark scene for "a drawing of a pelican riding a bicycle"
- `ai_adapter` now has a provider abstraction with a real Gemini prompt-to-scene HTTP path, a mock fallback, typed response envelopes, and extension seams for openai-compatible backends
- the Gemini path now includes a fallback model chain for transient provider overloads, currently defaulting to `gemini-2.5-flash-lite`
- the Gemini path now also includes repo-native few-shot examples and one retry-with-feedback pass for malformed scene output
- the Gemini path now uses a two-pass plan-then-scene flow before falling back to simpler generation behavior
- the Gemini path can now render a generated scene to PNG and send that image back for one critique/revision round
- the Gemini path now chooses a repo-native template family (`poster`, `shapes`, or `hybrid`) based on the prompt and conditions planning/generation on that scaffold
- `ai_adapter` now also contains a typed scene-operation contract as the foundation for future Gemini tool-call style scene construction
- `ai_adapter` now supports raster-preferred stages that can declare image resources and place them as `ImageLayer` patches through scene ops
- the renderer/export path and editor view-model path now carry `ImageLayer` resource paths, and Skia export renders actual local raster assets when present
- `scene_schema` typed parameter accessors layered over the generic JSON document
- `editor` binary scaffold
- JSON Schema for document version `0.1`
- Hand-authored example scene documents
- Git remote `origin` configured at `git@github.com:herocharge/tweaky.git`
- `main` pushed upstream

Expected next implementation step:

- Expand editor-side mutation flows beyond name/position/raw-json edits so the hierarchy, inspector, and canvas can stop being mostly read-only
- Add save/save-as behavior through the Rust app layer
- Preserve the Rust-owned view-model boundary as the source of truth for UI data
- Keep renderer/runtime boundaries stable while the desktop shell becomes interactive
- Move the AI pipeline from raw subtree emission toward operation-first scene construction and execution

## Intended Repo Shape

Planned top-level structure:

```text
tweaky/
  docs/
  apps/
    editor/
  crates/
    scene_schema/
    scene_runtime/
    renderer/
    ai_adapter/
  assets/
  examples/
```

This is still a plan, not yet the real filesystem layout.

## Development Workflow

This is the preferred working loop for future sessions.

### Standard dev loop

1. Read `CONTEXT.md` for the latest decisions and next-step guidance
2. Read `git status` and `git log --oneline -n 10`
3. Read `git remote -v` if push state or upstream configuration matters
4. Pick one milestone slice from `roadmap.md`
5. Make the smallest useful change that advances that slice
6. Verify locally
7. Update docs if the architecture or workflow changed
8. Commit with a focused message
9. Push `main` unless the user says otherwise

### Commit rhythm

Prefer small, scoped commits such as:

- `chore: scaffold rust workspace`
- `feat: add scene document json schema`
- `feat: parse and validate scene documents`
- `feat: add skia renderer skeleton`
- `feat: add qt editor shell`

Avoid giant mixed commits that touch unrelated layers without a reason.

### When changing architecture

If any of these change, update this file and the spec:

- Chosen tech stack
- Native document shape
- MVP node vocabulary
- Editor interaction model
- Milestone order

## Verification Habits

When code exists, verification should usually include the following where relevant:

- Run tests for the touched crate
- Validate example scene documents
- Confirm formatter/linter status if configured
- Confirm the editor still loads example scenes
- If touching Skia integration, run `cargo test -p renderer --features skia-safe-backend`
- If touching the Qt shell boundary, rebuild `build/qt_shell` and smoke-launch `tweaky-editor-qt`
- If touching the mock AI path, run `cargo test -p ai_adapter -p editor`
- Smoke-test canned generation with `cargo run -p editor -- --prompt "a drawing of a pelican riding a bicycle" --ai-provider mock --write-generated /tmp/mock-pelican.vsd.json --dump-view-model`
- Smoke-test live Gemini generation with `GEMINI_API_KEY=... cargo run -p editor -- --prompt "a drawing of a pelican riding a bicycle" --ai-provider gemini --ai-model gemini-2.5-flash --write-generated /tmp/pelican.vsd.json`
- If Gemini returns `UNAVAILABLE`, the adapter now automatically retries against the configured fallback chain
- If Gemini returns malformed scene output, the adapter now retries once with explicit validation feedback before falling through the model chain
- The live Gemini path now asks for a compact scene plan before requesting the final scene JSON
- Keep secrets in ignored local env files or shell env vars, not committed configs

For documentation-only changes:

- Re-read edited files for consistency
- Keep naming and decisions aligned across docs

## Session Reload Prompt

If a future assistant needs a fast handoff, this is a good prompt seed:

```text
Read CONTEXT.md, README.md, spec.md, and roadmap.md. Assume the project name is tweaky. The chosen stack is Rust + Qt + Skia. Continue from the next implementation step unless I redirect you.
```

## Near-Term Next Steps

The next likely sequence is:

1. Preserve the current CLI/editor app workflow as a smoke-test path
2. Build from mock AI generation into real scene revision flows
3. Add image-aware Gemini inputs and scene-revision flows on top of the current live generation path
4. Keep preview/export consistency tightening as the editor matures
5. Commit and push each slice separately

## Notes For Future Codex Sessions

- The user cares about serious tooling and explicitly does not want toy frameworks for the core editor
- Keep the tone collaborative and practical
- Prefer doing the work instead of over-planning
- Maintain regular commits as work progresses
