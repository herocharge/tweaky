# Visual Scene Document Editor

## Purpose

This document turns the idea in `thoughts.md` into a buildable MVP specification.

The core bet is simple:

- AI should generate an editable visual document, not just a final bitmap.
- The document should be represented as a structured scene graph.
- Users should primarily interact with that structure through a serious visual editor, not raw JSON.

The MVP goal is to prove that a structured scene document is a better interface for AI-assisted image creation than prompt-only bitmap generation.

## Product Definition

The product is a desktop editor for a native visual document format.

It has five core parts:

1. A native scene document format
2. A scene graph runtime
3. A renderer
4. A visual editor
5. An AI adapter that reads and writes the document format

The native format is the source of truth. Raster exports are outputs, not the main artifact.

## Product Principles

1. Visual-first UX

Users should primarily see:

- A hierarchy panel
- A canvas
- An inspector
- Direct manipulation handles

They should not need to read raw document data unless they explicitly want advanced access.

2. Declarative documents first

The document format should start declarative. Arbitrary embedded user code is out of scope for MVP.

That gives us:

- Better validation
- Better AI generation reliability
- Safer execution
- Better diffs and versioning

3. Serious desktop architecture

This should be built like real editing software, not a browser toy wrapped as a product.

4. Hybrid scene representation

The system should support both:

- Structured procedural/vector nodes
- Raster-backed fallback nodes for painterly or hard-to-parameterize regions

5. Fast interaction over maximal complexity

For MVP, interactive editing speed matters more than extreme visual sophistication.

## MVP Scope

The MVP should support:

- Opening and saving a native scene document
- Rendering a small set of node types
- Hierarchy-based editing
- Canvas selection and transforms
- Inspector-based property editing
- Incremental re-rendering
- AI-generated documents from prompts
- Export to `png`

The MVP should not attempt:

- Photorealism
- Full Photoshop parity
- Arbitrary plug-in execution
- Rich collaborative editing
- Advanced animation timelines
- Deep brush engine simulation

## Editor UX

The editor should feel closer to Unity's hierarchy model than to a pure node graph editor.

### Primary layout

- Left: hierarchy tree
- Center: canvas
- Right: inspector
- Top or bottom: document controls, export, AI chat, history

### Editing model

- Users select nodes from the hierarchy or canvas
- The inspector shows typed parameters and generated controls
- Nodes can be reordered, grouped, hidden, locked, and transformed
- Basic direct manipulation should include move, scale, rotate, and resize

### Advanced view

Later, some components may expose deeper procedural internals. That should be optional, not the default editing mode.

## Native Document Model

### Design goals

The native format should be:

- Declarative
- Human-readable enough for debugging
- AI-friendly
- Versionable
- Strictly validatable
- Stable across app versions

### File shape

Use JSON for the first implementation.

Suggested extension:

- `.vsd.json` for development
- Later a packaged format such as `.vsd` that can bundle assets

### Top-level schema

```json
{
  "version": "0.1",
  "document": {
    "id": "doc_001",
    "name": "Poster Concept",
    "width": 1600,
    "height": 900,
    "background": {"type": "solid", "color": "#f5f1e8"},
    "resources": {
      "images": {},
      "fonts": {},
      "palettes": {}
    },
    "root": {
      "id": "root",
      "type": "Group",
      "name": "Root",
      "visible": true,
      "locked": false,
      "transform": {
        "x": 0,
        "y": 0,
        "scaleX": 1,
        "scaleY": 1,
        "rotation": 0,
        "opacity": 1
      },
      "params": {},
      "children": []
    }
  }
}
```

### Document concepts

- `version`: schema version
- `document`: root document payload
- `resources`: shared external or embedded assets
- `root`: root scene node

### Node shape

Every node should have:

```json
{
  "id": "node_123",
  "type": "Rectangle",
  "name": "Card Background",
  "visible": true,
  "locked": false,
  "blendMode": "normal",
  "transform": {
    "x": 200,
    "y": 120,
    "scaleX": 1,
    "scaleY": 1,
    "rotation": 0,
    "opacity": 1
  },
  "params": {},
  "style": {},
  "children": [],
  "meta": {}
}
```

