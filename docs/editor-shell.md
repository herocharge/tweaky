# Editor Shell

This document captures the first Milestone 3 editor-shell boundary for `tweaky`.

## Goal

Introduce a real application layer while preserving a clean separation between desktop shell code and core editor/runtime logic.

## Current Boundary

The editor app is currently split into:

- `app.rs`: editor state, document loading, hierarchy building, summary generation, export workflow
- `cli.rs`: lightweight command-line entry flow for smoke testing
- `qt_shell.rs`: placeholder integration boundary for the future desktop shell
- `main.rs`: top-level orchestration
- `apps/editor/qt_shell`: Qt Widgets shell prototype

## Why This Boundary Exists

Rather than tie core editor logic directly to one UI layer, the project now has:

- a real editor state model
- a real document-open workflow
- a real render/export workflow
- a compiled Qt Widgets shell prototype
- a clean future insertion point for a stronger Rust/Qt bridge

This keeps Milestone 3 moving without polluting core app logic with platform assumptions.

## Planned Qt Shape

The first real Qt shell should likely map to:

- `QMainWindow` for the main desktop shell
- left panel: hierarchy
- center: canvas host
- right panel: inspector

Recommended responsibility split:

- Rust app state remains the source of truth
- Qt owns window chrome and widget composition
- the canvas host bridges into renderer output

Current prototype status:

- Implemented as a Qt Widgets app under `apps/editor/qt_shell`
- Loads `.vsd.json` scene data directly
- Displays hierarchy, inspector, and canvas placeholder panes
- Compiles successfully against local Qt 6

## Short-Term Next Steps

1. Keep the editor state model stable as the shell evolves
2. Introduce a first canvas-host abstraction
3. Reduce duplicated scene-loading logic between the Qt shell and the Rust editor app
4. Add hierarchy and inspector view-model slices that both shells can reuse
