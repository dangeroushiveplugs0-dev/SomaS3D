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
9. Performance and stress testing

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