### Validation rules

- `id` must be unique
- `type` must exist in the runtime registry
- `params` must satisfy the component schema
- Only container-capable nodes may have children
- Resource references must resolve
- Unknown fields should be preserved when safe, but flagged

## Component Model

Each component type should be registered in the runtime with:

- Type name
- Parameter schema
- Default parameter values
- Render implementation
- Bounds implementation
- Inspector metadata
- Serialization and validation rules

### Component interface

Pseudo-interface:

```ts
interface ComponentDefinition {
  type: string;
  canHaveChildren: boolean;
  defaultParams(): object;
  validate(node: SceneNode, doc: SceneDocument): ValidationResult[];
  render(node: SceneNode, ctx: RenderContext): void;
  getBounds(node: SceneNode, ctx: LayoutContext): Rect;
  getControls(node: SceneNode): ControlSpec[];
}
```

### MVP component set

- `Group`
- `Rectangle`
- `Ellipse`
- `Path`
- `Text`
- `ImageLayer`
- `Shadow`
- `Blur`

### Style model

For MVP, style can remain simple:

- Fill
- Stroke
- Opacity
- Blend mode
- Shadow
- Blur

Longer term, some style may move into dedicated effect nodes when composition demands it.

## Raster Fallback Node

Painterly content should not block the whole system.

The MVP should include a first-class raster-backed node called `ImageLayer`.

### `ImageLayer` role

This node represents transparent raster content that still participates in the scene graph like any other node.

It should support:

- Position, scale, rotation
- Opacity
- Crop
- Optional mask
- Blend mode
- Export-aware scaling hints

### `ImageLayer` metadata

The node should optionally track:

- Source image resource id
- Native width and height
- Intended display scale
- Prompt provenance
- Regeneration hint text
- Alpha handling mode

### Why it matters

This gives the document model a practical hybrid path:

- Shapes, text, transforms, and layout stay structured
- Painterly details, textures, and difficult regions can remain raster-backed

That is likely a better early product than forcing everything into vectors or procedural brushes.

## Rendering Model

### Render modes

The renderer should support:

- Interactive preview rendering
- Final export rendering

### Rendering requirements

- Incremental updates when a subtree changes
- Cached resources and effect intermediates
- Accurate hit testing
- Predictable z-order from hierarchy order

### Rendering strategy

The runtime should maintain:

- A parsed scene graph
- A dirty set of changed nodes
- Cached bounds
- Cached render surfaces when worthwhile

### Export

MVP exports:

- `png`
- Native scene document

Future exports:

- `svg` for compatible subsets
- PDF-like print output

## AI Adapter

The AI layer should target document structure, not pixels.

### MVP AI operations

- Create a scene document from a prompt
- Modify an existing document by instruction
- Explain the generated hierarchy
- Suggest useful tweak controls

### Generation strategy

The AI adapter should emit validated JSON matching the scene schema.

Recommended flow:

1. Prompt model with schema and allowed component vocabulary
2. Parse output
3. Validate document
4. Repair common issues automatically
5. Load into editor

### Repair examples

- Missing required fields
- Invalid enum values
- Unknown component type
- Resource references to missing assets
- Child nodes attached to leaf types

### UX principle

AI should feel like a collaborator operating on the same document model the user sees in the editor.

## Recommended Tech Stack

This section is for the actual implementation target, not a vague idea pool.

### Recommendation: desktop-first, Rust core, Qt shell, Skia rendering

Recommended baseline architecture:

- Language: Rust for core runtime and document logic
- UI shell: Qt
- Rendering engine: Skia
- AI bridge: local adapter layer that talks to model APIs and validates schema output

### Why this is the strongest default

Rust core:

- Strong safety story
- Good fit for document parsing, validation, rendering orchestration, and asset/runtime code
- Good long-term maintainability for a graphics-heavy desktop app

Qt shell:

- Mature cross-platform desktop UI framework
- Strong support for app chrome, panels, windows, docking, input handling, and production desktop UX
- Good separation between UI shell concerns and backend/runtime concerns

