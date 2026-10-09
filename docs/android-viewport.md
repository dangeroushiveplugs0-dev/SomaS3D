# Android viewport shell

The repository now includes a native Android application shell that can be built as a debug APK by GitHub Actions. The viewport is a deliberately small Canvas-based interactive prototype: a perspective cube, floor grid, XYZ axes, orientation gizmo, orbit drag, pinch zoom, and fit/grid/edge controls.

This milestone proves the Android launch and touch/UI path, not GPU performance or integration with the Rust modeling scene. The cube is a preview primitive drawn by Kotlin and is not yet sourced from soma-geometry. The next renderer milestone should replace this preview with a GPU-backed renderer and bridge the Rust scene snapshot/draw buffers into it. Keep the CPU rasterizer as a correctness reference.

## Build

The Android APK GitHub Actions workflow runs gradle assembleDebug and uploads app/build/outputs/apk/debug/app-debug.apk as somas3d-debug-apk.
