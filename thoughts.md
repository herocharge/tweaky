Problem

Part 1

AI image generation is powerful, but it is still barely configurable in the ways that matter for precision work. If I want a model to follow exact instructions, I often need a more expensive reasoning model, and even then it may still interpret my intent incorrectly.

This is not entirely the model's fault. A natural language prompt is underspecified. We cannot expect users to describe an image pixel by pixel, so the model fills in the gaps with reasonable guesses. Those guesses are often helpful, but they also make the result hard to control.

One possible fix is to stop asking the model to emit a final `png` or `jpeg` directly. Instead, the model should emit something more like a Photoshop document, a PDF editor file, or another editable scene format. Then the user can inspect and tweak the generated structure without needing another model call for every small change.

Part 2

The most accessible interface I currently have for image generation is chat. I want my Codex subscription to be able to generate and tweak images. The model is already smart enough to help; it just needs the right visual interface and output format.

Solution Direction

Create a new image and visual generation platform based on a hierarchical scene graph. Conceptually, this should feel closer to a game engine tree, a DOM tree, or an AST than to a flat bitmap.

Each object in the tree is generated as a component with code and parameters:

- A root node renders itself and then renders its children.
- Nodes can represent shapes, groups, layers, brush strokes, effects, masks, text, or procedural generators.
- Each node has editable parameters instead of baking everything into pixels immediately.

This should be built for humans first, not AI first. The initial platform should feel like an editor for structured visual composition. It should ship with default components for common visual primitives such as shapes, fills, paths, transforms, blur, noise, and brush-like marks.

Then we add the AI layer. Instead of asking the AI to produce pixels, we ask it to generate and modify objects in this hierarchy. In the ideal version, each component also advertises how it can be tweaked. That means the AI can inspect the tree it produced, identify likely user-facing controls, and surface sliders, toggles, handles, or presets automatically.

The important idea is that the editable structure is not an afterthought. It is the primary artifact.

Why This Is Interesting

- It separates "generation" from "final rasterization."
- It gives users a real editing surface instead of forcing prompt iteration.
- It gives AI a structured target to emit, inspect, and revise.
- It opens the door to mixed workflows where humans, deterministic tools, and AI all manipulate the same scene graph.
- It creates a path toward reproducibility, version control, and diffing for visual work.

Core Product Thesis

The product is not "an image model."

The product is a programmable, editable visual document format with:

- A renderer
- An editor
- A component system
- A schema for tweakable parameters
- An AI interface that reads and writes that structure

Core Architecture Sketch

1. Scene Document Format

Define a serializable document format, probably JSON-based at first, that stores:

- Document metadata
- A root scene node
- A tree of child nodes
- Component type for each node
- Parameters for each component
- References to shared resources such as palettes, textures, fonts, and reusable symbols

This format should be easy for both humans and models to read and edit. It should also be stable enough to support versioning and diffs.

2. Scene Graph Runtime

Build a runtime that loads the document and turns it into an in-memory scene graph.

Each node should expose:

- `render(context)`
- `getBounds()`
- `getControls()`
- `serialize()`
- `validate()`

Optional later methods:

- `hitTest(point)`
- `toPath()`
- `suggestTweaks()`

3. Component System

Every visual primitive should be a component with:

- A type name
- A parameter schema
- Rendering logic
- Default values
- UI control metadata

Example component families:

- `Group`
- `Rectangle`
- `Ellipse`
- `Path`
- `Text`
- `BrushStroke`
- `Image`
- `Mask`
- `Shadow`
- `Blur`
- `Noise`
- `Repeat`
- `Scatter`

The key architectural decision is that parameter schema and UI schema should live close to the rendering logic so the same component is easy to render, inspect, and edit.

4. Rendering Pipeline

The renderer should support at least two modes:

- Interactive preview rendering for editor responsiveness
- Final high-quality raster export

This implies quality levels, partial re-rendering, and good caching behavior. Nodes should ideally be re-rendered only when their inputs change.

5. Editor Layer

The editor should expose:

- Layer/tree view
- Canvas view
- Property inspector
- Direct manipulation controls
- Export actions

For MVP, the editor can be minimal. The critical goal is proving that structured generation plus direct manipulation is better than prompt-only iteration.

6. AI Interface Layer

The AI should not talk in raw pixels. It should interact with the document and component schemas.

Possible AI operations:

- Create a new document from a prompt
- Add nodes to an existing tree
- Modify parameters of selected nodes
- Rearrange structure
- Suggest useful controls for a generated composition
- Explain which nodes correspond to which visual elements

Long term, the model may need fine-tuning or constrained decoding so it reliably emits valid scene documents.

7. Export Layer

The system should export:

- Raster images such as `png`
- The native editable scene document
- Possibly `svg` for compatible subsets

The native format is the source of truth. Raster is just one output target.

Practical Product Decisions So Far

1. Native format should exist, but users should mostly interact with a visual editor.

The native document format is primarily for the system, AI, persistence, versioning, and advanced users. Most users should not have to read or edit raw document data directly.

The main interface should be visual and hierarchical:

- A Unity-like scene tree or hierarchy panel
- A canvas for direct manipulation
- An inspector for properties and generated controls
- Optional advanced access to the raw underlying document

So the right mental model is:

- Internally: structured scene document
- Externally: visual editor first, raw format second

2. The editor should be a hybrid of layer-based and node-based interaction.

Unity is a useful reference here. A pure node graph is probably the wrong default for ordinary image editing, while a pure layer stack is too limited for procedural and nested composition.

