# Modeling Foundation

SomaS3D is being built as a modeling application first.

## Order of work

1. Editable topology and adjacency
2. UV representation, seams, island detection, and editing algorithms
3. Selection
4. Fundamental modeling operations
5. Undo/redo
6. Viewport rendering
7. UV editor
8. Texturing and material authoring
9. Node-based shader/material editor
10. Toon/cel shading and outline rendering
11. Performance and stress testing

The PBR material data model and semantic material evaluator are already part of the initial foundation. Rich texture image management and authoring tools remain downstream of stable topology and UV behavior.

## Face deletion

- Removing a face rebuilds edge adjacency and removes edges no longer used by any face.
- Per-corner UV values are remapped with surviving face IDs; deleted-face UV values are discarded.
- Existing seam flags are preserved for edges that survive. Vertex IDs remain unchanged.
- Because face and edge IDs are compact vector indices, callers must apply the returned `TopologyRemap` to selections and other ID-based data. `Selection::apply_topology_remap` updates selected surviving edges/faces and drops deleted components.

## Transform primitives

- `Transform3D` supports translation, XYZ Euler rotation in radians, scale, and a shared pivot.
- `Mesh::transform_vertices` validates and applies the selected-vertex transform atomically. This is geometry-core functionality for future viewport move/rotate/scale gizmos; it is not a viewport or interactive gizmo implementation.

## Vertex editing

- Multi-vertex position changes validate every vertex ID and every coordinate before changing any position.
- Non-finite positions and duplicate IDs in one update are rejected atomically.
- Translation uses the same validation path, so overflow to infinity cannot partially move a selection.

## Topology and UV rules

- `Mesh::validate_topology` reports invalid vertex positions, broken edge/face references, missing edges, duplicate edges, and non-manifold edges without silently repairing user data. Non-manifold edges are diagnostics, not automatically treated as fatal corruption.

- UV coordinates are stored per face corner, not per shared vertex. A 3D vertex may have different UV coordinates on different faces at a seam.
- Edges track incident faces and an explicit seam flag.
- UV islands are connected components across manifold edges only when the edge is not marked as a seam and both endpoint UVs agree within a small tolerance.
- Non-manifold edges are conservatively treated as UV island boundaries.
- UV coordinates may lie outside the 0–1 square, supporting tiled and UDIM-style layouts.
- The runtime renderer may flatten indexed topology into GPU vertices when UV or attribute discontinuities require it.

## Texturing and PBR

The material system supports familiar physically based channels:

- base color
- metallic
- roughness
- normal
- ambient occlusion
- emissive
- opacity

Additional optional maps can be added without changing the core material identity.

SomaS3D also has a semantic material layer for properties such as wetness, dryness, and organicness. These are higher-level controls, not replacement PBR channels. The evaluator currently maps wetness and dryness into bounded roughness changes; organicness remains an authoring semantic until a well-defined material recipe uses it.

## Completion gate

The modeling foundation is not considered complete because basic tools exist. It must survive stress tests involving large meshes, many UV islands, heavy seam counts, non-manifold topology, duplicated/stacked UVs, invalid UV values, and repeated edit operations.

Only after the core passes these tests should character, physics, FEM, or advanced importer work become the primary focus.

## Face extrusion

- `Mesh::extrude_face` replaces the selected face with a new cap and one side quad per source edge, duplicating its vertices and returning the created IDs plus a remap for neighboring faces and surviving edges.
- The cap copies available per-corner UV coordinates from the source face. Side UVs use a predictable world-unit rectangle (source-edge length by extrusion distance); this is a starting policy for later interactive UV refinement, not automatic unwrap.
- Non-finite offsets, overflowed positions, invalid face IDs, and non-finite derived lengths are rejected before topology changes.
- This is a core mesh operation; interactive extrusion handles, live preview, and undo/redo are still future layers.

## Undo and redo foundation

