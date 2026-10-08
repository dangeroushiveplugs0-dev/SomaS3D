# Modeling Foundation

SomaS3D is being built as a modeling application first.

## Order of work

1. Editable topology
2. UV representation and editing algorithms
3. Selection
4. Modeling operations
5. Undo/redo
6. Viewport rendering
7. UV editor
8. Texturing and material authoring
9. Performance and stress testing

Other systems are deliberately downstream of this foundation.

## UV rule

UV coordinates are stored per face corner, not per shared vertex. A 3D vertex may therefore have different UV coordinates on different faces at a seam.

This representation supports:

- true UV seams
- separate islands
- mirrored/stacked UVs
- hard texture boundaries
- correct import/export of production meshes

The runtime renderer may flatten indexed topology into GPU vertices when UV or attribute discontinuities require it.

## Texturing and PBR

The material system will support the familiar physically based channels:

- base color
- metallic
- roughness
- normal
- ambient occlusion
- emissive
- opacity

Additional optional maps can be added without changing the core material identity.

SomaS3D will also have a semantic material layer for properties such as:

- wetness
- dryness
- organicness

These are **not** replacement PBR channels. They are higher-level controls that modify standard PBR inputs in a physically interpretable way.

For example, wetness can reduce effective roughness, increase specular response, and optionally darken a material's diffuse response. Dryness can push a surface toward higher roughness. Organicness is intentionally a semantic control rather than a claim that a universal physical "organic" value exists; it can drive a selected material recipe such as subsurface response, roughness variation, and micro-normal detail.

The renderer will eventually evaluate these semantic controls into ordinary PBR values. The authoring model remains explicit so materials can be serialized, inspected, and reproduced.

## Completion gate

The modeling foundation is not considered complete because basic tools exist. It must survive stress tests involving:

- large meshes
- many UV islands
- heavy seam counts
- non-manifold topology
- duplicated/stacked UVs
- invalid UV values
- repeated edit operations

Only after the core passes these tests should character, physics, FEM, or advanced importer work become the primary focus.
