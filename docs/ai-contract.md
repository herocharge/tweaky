# AI Contract

This document defines the first practical AI integration contract for `tweaky`.

Provider note:

- There is no single universal multimodal provider standard that cleanly covers scene-JSON generation, image understanding, structured outputs, and revision flows all at once.
- `tweaky` should keep its own provider abstraction around scene generation.
- OpenAI-compatible APIs are a useful extension target, but not the app's native contract.

The goal is not "prompt to final bitmap."

The goal is:

- prompt to editable scene document
- prompt to scene patch
- AI as a document author and reviser
- the editor as the review and correction loop

## Product Role

The AI should operate at the scene-document layer.

That means the model should:

- create new scene documents
- revise existing scene documents
- revise selected nodes or subtrees
- prefer structured nodes when possible
- fall back to raster-backed nodes when structure is not a good fit

The AI should not be treated as a final-image black box in MVP.

## First MVP Flows

The first AI flows should be:

1. `Prompt -> New Scene`
2. `Prompt + Current Scene -> Revised Scene`
3. `Prompt + Selected Node/Subtree -> Revised Subtree`

These cover the core product thesis without forcing us into complicated model orchestration too early.

In practice, the current `tweaky` Gemini path should prefer a two-pass generation flow:

1. `Prompt -> Scene Plan`
2. `Scene Plan -> Full Scene Document`

This splits composition thinking from strict schema emission and tends to be more stable than a single giant prompt.

After initial generation, the next refinement layer should be:

1. Render the generated scene
2. Send the rendered image, prompt, and scene JSON back to the model
3. Ask for targeted critique and a revised full scene

This gives the model a chance to see visual failures instead of only reasoning over JSON.

## Output Modes

The first contract should support two output modes.

### Mode 1: Full document

Use this for:

- new scene generation
- large scene rewrites

The model returns a full `.vsd.json` document.

### Mode 2: Patch

Use this for:

- selected-node revision
- localized edits
- conversational follow-ups like "move the bird higher"

The model returns a structured patch format that maps to scene edits.

For MVP, full-document output is the easiest place to start. Patch mode should follow once the generation path is stable.

## Generation Strategy

The AI should follow a three-tier generation strategy:

1. Prefer native structured nodes

- `Group`
- `Rectangle`
- `Ellipse`
- `Path`
- `Text`
- `ImageLayer`

2. Use `ImageLayer` when structure is a poor fit

- painterly textures
- collage fragments
- awkward organic detail
- external generated image cutouts

3. Keep layout and composition explicit

- use real positions
- use named nodes
- produce stable, editable hierarchy

This keeps the AI aligned with the editor instead of fighting it.

## First Contract Shape

The AI adapter should ask the model for a JSON envelope, not raw prose.

Suggested top-level response:

```json
{
  "mode": "full_document",
  "summary": "A playful poster-style scene of a pelican riding a bicycle.",
  "document": { "... full tweaky document ..." },
  "notes": [
    "Used Path nodes for the bicycle frame and pelican beak.",
    "Used Ellipse nodes for the wheels and body masses."
  ]
}
```

Suggested patch shape for later:

```json
{
  "mode": "patch",
  "summary": "Move the title upward and make the wheels larger.",
  "operations": [
    {
      "op": "set_transform",
      "nodeId": "title",
      "x": 220,
      "y": 100
    },
    {
      "op": "replace_params",
      "nodeId": "front_wheel",
      "params": {
        "radiusX": 96,
        "radiusY": 96
      }
    }
  ],
  "notes": []
}
```

## Validation And Repair

The AI adapter must never assume model output is ready to use.

Pipeline:

1. Parse JSON
2. Validate envelope
3. Validate scene document or patch semantics
4. Apply safe repairs
5. Reject with clear feedback if still invalid

### Safe repairs

- fill omitted optional defaults
- normalize obvious enum casing when safe
- inject empty `children`, `style`, or `meta` objects when missing
- clamp absurd numeric values where policy allows it

### Unsafe repairs

Do not silently invent:

- missing required node types
- missing resource refs
- missing path points
- structurally ambiguous transforms

Those should surface as explicit model errors or trigger a retry.

## Editor Integration

The editor-side AI panel should support:

- prompt text box
- mode switch:
  - new scene
  - revise scene
  - revise selection
- preview summary
- apply / cancel

For MVP, preview can simply be:

- generated summary
- node count
- whether raster fallback was used

## Prompt Inputs

The adapter should provide the model with:

### For new-scene generation

- scene vocabulary
- schema rules
- canvas size target
- style guidance
- benchmark prompt if applicable

### For revision

- current document JSON
- selected node ids if any
- concise scene summary
- user edit request

### For selection revision

- selected subtree JSON
- parent context summary
- user edit request

## Benchmark

The first funny benchmark prompt is:

`a drawing of a pelican riding a bicycle`

This is useful because it tests:

- recognizable subject composition
- organic character shapes
- prop interaction
- scene coherence
- whether the model can choose a sane mix of `Ellipse`, `Path`, `Text`, and grouping

The benchmark is not about photorealism.
It is about editable scene quality.

## Benchmark Evaluation

The benchmark output should be judged on:

1. Validity

- parses
- validates
- renders

2. Editability

- nodes are named sensibly
- the bicycle is decomposed into meaningful parts
- the pelican is decomposed into meaningful parts
- wheels / body / beak / text can be adjusted independently

3. Visual coherence

- a human can recognize "pelican riding a bicycle"
- the scene reads clearly at first glance

4. Hybrid discipline

- only use raster fallback where it materially helps
- do not hide basic composition behind one giant image layer

## Recommended MVP Rollout

1. Document the contract
2. Add benchmark examples
3. Build a local mock adapter that reads canned AI JSON
4. Add validation + repair pipeline
5. Add the first editor prompt panel
6. Add a real model backend after the workflow is stable

That keeps us testing the product shape before model plumbing becomes the whole project.