- `EditHistory<T>` provides bounded snapshot-based undo/redo for cloneable editor state. It can hold a combined state containing the mesh and selection so they can be restored together.
- Edits run against a cloned candidate and commit only when the operation returns success. Failed edits leave current state and both history stacks untouched; a successful new edit clears redo history.
- The history limit bounds the number of snapshots, and a zero limit disables undo storage while still applying edits.
- This is the correctness-first foundation. Large meshes may make full snapshots expensive, so operation-specific deltas or copy-on-write storage should be introduced after measuring actual workloads. Viewport input, drag previews, and app-level persistence still need integration.


## Editor command integration

- `ModelingEditor` owns a combined `EditorState` containing the mesh and component selection, coordinated through `EditHistory`.
- Selected-vertex transforms, selected-face deletion, and single-face extrusion run transactionally and can be undone/redone.
- Extrusion remaps neighboring selections and selects the new cap. Face deletion remaps surviving face/edge selections as topology IDs compact.
- Tests verify that undo/redo restores geometry and selection together and that rejected commands leave the document unchanged.
- Selection changes currently use the same snapshot history for simple, deterministic behavior. A future viewport may choose a separate transient selection channel if selection should not consume undo steps.


## Connected component selection

- Connected-face selection traverses adjacent faces through manifold edges. It stops at boundaries and non-manifold edges to avoid ambiguous branching.
- Connected-edge selection traverses edges that share a vertex, producing the complete connected edge component.
- Both operations add to the current selection and are undoable through the editor history.
- Tests cover adjacent faces, connected edge loops, and restoring selection through undo/redo.

## Primitive system and procedural hair (planned)

Primitives should be first-class modeling objects, not just a fixed menu of cube/sphere meshes. Standard geometric primitives can be added alongside specialized procedural primitives such as hair. Hair is generated from selected surface faces, so the user can select a scalp, armpit, or other growth region and choose a hair preset.

### Hair creation workflow

1. Select one or more faces on the target surface.
2. Choose **Hair**, then choose **Flowing Hair** or **Short Hair** (the latter is the short, dense preset intended for body hair).
3. Open a movable floating tool panel with live controls and a close/dock action. Closing the floating panel docks it into the normal SomaS3D tool panel; it must not discard the current hair settings.
4. Preview changes before committing the generated hair, and allow the user to reopen the panel to edit the hair object later.

### Shared controls

- **Length:** maximum strand length, with a much larger supported range for Flowing Hair and a deliberately short range for Short Hair.
- **Curl:** controls strand curvature.
- **Color:** hair color, independent from the surface material.
- **Gravity:** a normalized slider, not a raw numeric field. Low values bias strands upward; high values bend them downward. Strand roots remain attached to the selected surface.
- **Seed:** a deterministic random seed so the same settings can be regenerated consistently.
- **Density / Amount:** available for Short Hair, controlling strand count over the selected area. A sensible preset is the default; users can reduce or increase it within device-safe limits.

### Performance and rigging strategy

Do not create and independently draw a heavy, fully segmented mesh for every strand. Keep a compact procedural description and a limited set of guide curves, derive nearby strands from those guides, and render hair in batched strand/ribbon or clump geometry. This retains a controllable root and strand identity for future rigging while reducing per-strand object and draw-call overhead.

The proposed merge ratios are optimization targets, not a guarantee that arbitrary strands can be fused without visual loss: for Short Hair, group roughly two strands per clump; for Flowing Hair, group up to four strands per render clump. Preserve the underlying guide strands and stable root data so clumps can still follow skin deformation. Build clumps in batches and use level-of-detail limits on mobile rather than destructively merging the authored strands. Users should be able to choose a lower-density preview and a higher-quality final render.

### Safety and correctness rules

- Growth roots must remain on the selected faces and orient from their surface normals, with optional root direction and combing added later.
- Hair generation must be deterministic for a given mesh, face selection, and settings.
- A generation preview should be cancellable and should not mutate the base mesh until committed.
- Hair should be stored as its own editable procedural object linked to the source surface, not baked irreversibly into the body's topology. This keeps styling, recoloring, density changes, and later rig binding possible.
- Short Hair should use a clamped, explicitly documented length range; the exact limits should be tuned with real viewport testing rather than guessed now.
- Floating/docked panel behavior belongs to the future editor UI layer; this geometry repository documents the interaction contract but does not claim to implement that UI yet.

