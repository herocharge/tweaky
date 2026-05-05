# Editor Shell

This document captures the first Milestone 3 editor-shell boundary for `tweaky`.

## Goal

Introduce a real application layer while preserving a clean separation between desktop shell code and core editor/runtime logic.

## Current Boundary

The editor app is currently split into:

- `app.rs`: editor state, document loading, hierarchy building, summary generation, export workflow
- `cli.rs`: lightweight command-line entry flow for smoke testing and view-model dumping
- `qt_shell.rs`: placeholder integration boundary for the future desktop shell
- `main.rs`: top-level orchestration
- `apps/editor/qt_shell`: Qt Widgets shell prototype

## Why This Boundary Exists

Rather than tie core editor logic directly to one UI layer, the project now has:

- a real editor state model
- a real document-open workflow
- a real render/export workflow
- a serializable Rust-owned view model for hierarchy, inspector, node bounds, and canvas render items
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
- the Qt shell should consume Rust-produced UI data instead of independently re-deriving scene semantics
- the Qt shell should treat the loaded scene as a working document, not a direct file mutation target; edits can auto-apply into a temp working copy, while explicit save actions persist back to disk
- the canvas host can start with Rust-produced preview primitives before later moving closer to live renderer hosting

Current prototype status:

- Implemented as a Qt Widgets app under `apps/editor/qt_shell`
- First tries to load a Rust-produced JSON view model via `editor --dump-view-model`
- Falls back to raw `.vsd.json` loading if the Rust CLI is unavailable
- Displays hierarchy, inspector, and a simple canvas preview driven by Rust-fed render items
- Exposes `File` actions for open, reload, save, save-as, export PNG, and quit
- Supports selected-node name, `x`/`y`, raw `params`, and raw `style` edits from the inspector via the Rust editor CLI
- Auto-applies valid inspector edits and preserves selection on refresh by consuming the edited Rust view model directly
- Maintains a temp working copy so `Save` and `Save As` are explicit document actions instead of every edit mutating the original file
- Supports arrow-key nudging for selected nodes, with larger `Shift` steps, while leaving text-entry fields alone
- Compiles successfully against local Qt 6

## Short-Term Next Steps

1. Keep the editor state model stable as the shell evolves
2. Add more editable properties through the Rust app layer beyond name/position/raw-json
3. Add richer canvas manipulation like resize handles and direct geometry editing
4. Replace more fallback-only Qt logic with Rust-owned view/state data over time
