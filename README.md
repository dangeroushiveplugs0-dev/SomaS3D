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

The geometry crate remains independent of Android UI. Rendering, importers, character systems, physics, and ShofterUI will build on this foundation.

See `docs/modeling-foundation.md` for the geometry architecture and `docs/android-viewport.md` for the Android prototype.

## Viewport progress

The Rust geometry crate includes a CPU reference path from scene snapshots through draw buffers and camera projection to RGBA pixels, plus screen-ray mesh picking and camera navigation helpers.

A native Android app shell has now been added with a perspective cube preview, floor grid, XYZ axes/orientation gizmo, drag-to-orbit, pinch-to-zoom, and fit/grid/edge controls. GitHub Actions builds a debug APK artifact. The Android preview is currently a Kotlin Canvas prototype and is not yet connected to the Rust scene or a GPU renderer; this is the launch-and-touch foundation for the next integration milestone.