## Geometric primitive generation

- `generate_primitive` builds centered cube, plane, and UV-sphere meshes through a UI-independent API.
- Primitive dimensions must be finite and positive. UV-sphere segments and rings are validated, and each resolution is capped at 256 for mobile safety.
- Generated faces share topology vertices and edges instead of duplicating a separate vertex set for every face.
- Cube and plane primitives receive basic per-corner UV coordinates; UV spheres receive spherical per-corner coordinates with the longitudinal seam represented by UVs outside the 0–1 interval where needed. These are usable starting maps, not a substitute for a dedicated UV editor. Viewport-facing creation controls and explicit normal data remain follow-up work.
- Procedural hair will use a separate editable-object representation rather than being forced into these ordinary closed-surface mesh builders.


## Editable primitive objects

- `PrimitiveObject` stores a caller-assigned stable object ID, a display name, the source `PrimitiveKind` parameters, the generated mesh, and a geometry revision.
- Updating primitive parameters generates a candidate mesh before committing. Invalid dimensions or resolution leave the previous parameters, mesh, and revision untouched.
- Identical parameter updates are no-ops; renaming does not regenerate geometry. The geometry revision increments only after successful parameter changes, so future viewport caches can identify stale mesh data.
- The caller supplies `PrimitiveObjectId`; the future scene/document layer is responsible for allocating unique IDs and storing objects. This avoids global counters and keeps object identity deterministic in tests and serialized documents.
- The mesh is exposed read-only through this object API to prevent parameter/mesh drift. Direct mesh editing remains available through the existing `Mesh` and `ModelingEditor` APIs; a future scene layer will define how an object transitions from parametric editing into ordinary topology editing.
- This is a UI-independent data contract. It does not yet implement a scene graph, object transforms, primitive creation panels, viewport gizmos, persistence, or procedural hair.

## Scene and object layer

- `Scene` owns a list of scene objects, allocates monotonically increasing object IDs, and tracks the active object. Removing the active object clears active state; IDs are not reused.
- Each `SceneObject` stores local-space geometry separately from its `Transform3D`, keeping object transforms independent from component-level mesh edits.
- Scene primitives retain their `PrimitiveKind` and geometry revision. Parameter updates generate a replacement mesh before committing and preserve object identity and object transform.
- `make_editable_mesh` converts a parametric primitive into ordinary mesh geometry without changing its current geometry, object ID, name, or transform. Subsequent parameter updates are rejected rather than silently overwriting direct topology work.
- The scene is a platform-independent document foundation, not a rendered viewport. Picking, hierarchy UI, transform gizmos, serialization, duplication, and scene-level undo/redo remain later integrations.

- `SceneObject::evaluated_mesh` returns a transformed mesh copy for viewport/render consumers while leaving stored local-space geometry untouched. Invalid object transforms return an error instead of partially changing scene geometry.


## Node-based shader editor roadmap

SomaS3D should support a visual, node-based material editor inspired by Blender's Shader Editor. This is a planned authoring system, not an implemented UI yet.

### Core graph design

- Represent materials as a typed graph of nodes and sockets rather than a collection of hard-coded UI presets.
- Start with output, constant/color, numeric value, texture sample, UV coordinates, basic math/mix, normal, and a physically based surface shader node.
- Validate socket types and graph connections; reject cycles where the evaluation model cannot support them, missing required inputs, and invalid numeric values without corrupting the previous working material.
- Keep graph data separate from the renderer so graphs can be saved, inspected, tested, and later compiled to the mobile GPU backend.
- Provide a simple material-properties view and ready-made presets alongside the graph editor. Nodes should be optional for users who only want quick material controls.

### Toon / cel shading target

The supplied reference is a useful visual target: a stylized character with clean, deliberate light bands rather than photorealistic shading. A dedicated Toon/Cel Surface node should expose:

- Base color and optional color texture
- Shadow, midtone, and highlight colors or thresholds
- Number of light bands and band softness
- Shadow strength and ambient-light contribution
- Optional rim-light color and intensity
- Optional specular highlight controls

