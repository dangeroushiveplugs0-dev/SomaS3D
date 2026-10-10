# Blender `.blend` Importer Architecture

## Status

Design specification only. The current reset branch does not yet contain an implemented `.blend` parser or conversion pipeline.

## Goal

Read Blender project files on-device and convert supported content into native, editable Soma assets. Do not execute Blender scripts, Python add-ons, or arbitrary code stored in the project.

## Pipeline

```text
.blend file
   ↓
File validation and version detection
   ↓
Version-aware SDNA / block reader
   ↓
Blender data extraction
   ↓
Normalized intermediate representation (Soma IR)
   ↓
Capability checks and conversion diagnostics
   ↓
Native editable Soma asset
```

Each stage should be a small module with one responsibility. Keep binary parsing, semantic extraction, normalization, and native asset writing separate.

## Stage 1 — File validation and version detection

- Validate the header and identify the file's Blender version and pointer-size / byte-order details.
- Reject truncated files, invalid block lengths, impossible offsets, and oversized allocations.
- Apply explicit resource limits for mobile devices.
- Keep version-specific behavior behind a small compatibility layer.

## Stage 2 — SDNA and block reader

- Parse the file's block structure and SDNA schema metadata.
- Resolve structure fields through parsed schema information instead of assuming fixed offsets across all Blender versions.
- Use checked arithmetic for every offset, count, and byte-length calculation.
- Never dereference a file-provided pointer as a process pointer.
- Return structured parse errors with location and context.

## Stage 3 — Extract supported data

Implement and test in increments:

1. Object and mesh identity / hierarchy needed to associate data correctly.
2. Mesh positions and polygon topology.
3. Per-corner UV coordinates and material-slot associations.
4. Armatures, bones, and parent relationships.
5. Vertex groups / skinning weights.
6. Shape keys, including basis geometry and per-key deltas.
7. Relevant custom properties and recognized character metadata.
8. Materials and texture references, with documented fidelity limits.

Later support for additional Blender structures must be based on test fixtures and explicit format analysis, not guessed offsets.

## Stage 4 — Normalized Soma IR

The IR should not expose Blender's raw block layout to the rest of the app. It should represent supported concepts in stable, version-independent structures, including:

- Mesh vertices and polygon / corner data.
- UV layers indexed per corner.
- Object hierarchy and transforms.
- Armature and bone hierarchy.
- Vertex weights.
- Shape-key basis and deltas.
- Material / texture references and relevant metadata.
- Import warnings for anything omitted or approximated.

The IR should preserve stable relationships between objects and data blocks, and distinguish absent data from data that failed to parse.

## Stage 5 — Native asset conversion

Convert the IR into the app's native editable asset format (the exact on-disk format remains to be defined). Keep the conversion deterministic and independent from viewport rendering. Preserve source metadata where appropriate and attach a conversion report.

## ShofterUI integration

The importer supplies normalized character and shape data to the same native character-data model used by ShofterUI. The importer must not own ShofterUI behavior, and ShofterUI must also accept assets created by other means.

Shape keys should map to native morph targets where their semantics can be preserved. Unsupported or incompatible shapes must be reported clearly.

## Testing strategy

Build a small, checked-in fixture corpus with known Blender versions and expected outputs. Include:

- A single static mesh.
- Multiple objects and mesh instances.
- Multiple UV layers and per-corner UV seams.
- An armature with weighted mesh.
- Several shape keys with known deltas.
- Relevant custom properties.
- Materials and referenced textures.
- Truncated, malformed, and adversarial files.
- Files from each explicitly supported Blender version.

For every fixture, test both parsed structure and final normalized data. Include resource-limit tests suitable for low-memory Android devices.

## Incremental delivery

1. File header validation and version detection.
2. Block / SDNA reader with unit tests.
3. Static mesh and topology extraction.
4. Per-corner UV preservation.
5. Object hierarchy and transforms.
6. Armature and weights.
7. Shape keys and morph conversion.
8. Metadata and material / texture support.
9. Broader version coverage based on fixtures.

Do not advertise general `.blend` support until the supported-version matrix and fixture tests justify that claim.
