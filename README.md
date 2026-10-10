# SomaS3D

SomaS3D is restarting from a clean baseline. Its two product systems are:

1. **ShofterUI** — a standalone character-shaping system for morphs, outfits, and deformations.
2. **Blender .blend importer** — a staged conversion pipeline that translates Blender project data into native, editable Soma data.

A small Android viewport and orbit camera are being built as shared test infrastructure for both systems. The old modeling-core prototype is not the foundation for this restart.

## Current foundation

- Perspective viewport with ground grid, world axes, and origin.
- Independent orbit camera with drag-to-orbit, pinch-to-zoom, and reset.
- .blend document picker shell; selecting a file is not yet importing it.
- ShofterUI placeholder; character-shaping data integration is not yet implemented.

## Design documents

- [Retained scope](docs/retained-scope.md)
- [Viewport and camera foundation](docs/viewport-camera-foundation.md)
- [Blender importer architecture](docs/blend-importer-architecture.md)

## Naming

Use **ShofterUI** in the app. Use **SomaS3D Character Shaping System** only in documentation.
