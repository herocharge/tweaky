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
- Placeholder crates for `scene_runtime`, `renderer`, and `ai_adapter`
- `editor` binary scaffold
- JSON Schema for document version `0.1`
- Hand-authored example scene documents

Expected next implementation step:

- Deepen `scene_schema` from baseline validation into more complete typed document modeling as needed
- Start `scene_runtime` with registry and traversal primitives

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
3. Pick one milestone slice from `roadmap.md`
4. Make the smallest useful change that advances that slice
5. Verify locally
6. Update docs if the architecture or workflow changed
7. Commit with a focused message

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

1. Build `scene_runtime` registry and traversal helpers
2. Decide whether validation should remain stringly in `params` for MVP or become partially typed per node
3. Add command-oriented document mutation helpers
4. Begin renderer crate architecture around Skia integration points
5. Commit each slice separately

## Notes For Future Codex Sessions

- The user cares about serious tooling and explicitly does not want toy frameworks for the core editor
- Keep the tone collaborative and practical
- Prefer doing the work instead of over-planning
- Maintain regular commits as work progresses
