# Roadmap

This document turns the MVP milestones from `spec.md` into an execution plan.

Chosen stack:

- Language/runtime core: Rust
- Desktop shell: Qt
- Rendering engine: Skia

The goal of this roadmap is to make implementation order explicit, reduce thrash, and define what "done" means for each phase.

## Working Rules

- Keep the document model declarative.
- Prefer a small, reliable node vocabulary over broad early ambition.
- Optimize for interactive editing speed before visual complexity.
- Treat AI integration as a consumer of the scene schema, not a separate product.
- Keep commits small and meaningful.

## Milestone 0: Project Bootstrap

Purpose:

Create a clean project foundation so implementation can begin without organizational churn.

Tasks:

- Initialize Git repository
- Create initial project docs and decision records
- Choose initial repo layout
- Add `.gitignore`
- Decide license later if needed, but leave room for it in repo structure

Suggested repo layout:

```text
draw/
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

Deliverables:

- Git repo initialized
- Core docs committed
- Basic directory strategy agreed on

Exit criteria:

- A new contributor can clone the repo and understand the project shape from the top-level docs

## Milestone 1: Document Schema And Runtime

Purpose:

Define the source-of-truth document format and make it loadable, savable, and validatable.

Scope:

- Scene document schema
- Node identity model
- Resource reference model
- Validation layer
- Serialization/deserialization

Detailed tasks:

1. Define schema boundaries

- Freeze top-level document fields
- Freeze node common fields
- Freeze transform model
- Freeze style model for MVP
- Freeze resource reference shape

2. Define MVP component contracts

- `Group`
- `Rectangle`
- `Ellipse`
- `Path`
- `Text`
- `ImageLayer`
- `Shadow`
- `Blur`

3. Implement machine-checkable schema

- Create JSON Schema for document version `0.1`
- Document required and optional fields
- Define enum constraints
- Define validation error categories

4. Build Rust schema crate

- Parse scene documents from JSON
- Serialize scene documents to JSON
- Validate schema-level correctness
- Preserve stable field ordering if useful for diffs

5. Build runtime crate skeleton

- Scene graph node registry
- Component definition registry
- Validation pipeline
- Basic traversal helpers

6. Create hand-authored examples

- Simple poster scene
- Shape composition scene
- Text plus raster hybrid scene

Deliverables:

- JSON Schema file
- Rust document types
- Validation pipeline
- Example documents

Exit criteria:

- Example documents load successfully
- Invalid documents produce useful validation output
- Document format is stable enough for renderer work to begin

Risks:

- Over-designing the schema before real renderer/editor feedback
- Letting effects and text modeling balloon too early

Risk controls:

- Keep schema versioned
- Limit node vocabulary aggressively
- Add extension points without expanding MVP scope

## Milestone 2: Renderer

Purpose:

Build a serious rendering path for the MVP node set with good interaction performance.

Scope:

- Scene traversal
- Paint/style application
- Transform stack
- Bounds calculation
- Hit testing
- Incremental redraw support

Detailed tasks:

1. Renderer architecture

- Define render context abstraction
- Define scene traversal order
- Define z-order semantics from hierarchy order
- Define clipping and masking strategy for MVP

2. Skia integration

- Set up Skia surface creation
- Set up CPU and GPU-backed rendering options if needed
- Build canvas abstraction over Skia primitives

3. Implement MVP node rendering

- `Group`
- `Rectangle`
- `Ellipse`
- `Path`
- `Text`
- `ImageLayer`

4. Implement simple effects

- `Shadow`
- `Blur`
- Opacity
- Blend mode baseline

5. Geometry and interaction support

- Bounds computation
- Hit testing
- Selection outlines
- Transform handles contract for editor integration

6. Incremental rendering

- Dirty node tracking
- Cached bounds
- Subtree invalidation
- Resource cache for images and fonts

7. Export

- `png` export path
- Deterministic render output for test scenes where possible

Deliverables:

- Renderable scene pipeline
- Hit testing support
- PNG export
- Example outputs from sample documents

Exit criteria:

- Example documents render correctly
- Selection and bounds are trustworthy enough for editor work
- Small edits do not require full document re-render in the common case

Risks:

- Skia integration complexity
- Text rendering inconsistencies
- Effect caching complexity

Risk controls:

- Start with minimal text capabilities
- Keep effects shallow in MVP
- Make performance instrumentation visible early

## Milestone 3: Editor Shell

Purpose:

Wrap the document/runtime/renderer in a usable desktop editing experience.

Scope:

- Qt desktop shell
- Hierarchy panel
- Canvas host
- Inspector
- Basic document actions

Detailed tasks:

1. Desktop app skeleton

- Window shell
- Menu bar
- Dock or panel layout
- Project/open/save/export actions

2. Canvas integration

- Host Skia-rendered canvas in Qt
- Pointer event routing
- Keyboard shortcut routing
- Resize handling

3. Hierarchy panel

- Display scene tree
- Selection sync with canvas
- Reordering
- Visibility toggle
- Lock toggle
- Grouping basics

4. Inspector

- Show selected node type
- Show typed parameter controls
- Show transform controls
- Show style controls
- Show component-specific controls

5. Editing commands

- Select
- Move
- Rotate
- Scale
- Resize
- Delete
- Duplicate
- Undo/redo command backbone

6. Document lifecycle

- New document
- Open document
- Save document
- Save as
- Export PNG

Deliverables:

- Usable desktop app shell
- Interactive canvas
- Hierarchy and inspector working together

Exit criteria:

- A user can open a scene, select nodes, modify properties, and export output without touching raw JSON

Risks:

- Event coordination across Qt and custom canvas
- Overbuilding the editor shell before workflow validation

Risk controls:

- Keep editor chrome minimal at first
- Focus on selection, transform, and property edits before polish

## Milestone 4: AI Integration

Purpose:

Let AI create and modify real scene documents inside the editor workflow.

Scope:

- Prompt-to-document generation
- Validation and repair
- Document patching or replacement strategy
- AI editing UX

Detailed tasks:

1. Schema-constrained generation design

- Define prompt format
- Define allowed component vocabulary
- Define output contract
- Decide whether AI returns full documents or structured patches first

2. AI adapter crate

- Request builder
- Response parser
- Validation bridge
- Repair pipeline
- Error reporting

3. Repair system

- Fill missing defaults
- Reject unknown component types
- Convert minor invalid values where safe
- Surface non-repairable failures clearly

4. Editor integration

- AI prompt panel or chat dock
- Preview generated scene before apply if useful
- Apply as new document or current document edit
- Explain generated node mappings to user

5. Basic usability features

- "Make this more minimal"
- "Move the title higher"
- "Change the background tone"
- "Replace painterly area with raster layer"

Deliverables:

- AI prompt flow inside editor
- Valid generated scene documents
- Repair and error handling path

Exit criteria:

- The AI can generate scenes that users can open and edit immediately
- Simple follow-up edits work reliably enough to demonstrate the core product thesis

Risks:

- Model outputs drifting outside schema
- Poor UX if every AI edit replaces the whole document

Risk controls:

- Start with very constrained vocabulary
- Keep repair rules explicit
- Prefer transparent failure modes over hidden magic

## Milestone 5: Workflow Validation

Purpose:

Prove the product is actually better than prompt-only image generation for the target use case.

Scope:

- Structured editing workflow validation
- Performance validation
- User comprehension validation
- MVP refinement decisions

Detailed tasks:

1. Define evaluation scenarios

- Poster concept creation
- Social card composition
- Illustration with mixed vector and raster elements
- Text-layout-heavy composition

2. Measure core workflow questions

- Can users understand the generated hierarchy?
- Can users make common edits faster than re-prompting?
- Which node types get edited most often?
- Where does the AI output become confusing?

3. Measure technical quality

- Render latency during edits
- Load/save times
- Export times
- Memory footprint on medium scenes

4. Capture product adjustments

- Add or remove node types
- Refine inspector controls
- Refine AI prompting contract
- Decide whether patches or full replacement should be the primary AI edit mechanism

Deliverables:

- Internal evaluation notes
- Performance baselines
- Post-MVP adjustment list

Exit criteria:

- The team can clearly answer whether the structured-document approach beats prompt iteration for at least one real workflow

## Cross-Cutting Workstreams

These tasks should be revisited throughout multiple milestones.

### Testing

- Schema validation tests
- Renderer golden-image tests where practical
- Command/edit integration tests
- AI adapter validation tests with canned fixtures

### Performance

- Render timing instrumentation
- Dirty-region statistics
- Document size and load profiling

### Reliability

- Clear validation errors
- Crash-resistant document loading
- Stable save behavior

### Developer Experience

- Keep docs current with architectural changes
- Use small examples as regression fixtures
- Keep commit messages descriptive and scoped

## Suggested Commit Rhythm

Good commit boundaries for this project:

- One commit for repo/bootstrap changes
- One commit for schema design
- One commit for example documents
- One commit for runtime validation
- One commit per renderer capability slice
- One commit for each editor surface slice
- One commit for AI adapter foundation

Avoid:

- One giant "initial project" commit after days of work
- Mixing docs, runtime, renderer, and editor changes without a reason

## Immediate Next Actions

1. Initialize the repo and commit the current project docs
2. Create a machine-checkable JSON Schema from `spec.md`
3. Scaffold the Rust workspace and crate layout for document/runtime/rendering separation
4. Add first example scene documents as fixtures
