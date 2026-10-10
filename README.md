# SomaS3D

SomaS3D is restarting from a clean baseline. Its two product systems are:

1. **ShofterUI** — a standalone character-shaping system for morphs, outfits, and deformations.
2. **Blender .blend importer** — a staged conversion pipeline that translates Blender project data into native, editable Soma data.

A small Android viewport and orbit camera are shared test infrastructure for both systems. The old modeling-core prototype is not the foundation for this restart.

## Current foundation

- Perspective viewport with ground grid, world axes, and an independent orbit camera.
- .blend picker validates the 12-byte Blender header.
- A bounded sequential reader scans outer block headers, reads the DNA1 schema, and checks for the ENDB terminator without loading the entire project into memory.
- Block scanning runs off the UI thread and reports block count, Blender version, and SDNA schema counts.
- Limits guard individual block sizes, schema size, and block count.
- Normalized rig-control models preserve the design for custom properties, drivers, control-bone transforms, action constraints, and transformation constraints.
- Driver expressions are stored as metadata only; the importer never executes Blender scripts or expressions.
- Unit tests cover header validation and synthetic block/SDNA scanning.
- **Not implemented yet:** object/mesh extraction, visible mesh rendering, rig-control extraction from real .blend files, and ShofterUI editing.

## Design documents

- [Retained scope](docs/retained-scope.md)
- [Viewport and camera foundation](docs/viewport-camera-foundation.md)
- [Blender importer architecture](docs/blend-importer-architecture.md)

## Naming

Use **ShofterUI** in the app. Use **SomaS3D Character Shaping System** only in documentation.
