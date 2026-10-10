# Retained Scope: ShofterUI and Blender Import

## Reset decision

This branch starts from the repository's minimal `main` baseline. The previous modeling prototype, including its viewport, primitive creation, selection modes, and Rust modeling core, is not part of the active product scope.

Keep only the requirements in this document and the Blender importer architecture document. Treat this as a clean restart, not a claim that the features below are already implemented.

## 1. ShofterUI

**In-app name:** ShofterUI  
**Documentation name:** SomaS3D Character Shaping System

ShofterUI is an independent character-shaping system. It must not depend on the `.blend` importer; it should work with character data from multiple sources.

### Required input sources

- Imported character assets.
- Characters or shape data created inside SomaS3D.
- AI-generated morphs, outfits, and deformations.

### Core purpose

- Create, organize, apply, and edit character morphs.
- Support outfits and deformation data as part of the character-shaping workflow.
- Keep the underlying data editable instead of flattening everything into a rendered result.
- Define a native representation that can be populated by the Blender importer or other asset sources.

### Architecture constraint

ShofterUI is a consumer and editor of normalized character data. It is not a Blender-specific interface and must remain usable when no Blender file was involved.

## 2. Blender `.blend` importer

The importer converts Blender project data into native, editable Soma data. It must not execute Blender Python add-ons or rely on running Blender itself on the Android device.

Preserve data where supported, and report unsupported features clearly rather than silently pretending the conversion is exact.

### Data preservation targets

- Mesh geometry and topology.
- Per-corner UV data.
- Armatures and bone hierarchy.
- Skinning / vertex weights.
- Shape keys and morph deltas.
- Relevant custom properties and recognized character metadata.
- Materials and textures where their data can be represented faithfully.

### Conversion principle

`.blend` → version-aware reader → normalized intermediate representation → native Soma asset.

The intermediate representation should separate parsing Blender's changing file layouts from the app's own data model. The importer should not be a direct renderer-only path.

### Safety and reliability

- Treat imported files as untrusted binary input.
- Never execute embedded scripts, Blender Python, or add-on code.
- Validate offsets, counts, sizes, references, and allocation limits before reading.
- Detect unsupported Blender versions or structures and return actionable errors.
- Keep parsing separate from conversion and from rendering.

## Explicitly out of scope for this reset

- Reusing the abandoned modeling-core prototype as the foundation.
- Continuing its primitive, viewport-selection, face-editing, or transform roadmap.
- Claiming full Blender compatibility before tests demonstrate it.
- Making ShofterUI depend on Blender or on one specific import route.

## Initial success criteria

1. A clean project baseline builds without the old modeling prototype.
2. ShofterUI's native character-shaping data model is documented before UI implementation.
3. The importer can safely parse a small, version-pinned set of test files.
4. Imported mesh data can be converted into native editable Soma data with explicit capability reporting.
5. Tests cover malformed files and verify that unsupported data is reported rather than silently discarded.