Skia rendering:

- Serious 2D graphics engine
- Used in large production software stacks
- Strong foundation for paths, text, transforms, effects, and multiple backends

### Recommended app split

- Qt owns the desktop shell, panels, menus, dialogs, hierarchy view, inspector, and app lifecycle
- A custom canvas widget hosts the rendering surface
- Rust owns document parsing, validation, scene graph state, and edit commands
- Skia owns actual scene rendering and export

This keeps the editor architecture honest:

- A real desktop app shell
- A custom rendering core
- A clean document/runtime boundary

## Tech Stack Shortlist

### Option A: Rust + Qt + Skia

Pros:

- Best fit for a serious desktop editor
- Mature UI shell and windowing model
- Strong long-term path for large documents and complex tooling
- Easy to keep a clear boundary between editor UI and rendering engine

Cons:

- Higher integration cost
- Rust and Qt integration adds complexity
- Build and packaging work will be more involved than a web stack

Use when:

- You want the best long-term foundation for professional editing software

### Option B: C++ + Qt + Skia

Pros:

- Very standard architecture for graphics-heavy desktop software
- Fewer FFI boundaries than Rust + Qt
- Strongest alignment with existing desktop graphics tooling traditions

Cons:

- More footguns in core systems code
- Harder memory-safety story
- Less appealing if you want a modern systems-language codebase

Use when:

- You value industry-standard native tooling and minimal integration layers over Rust ergonomics

### Option C: Rust + custom desktop shell + wgpu

Pros:

- Modern Rust-native stack
- Strong control over rendering and engine design
- Fewer heavyweight framework dependencies

Cons:

- More editor infrastructure must be built from scratch
- Harder to get polished desktop UX quickly
- More risk in recreating solved desktop-app problems

Use when:

- You want a rendering-engine-first product and are willing to build more editor shell yourself

### Option D: Rust + Slint

Pros:

- Rust-friendly
- Native desktop target
- Compact and modern

Cons:

- Less proven for large, Photoshop-class editor UX than Qt
- More risk if the app needs highly customized professional desktop chrome and workflows

Use when:

- You want a lighter native stack and accept more product risk for a complex editor

## Recommendation Summary

Pick one of these two paths early:

1. Rust + Qt + Skia
2. C++ + Qt + Skia

If the product ambition is genuinely "serious visual editor," Qt plus a real graphics engine is the safest family of choices.

My recommendation is `Rust + Qt + Skia` unless you want to optimize aggressively for lower integration complexity over language safety.

Decision:

The project will proceed with `Option A: Rust + Qt + Skia`.

## MVP Milestones

### Milestone 1: document and runtime

- Define schema
- Implement parser and validator
- Register component definitions
- Load and save sample documents

### Milestone 2: rendering

- Render `Group`, `Rectangle`, `Ellipse`, `Path`, `Text`, and `ImageLayer`
- Implement transforms and simple effects
- Add hit testing and selection outlines

### Milestone 3: editor shell

- Build hierarchy panel
- Build canvas host
- Build inspector
- Add selection and transforms

### Milestone 4: AI integration

- Add prompt-to-document generation
- Add validation and repair
- Load generated scenes into the editor

### Milestone 5: workflow proof

- Compare manual editing of generated structure against prompt iteration
- Measure responsiveness and edit success
- Refine component vocabulary based on actual user edits

## Open Questions

- Should effects be embedded in style fields first, or represented as explicit child/effect nodes from the start?
- What is the minimum viable text model for useful poster and layout work?
- How should resources be packaged once the app moves beyond plain JSON files?
- Should AI edits be expressed as full document replacement, structured patches, or command sequences?
- What level of automatic control generation belongs in MVP versus later iterations?

## Proposed Immediate Next Step

Before building UI, lock three things:

1. The JSON schema for the first document version
2. The exact MVP node vocabulary
3. The platform choice between `Rust + Qt + Skia` and `C++ + Qt + Skia`

Once those are fixed, the runtime and renderer can start without thrashing.