The likely best UX is:

- Primary structure: hierarchy/tree view
- Primary editing: canvas plus inspector
- Optional deeper procedural views for complex components later

That gives users something familiar enough to edit visually while preserving the compositional power of a structured scene graph.

3. This should target a serious desktop platform, not a toy framework.

If the goal is "real software" in the category of document editors, illustration tools, and eventually Photoshop-like workflows, then the safest direction is a native desktop application with a serious rendering stack.

Recommended direction:

- Desktop-first application
- Native rendering core in Rust or C++
- A real 2D graphics engine such as Skia
- A serious desktop UI layer such as Qt, or another mature native shell that can host a custom canvas/editor experience

Why this direction makes sense:

- Better control over rendering performance
- Better support for large documents and incremental redraw
- Better long-term path for precision tooling
- More credible foundation for professional editing workflows

What to avoid for the core product:

- Lightweight toy UI kits that are pleasant for demos but weak for complex editors
- Architectures that assume the canvas is just a web toy instead of the heart of the application

The web can still be useful later for viewers, lightweight sharing, or constrained editing, but the main product should start from a serious desktop architecture.

MVP Goals

The MVP should prove one thing: a structured visual document is a better interface for AI-assisted image creation than one-shot bitmap generation.

MVP scope:

- A native scene document format
- A renderer for a small set of primitives
- A simple editor UI
- Manual editing of generated nodes
- A basic AI bridge that generates valid scene documents from prompts

MVP component set:

- Group
- Rectangle
- Ellipse
- Path
- Text
- Fill
- Stroke
- Transform
- Blur or shadow as one simple effect

MVP user flow:

1. User enters a text prompt in chat.
2. AI generates a scene document instead of a final bitmap.
3. The document opens in the editor.
4. The user selects nodes and tweaks properties directly.
5. The system re-renders quickly.
6. The user exports a final image.

MVP non-goals:

- Photorealism
- Full Photoshop parity
- Arbitrary brush engine complexity
- Perfect AI generation quality
- Collaborative multiplayer editing
- Advanced animation

Success Criteria

- The AI can generate valid documents consistently.
- Users can understand the generated hierarchy well enough to edit it.
- Common edits can be done faster than re-prompting.
- Rendering is fast enough to feel interactive.
- The resulting images are expressive enough to demonstrate the concept.

Key Technical Constraints

Speed of rendering:

- Incremental rendering matters more than maximum offline quality for MVP.
- The scene graph should support dirty-region or subtree-based updates.
- Heavy effects should be limited early unless they can be cached well.

AI compatibility:

- The schema must be simple and predictable.
- Component parameters should be explicit and typed.
- Invalid outputs should be repairable automatically when possible.
- The system should prefer a small, reliable vocabulary of components over an ambitious but fragile one.

Big Open Questions

- Should the native format be purely declarative, or should nodes be allowed to contain custom code?
- How much procedural power should components have before they become too hard for AI and humans to reason about?
- How should a desktop UI shell and rendering engine be divided between app chrome and the custom canvas/runtime?
- Should AI generate the whole tree at once, or iteratively build and refine subtrees?
- How do we represent brush-like and painterly effects without collapsing back into opaque pixels too early?

Current Leaning On Open Questions

Native format:

The native format should start declarative. That will make it easier to validate, diff, repair, and generate with AI. Custom code inside document nodes is powerful, but it introduces security, portability, determinism, and model-reliability problems too early.

A good compromise is:

- Documents are declarative
- Components are implemented in trusted runtime code
- Later, limited procedural operators can be added as controlled component types instead of arbitrary embedded code

Brush-like and painterly content:

It is reasonable to support a fallback raster-backed object for content that does not compress well into clean vector or procedural structure.

This could be something like an `ImageLayer` or `PaintPatch` node:

- Stores transparent raster content
- Has transform, crop, mask, opacity, and blend controls
- Optionally stores extra metadata for smarter scaling or regeneration
- Can participate in the same hierarchy as procedural nodes

Useful extra metadata could include:

- Native resolution
- Intended display scale
- Prompt or generation provenance
- Optional segmentation or alpha structure
- Optional higher-resolution source for re-export

This gives the system a practical escape hatch. Not every visual element needs to be represented as pure geometry or brush simulation in the first version.

The important rule is that raster fallback should be a first-class node type, not a failure mode that breaks the rest of the editing model.

AI compatibility for raster fallback:

Yes, AI should be able to generate these raster-backed nodes too. In practice, the system may evolve toward mixed scenes:

- Structured nodes for layout, text, shapes, masks, transforms, and simple effects
- Raster-backed nodes for painterly details, textures, or hard-to-parameterize regions

That hybrid model is probably more realistic than forcing everything into one representation too early.

Suggested First Build Order

1. Choose the core platform and rendering stack for a serious desktop editor.
2. Define the declarative document schema.
3. Build a tiny renderer for 5 to 8 primitive node types plus one raster-backed fallback node.
4. Build a minimal Unity-like editor with hierarchy view, canvas, and inspector.
5. Hand-author a few example documents to validate the model.
6. Add an AI adapter that emits valid documents.
7. Add repair, validation, and schema-guided generation.
8. Test whether editing generated structure actually feels better than re-prompting.

Short Version

This project should treat image generation as structured program synthesis for a visual scene graph, not as direct bitmap emission. The editable scene document is the real product. AI becomes much more useful when it generates something users can inspect, manipulate, and continue refining by hand.
