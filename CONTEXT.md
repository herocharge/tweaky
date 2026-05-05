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
- the current inspector is intentionally JSON-first: selected-node name, `x`/`y`, raw `params`, and raw `style` edits flow from the Qt shell into the Rust editor app, write the scene back to disk, and reload the updated view model
- `scene_schema` typed parameter accessors layered over the generic JSON document
- Placeholder crate for `ai_adapter`
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

1. Add more property editing commands across the Rust editor app boundary
2. Add save/save-as actions that flow through the Rust app layer
3. Expand the canvas from preview-only rendering toward interactive selection/manipulation
4. Preserve the current CLI/editor app workflow as a smoke-test path
5. Commit and push each slice separately

## Notes For Future Codex Sessions

- The user cares about serious tooling and explicitly does not want toy frameworks for the core editor
- Keep the tone collaborative and practical
- Prefer doing the work instead of over-planning
- Maintain regular commits as work progresses
