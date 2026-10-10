# Viewport and Camera Foundation

## Purpose

The viewport is shared infrastructure for testing imported Blender assets and eventually editing characters with ShofterUI. It is not a commitment to continue the previous modeling prototype.

## Current implementation

- Native Android custom-drawn perspective viewport.
- Ground grid on the X/Z plane with world Y up.
- Red X, green Y, and blue Z axis cues, plus a marked world origin.
- Standalone `OrbitCamera` state and perspective projection.
- One-finger orbit and pinch-to-zoom controls.
- Reset-camera action.
- Android document picker that can select a `.blend` file and display its name.

## Current limitations

- The viewport currently draws only the grid, axes, and origin; it does not yet render imported meshes.
- Selecting a file is not parsing it. The actual `.blend` reader and conversion pipeline remain unimplemented.
- The ShofterUI button is a placeholder and does not yet edit character data.
- Camera pan, object framing, orthographic views, and production rendering are not implemented yet.

## Integration rules

1. Keep camera state independent of imported asset data.
2. Imported geometry must not reset camera orbit or zoom unless the user explicitly requests framing/reset.
3. The importer should produce normalized native scene data, not issue viewport drawing commands.
4. ShofterUI should consume the same normalized character data model as other asset sources.
5. Keep camera math, viewport drawing, file parsing, and character-shaping code in separate modules.
