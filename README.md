# SomaS3D

SomaS3D is a mobile-first Android foundation for **ShofterUI** and safe Blender project import.

## Current importer pipeline

- Validates Blender headers, including supported legacy headers and modern Blender 5.x header markers.
- Detects and decompresses Zstandard-compressed .blend streams before scanning.
- Reads block headers and SDNA schemas using bounded streaming and a disk-backed cache.
- Decodes selected datablocks with SDNA-derived layouts and address-range validation.
- Extracts legacy direct-array mesh geometry where the file schema supports it.
- Attempts the explicitly recognized Blender 4.0+ CustomData path for vertex positions, corner-to-vertex indices, and face offsets.
- Links mesh datablocks to scene objects by stored file address and applies object transforms.
- Imports shape-key coordinate sets, values, and relative-key index metadata; attaches shape keys to normalized mesh instances and applies object transforms to their coordinates.
- Imports armature bone names, parent relationships, head/tail coordinates, and roll metadata when the relevant records are decodable.
- Catches import failures and reports warnings instead of intentionally crashing the app.
- Keeps imported driver expressions and script-like data inert; it does not execute Blender Python or evaluate drivers.

## Compatibility policy

The target is Blender **3.0 through 4.x and later**, using the file's SDNA schema and conservative version-specific recognizers rather than assuming every release has the same layout. Blender 4.0+ CustomData geometry is attempted only when the schema and layer checks pass. Unknown layouts are reported as unsupported.

This is not a claim of complete compatibility with every Blender version or every character feature. Real-file validation is still required, and the current viewport displays base mesh geometry; a ShofterUI editor for interactively changing shape-key values and rig controls is not yet complete.

## Current limits

- Shape keys are preserved as coordinate sets but are not yet exposed as live morph sliders in the viewport.
- Armature import currently preserves basic bone hierarchy/geometry metadata; full pose evaluation, vertex weights/skinning, constraints, actions, and driver behavior are not implemented as a full Blender rig runtime.
- UV layers, materials, texture packing, and complex node graphs need additional version-aware extraction and fidelity tests.
- A malformed or unsupported structure may be skipped with warnings rather than imported.

## Design documents

- [Retained scope](docs/retained-scope.md)
- [Viewport and camera foundation](docs/viewport-camera-foundation.md)
- [Blender importer architecture](docs/blend-importer-architecture.md)

## Naming

Use **ShofterUI** in the app. Use **SomaS3D Character Shaping System** only in documentation.
