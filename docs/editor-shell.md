# Editor Shell

This document captures the first Milestone 3 editor-shell boundary for `tweaky`.

## Goal

Introduce a real application layer without blocking on full Qt availability in the local environment.

## Current Boundary

The editor app is currently split into:

- `app.rs`: editor state, document loading, hierarchy building, summary generation, export workflow
- `cli.rs`: lightweight command-line entry flow for smoke testing
- `qt_shell.rs`: placeholder integration boundary for the future desktop shell
- `main.rs`: top-level orchestration

## Why This Boundary Exists

Qt is the chosen long-term desktop shell, but Qt tooling is not installed in the current environment.

Rather than stall on platform setup, the project now has:

- a real editor state model
- a real document-open workflow
- a real render/export workflow
- a clean future insertion point for Qt-specific shell code

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

## Short-Term Next Steps

1. Keep the editor state model stable as the shell evolves
2. Add document-open and example-load commands at the app layer
3. Introduce a first canvas-host abstraction
4. Only then wire in the actual Qt boundary
