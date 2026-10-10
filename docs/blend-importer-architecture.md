# Blender .blend Importer Architecture

## Status

Stage 1 (header validation) and the first pass of Stage 2 (outer block scanning and SDNA schema reading) are implemented in Kotlin. The Android picker runs the scan on a background thread and reports the result. This is still **not a model importer**: it does not yet resolve object datablocks or extract mesh geometry.

Tests cover header markers and malformed inputs, plus a synthetic 64-bit little-endian block stream with a minimal SDNA schema. CI runs tests before building the debug APK.

## Goal

Read Blender project files on-device and convert supported content into native, editable Soma assets. Do not execute Blender scripts, Python add-ons, driver expressions, or arbitrary code stored in the project.

## Blender version compatibility policy

The intended compatibility floor is **Blender 3.0**, with broad coverage across Blender 3.x and 4.x and later versions added as fixtures and schema behavior can be verified. Do not optimize only for the newest release: downloaded character projects commonly remain on older versions because creator workflows, add-ons, and collaborators do not upgrade in lockstep.

Compatibility is capability-based, not just a version-number check:

- Validate the outer file header and decode SDNA from each file instead of relying on a hard-coded structure layout for one release.
- Keep version-specific differences behind tested schema/layout rules and optional recognizers.
- Maintain real-file fixtures across multiple minor versions, including 3.0-era files and representative later 3.x / 4.x files. Add newer major versions as they are released and can be tested.
- Report the Blender version, recovered capabilities, approximations, and unsupported data for each import.
- A file can be partially useful even when add-on features are unknown; do not reject supported mesh data just because custom rig metadata cannot be interpreted.
- Never claim a version is supported based on its header alone. Support claims require real fixtures and expected normalized outputs.

The current implementation has **not yet demonstrated full Blender 3.0+ compatibility**. The version range above is the project target; the verified support matrix must be updated only after real files from each listed version pass tests.

## Pipeline

.blend file
↓
Header validation and version detection [implemented]
↓
Outer block reader and SDNA schema reader [initial implementation]
↓
Safe address-range indexing and conservative SDNA struct decoding [initial implementation]
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
- `BlendBlockIndex.kt` builds a sorted address-range index, rejects ambiguous overlaps, and refuses zero/overflowing ranges.
- `BlendStructDecoder.kt` computes candidate field offsets from SDNA types/declarations and accepts a layout only when the computed size exactly matches the SDNA TLEN. It can decode bounded primitive/pointer fields and numeric arrays from an already-loaded block payload.
- This decoder is intentionally conservative: pointer-to-array declarations and layouts that do not match TLEN are rejected. Nested struct fields can contribute to layout calculation but are not yet expanded into nested decoded values.
- The scanner still skips non-DNA payloads; the payload retrieval/import pipeline must be added before these modules can decode a selected real datablock end-to-end.
- Resource limits and malformed-file handling need continued fuzzing and real-file tests.

## Current implementation status

The importer now has a cache-backed decoding path:

1. Copy the selected Android document to a bounded temporary cache in chunks (current hard cap: 1 GiB).
2. Scan block headers and retain the SDNA schema while recording each block's payload offset.
3. Read individual block payloads on demand with per-payload limits and header consistency checks.
4. Decode selected datablock records using the SDNA structure table.
5. Use the stored-address index only for file-format address resolution, never as native pointers.
6. The current UI probes blocks whose SDNA type is `Object` or `Mesh` and reports decoded versus unsupported counts.

This is still a generic record-decoding stage, **not yet semantic mesh extraction or viewport import**. SDNA layout validation deliberately fails closed when computed layouts do not match declared TLEN sizes. Some real Blender ABI/layout cases may need explicit, fixture-backed rules before their records can be decoded.

## Stage 3 — Datablock linking and static mesh extraction

Next:

1. Use `BlendBlockIndex` to resolve a stored file address to a non-overlapping block range.
2. Add bounded payload retrieval from a seekable or cache-backed file source; the current scanner deliberately skips non-DNA payloads.
3. Decode supported structures through `BlendStructDecoder`, accepting only layouts validated against SDNA TLEN.
4. Identify objects and their mesh data using decoded datablock relationships.
5. Extract vertex positions and polygon topology.
6. Add a minimal mesh renderer and fit the camera to imported bounds.

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
3. Safe address-range indexing and conservative SDNA layout decoding — initial modules and synthetic tests added.
4. Payload retrieval and real-file validation across supported Blender versions.
5. Datablock linking and static mesh extraction.
5. Minimal mesh renderer and camera framing.
6. Per-corner UVs and object transforms.
7. Armature and skinning weights.
8. Shape keys and morph conversion.
9. Custom properties, drivers, control bones, and constraint mappings.
10. Materials, textures, and broader version coverage.

