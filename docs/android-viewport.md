# Android viewport

The Android Canvas viewport consumes a scene snapshot from the Rust `soma-geometry` crate through the JNI `soma-android-bridge`.

## Working scene interactions

- Scene state lives in a persistent Rust `Scene` behind a mutex.
- The bridge serializes each mesh object's stable ID, name, world-space positions, polygon indices, and active-object ID.
- `CUBE+` and `SPH+` add a cube or UV sphere to the Rust scene.
- Tap a visible object face to select that object; the active object is highlighted.
- Existing orbit drag, pinch zoom, grid, edge toggle, orientation gizmo, and fit controls remain in place.
- A Kotlin preview cube remains as a fallback if the native library is unavailable.

## Known limits

The viewport still draws with Android Canvas and sorts polygon faces by average depth. It is not yet a GPU renderer, and selection is a screen-space polygon approximation rather than the Rust ray-picking implementation. Editing transforms/topology and saving scene documents are not connected to the UI yet. The next rendering milestone should use GPU depth testing while preserving the camera behavior.

## Build

GitHub Actions builds the Rust bridge for Android `arm64-v8a`, runs `gradle assembleDebug`, and uploads `somas3d-debug-apk`.
