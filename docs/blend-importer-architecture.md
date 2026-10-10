# Blender .blend Importer Architecture

## Status

Stage 1 (header validation) and the first pass of Stage 2 (outer block scanning and SDNA schema reading) are implemented in Kotlin. The Android picker runs the scan on a background thread and reports the result. This is still **not a model importer**: it does not yet resolve object datablocks or extract mesh geometry.

Tests cover header markers and malformed inputs, plus a synthetic 64-bit little-endian block stream with a minimal SDNA schema. CI runs tests before building the debug APK.

## Goal

Read Blender project files on-device and convert supported content into native, editable Soma assets. Do not execute Blender scripts, Python add-ons, driver expressions, or arbitrary code stored in the project.

## Pipeline

.blend file
↓
Header validation and version detection [implemented]
↓
Outer block reader and SDNA schema reader [initial implementation]
↓
Datablock linking and object / mesh extraction [next]
↓
Normalized intermediate representation (Soma IR)
↓
Capability checks and conversion diagnostics
↓
Native editable Soma asset

Each stage should be a small module with one responsibility. Keep binary parsing, semantic extraction, normalization, and native asset writing separate.

## Stage 1 — File validation and version detection

Implemented in BlendFileInspector.kt:

- Reads only the fixed 12-byte header.
- Checks the BLENDER signature.
- Detects 32-bit / 64-bit pointer markers, byte-order marker, and three-digit Blender version.
- Reports file size when the document provider exposes it.
- Rejects short headers, invalid signatures, and malformed header markers.
- The UI calls this result **header validated**, not **imported**.

## Stage 2 — Outer blocks and SDNA

Initial implementation in BlendBlockReader.kt:

- Reads the file header and sequential block headers using the detected pointer width and byte order.
- Records block code, payload length, stored address, SDNA index, and element count.
- Reads only the DNA1 payload into memory, subject to a 64 MiB limit; other payloads are skipped using a small buffer.
- Parses SDNA NAME, TYPE, TLEN, and STRC sections into schema names, type lengths, structures, and field references.
- Detects the ENDB terminator and reports truncated or malformed streams.
- Applies a 256 MiB per-block limit and a block-count limit.
- Runs off the UI thread.

Important limitations:

- This is an initial reader, not yet validated against a corpus of real Blender files across versions.
- It does not yet resolve stored datablock addresses into object relationships or use the SDNA schema to decode arbitrary block payloads.
- It does not extract meshes, armatures, shape keys, drivers, or constraints yet.
- 64-bit stored addresses are retained only as numeric metadata; they are never dereferenced as process pointers.
- Resource limits and malformed-file handling need continued fuzzing and real-file tests.

## Stage 3 — Datablock linking and static mesh extraction

Next:

1. Resolve datablock references through a safe address-to-block index.
2. Decode supported structures using SDNA field names and type sizes, never guessed fixed offsets.
3. Identify objects and their mesh data.
4. Extract vertex positions and polygon topology.
5. Add a minimal mesh renderer and fit the camera to imported bounds.

Use checked arithmetic for offsets, counts, and byte lengths. Do not allocate arrays directly from unchecked file-provided counts.

## Stage 4 — Preserve character data

Implement and test in increments:

1. Object identity, hierarchy, and transforms.
2. Mesh positions and polygon topology.
3. Per-corner UV coordinates and material-slot associations.
4. Armatures, bones, and parent relationships.
5. Vertex groups / skinning weights.
6. Shape keys, including basis geometry and per-key deltas.
7. Relevant custom properties and recognized character metadata.
8. Materials and texture references, with documented fidelity limits.

## Rigging controls and ShofterUI

Blender controls must become native, inspectable data, not executable Blender behavior. The normalized model is defined in RigControlModel.kt.

Preserve, when successfully decoded:

- Object and bone custom properties, including names, types, defaults, and limits.
- Driver source / target relationships, target data paths, channels, and expression text as inert metadata.
- Control-bone transform sources and their axis / space information.
- Action-constraint and transformation-constraint identities, targets, and relevant ranges.
- Links from controls to shape keys, transforms, and other supported parameters.

ShofterUI should expose supported properties as sliders, numeric inputs, toggles, or other suitable controls. Moving a supported control bone should map to a native parameter using explicit, bounded conversion rules. An Action Constraint should be represented as a relationship and range mapping where feasible, not as an executed Blender animation graph.

Security and fidelity rules:

- Never evaluate imported driver expression text or execute embedded scripts.
- Do not assume every Blender driver expression can be converted exactly.
- Mark each link as natively supported, approximated, or unsupported, and explain losses in the import report.
- Preserve unknown custom properties where practical rather than silently discarding them.

## Stage 5 — Normalized Soma IR

The IR should not expose Blender's raw block layout to the rest of the app. It should represent supported concepts in stable, version-independent structures, including:

- Mesh vertices and polygon / corner data.
- UV layers indexed per corner.
- Object hierarchy and transforms.
- Armature and bone hierarchy.
- Vertex weights.
- Shape-key basis and deltas.
- Custom properties and rig-control definitions / links.
- Material / texture references and relevant metadata.
- Import warnings for anything omitted or approximated.

Distinguish absent data from data that failed to parse.

## Stage 6 — Native asset conversion

Convert the IR into the app's native editable asset format (the exact on-disk format remains to be defined). Keep conversion deterministic and independent from viewport rendering. Preserve source metadata where appropriate and attach a conversion report.

## Testing strategy

Build a checked-in fixture corpus with known Blender versions and expected outputs. Include:

- A single static mesh.
- Multiple objects and mesh instances.
- Multiple UV layers and per-corner UV seams.
- An armature with weighted mesh.
- Several shape keys with known deltas.
- Custom properties and drivers linking properties to shape keys.
- A control bone mapped by a transformation driver or constraint.
- An action constraint with known input range.
- Materials and referenced textures.
- Truncated, malformed, and adversarial files.
- Files from each explicitly supported Blender version.

For every fixture, test parsed structure and final normalized data. Include resource-limit tests suitable for low-memory Android devices.

## Incremental delivery

1. Header validation — implemented and unit tested.
2. Outer block scanning and initial SDNA parsing — implemented; synthetic test only.
3. Real-file validation across supported Blender versions.
4. Datablock linking and static mesh extraction.
5. Minimal mesh renderer and camera framing.
6. Per-corner UVs and object transforms.
7. Armature and skinning weights.
8. Shape keys and morph conversion.
9. Custom properties, drivers, control bones, and constraint mappings.
10. Materials, textures, and broader version coverage.

Do not advertise general .blend support until the supported-version matrix and fixture tests justify that claim.