Do not advertise general .blend support until the supported-version matrix and fixture tests justify that claim.

## Extended character-project recovery

Treat a `.blend` as a structured project, not merely a mesh container. SDNA describes how structures are laid out; actual values and relationships live in datablock payloads. Add-on-specific features should be discovered through generic datablock decoding first, then interpreted by optional recognizers.

### Additional data families to preserve

- **Physics and collision setup:** rigid/soft-body settings, cloth settings, collision modifiers, collision-proxy objects, parent links, and relevant custom properties. Preserve numeric parameters and object relationships even when Soma has no matching runtime simulation yet.
- **Custom control-panel metadata:** MustardUI-like panel names, sections, labels, toggles, numeric fields, property paths, and defaults where stored in custom properties or other datablocks. Recreate a native ShofterUI panel from recognized metadata; unknown layouts remain inspectable metadata.
- **Outfit and geometry toggles:** drivers or properties that control object visibility, collection visibility, modifier enablement, and alternative mesh variants. Convert recognized boolean/enum mappings into native toggles without running the original driver code.
- **Morph and corrective deformation systems:** shape keys, lattice/modifier parameters, and pose-space or bone-angle relationships. Preserve source relationships and ranges; only implement deterministic mappings that can be represented safely in the native rig model.
- **Embedded text and add-on data:** enumerate text datablocks and recognized add-on metadata as untrusted project content. Never auto-run scripts, auto-run rig UI files, install add-ons, or execute expressions. Keep script text inert and optionally report its name and presence to the user.
- **External and plugin-specific origins:** retain generic object, armature, vertex-group, material, texture, and custom-property data even when a file originated from XNALara/XPS, Cats, Diffeomorphic/DAZ, Auto-Rig Pro, Wiggle Bones, Jiggle Gen, or another add-on. Do not require the original add-on to read core Blender data.

### Implementation rule

Build a **generic decoder + normalized Soma IR + recognizer registry**:

1. Generic decoder interprets datablocks using the file's SDNA schema and safe pointer/reference resolution.
2. The IR stores known Blender concepts plus typed unknown/custom metadata, with source IDs and relationships.
3. Recognizers identify known add-on conventions or driver/constraint patterns without executing code.
4. A capability mapper converts supported data into native Soma/ShofterUI controls and runtime features.
5. The import report labels each feature as supported, approximated, preserved-but-inactive, or unsupported, with a reason.

Do not assume every downloaded character file contains these features or that a particular add-on's data always uses the same property names. Add-on conventions vary by version. Confirm recognizers against legally obtained test fixtures and anonymized structural samples before claiming support.

### Expanded fixture coverage

Add fixtures for physics settings and collision proxies, custom UI metadata, object/collection and modifier toggles, corrective shape-key relationships, and embedded script/text datablocks. Assert that scripts remain inert and that unknown properties survive round-tripping through the IR where feasible.


## Normalized mesh foundation

`SomaMeshIR.kt` now defines Blender-independent vertex, polygon, mesh, and import-result types. `BlendMeshExtractor.kt` converts already-decoded MVert/MLoop/MPoly-style records into that representation, with checks for non-finite coordinates, out-of-range loop spans, invalid vertex indices, and conservative topology limits. Synthetic unit tests cover a valid triangle and malformed vertex data.

This is deliberately only the normalization boundary. The extractor does not yet resolve Mesh pointers to their vertex/loop/polygon datablocks, and no imported mesh is rendered in the viewport. The next implementation step is a bounded, type-checked datablock linker that resolves stored file addresses to records, followed by fixture-backed tests against real `.blend` files from Blender 3.x and 4.x.


## Direct pointer linking (initial legacy-style recognizer)

`BlendDatablockDecoder.decodeRecordsAtAddress` now resolves a stored address only when it lies within an indexed block, aligns to the expected SDNA record size, matches the expected type, and stays within the declared count and payload bounds. `BlendMeshDatablockLinker.kt` uses this for direct `Mesh.mvert`, `Mesh.mloop`, and `Mesh.mpoly` pointers when those fields exist and decode cleanly. The Android inspection screen reports how many meshes normalize and how many are skipped.

This recognizer is intentionally not a universal Blender mesh reader. Files whose Mesh geometry is represented through newer or different CustomData layouts will be reported as unsupported by this path rather than guessed. Synthetic address-resolution tests cover alignment and type checks; real Blender fixture validation is still required before claiming import support.
