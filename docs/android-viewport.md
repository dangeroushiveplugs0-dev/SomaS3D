# Android viewport shell

The repository includes a native Android application shell that can be built as a debug APK by GitHub Actions. The viewport remains a small Canvas-based interactive prototype: perspective mesh display, floor grid, XYZ axes, orientation gizmo, orbit drag, pinch zoom, and fit/grid/edge controls.

## Rust geometry bridge

The `soma-android-bridge` crate creates a default scene using `soma-geometry`, requests its validated viewport snapshot, and serializes the mesh positions and polygon indices across JNI. `NativeGeometry.kt` loads the Android shared library and parses that snapshot; `ViewportView` renders the supplied polygons while retaining the existing Kotlin camera and touch controls. A preview cube remains as a graceful fallback if the native library cannot be loaded.

This is the first real geometry connection, not yet the final renderer. Drawing still uses Android Canvas and painter-order face sorting; a GPU-backed renderer, depth testing, multiple scene objects, selection, and editing operations remain future work. Keep the Rust CPU rasterizer as a correctness reference.

## Build

The Android APK workflow compiles the Rust bridge for Android `arm64-v8a`, then runs `gradle assembleDebug` and uploads `app/build/outputs/apk/debug/app-debug.apk` as `somas3d-debug-apk`.