The renderer will need a compatible lighting path to quantize diffuse lighting into stable bands. An outline is a separate rendering feature (for example, an inverted-hull or screen-space outline), so it should be an optional material/render setting rather than falsely treated as something a basic color node can provide. A practical first version should target predictable real-time mobile performance, then add more advanced nodes and effects after profiling on-device.

### Suggested delivery order

1. Define serializable node, socket, connection, and graph data types with stable IDs.
2. Add graph validation and deterministic CPU-side evaluation for supported simple nodes.
3. Implement a minimal graph-to-renderer compilation path and default PBR material compatibility.
4. Add the Toon/Cel Surface node and light-band rendering.
5. Add optional outline and rim-light support, then build the touch-friendly node canvas, searchable node menu, and material preview.
6. Add texture nodes and advanced graph features incrementally, with mobile performance budgets and test coverage.

The reference image is a shading goal, not a promise that the current geometry foundation already renders toon shading. The shader graph belongs after the scene/document and rendering interfaces are stable enough to consume it.


## Scene-aware modeling commands

- `SceneModelingEditor` routes supported mesh commands to the active scene object while keeping a separate `ModelingEditor` history and component selection for each object.
- Mesh edits commit back to the object's local-space geometry. The object's ID and object-level transform remain unchanged, and the first direct topology/vertex edit converts a parametric primitive to ordinary editable mesh geometry.
- Undo and redo synchronize the selected object's restored mesh back into the scene. Switching active objects does not discard the other object's edit history.
- Current wrapper commands include component selection, selected-vertex transforms, face extrusion, face deletion, and per-object undo/redo. Scene creation/removal and object transforms are not yet part of the mesh undo stack; those require a document-level transaction history.


## In-space drawing, text, and render effects (planned)

SomaS3D should include a native **In-Space Studio** for drawing and placing graphic elements directly inside the 3D scene, so creators can produce a finished composition without exporting a render to a separate drawing app.

### Spatial creation tools

- **3D drawing:** draw freehand strokes in a chosen plane, on a model surface, or facing the camera. Support adjustable brush size, color, opacity, smoothing, pressure when available, and erase/undo.
- **3D text:** place text in the scene with font, size, alignment, extrusion/bevel options where supported, color/material, and controls to orient it toward the world, a selected surface, or the camera.
- **2D overlay layers:** add screen-facing annotations, captions, graphic marks, frames, and decorative elements that belong to the saved scene/render composition rather than being painted onto the exported image afterward.
- **Effects:** provide non-destructive glow, neon strokes, particles/sparkles, trails, light streaks, depth-aware blur, and color grading as staged features. Each effect should expose practical presets and a small set of controls.
- **Composition controls:** layers, visibility, ordering where relevant, lock, duplicate, group, opacity, and per-element transforms. Distinguish world-space elements from camera-facing overlays in the UI.
- **Capture workflow:** a final-render mode should include these elements in the render and export a clean image directly. Provide a toggle to hide editor gizmos, selection outlines, and guides before capture.

### Data and rendering architecture

- Store strokes as editable curve/control-point data where possible, with a separate render representation. Do not bake strokes or text destructively into the base mesh.
- Store text as editable text plus typography/layout settings; convert to mesh only when the user explicitly requests geometry or an effect requires it.
- Keep camera-facing graphics separate from world-space geometry so they can remain crisp and readable without unexpectedly moving with the camera.
- Effects should be non-destructive, budgeted for mobile GPUs, and degrade gracefully through quality presets. Preview quality and final-render quality can differ.
- Make spatial strokes selectable and, later, optionally riggable or attached to an object/surface. Surface-attached strokes should retain stable attachment data instead of relying only on fixed world coordinates.

### Delivery order

1. Add document data types for strokes, text objects, overlay layers, and effect settings.
2. Add basic drawing planes/surface projection, editable stroke selection, transforms, and undo/redo.
3. Add editable text placement and camera-facing overlay elements.
4. Add a render-composition pass that includes these elements and supports clean image export.
5. Add mobile-friendly effect presets incrementally, profiling on-device before enabling expensive effects by default.

This is a roadmap item, not a claim that spatial drawing, text, effects, or image export are implemented today.
