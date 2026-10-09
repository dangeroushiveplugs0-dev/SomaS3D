# SomaS3D

SomaS3D is a mobile-first 3D modeling application.

## Current foundation

The first engineering milestone is the modeling core:

- editable topology
- per-corner UVs and UV islands
- material/PBR data
- procedural material-property semantics
- validation
- deterministic tests

The modeling core is intentionally independent of Android and the viewport. Rendering, importers, character systems, physics, and ShofterUI will build on this foundation.

See `docs/modeling-foundation.md` for the current architecture and completion criteria.

## Viewport progress

The geometry crate now includes a CPU reference path from scene snapshots through draw buffers and camera projection to RGBA pixels, plus screen-ray mesh picking and camera navigation helpers. This is a testable rendering foundation, not yet an Android app or GPU-accelerated interactive viewport.
