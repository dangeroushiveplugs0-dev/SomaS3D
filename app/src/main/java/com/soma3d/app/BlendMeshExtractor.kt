package com.soma3d.app

/**
 * Converts already-decoded Blender mesh-related records into a stable Soma mesh representation.
 *
 * This module intentionally does not guess pointer targets. The caller supplies records that
 * have been linked to the corresponding Mesh/MVert/MLoop/MPoly datablocks by a future
 * version-aware linker. All untrusted counts and indices are checked before constructing output.
 */
object BlendMeshExtractor {
    private const val MAX_VERTICES_PER_MESH = 2_000_000
    private const val MAX_POLYGONS_PER_MESH = 2_000_000
    private const val MAX_CORNERS_PER_MESH = 8_000_000

    data class MeshRecords(
        val mesh: BlendStructDecoder.DecodedRecord,
        val vertices: List<BlendStructDecoder.DecodedRecord>,
        val loops: List<BlendStructDecoder.DecodedRecord>,
        val polygons: List<BlendStructDecoder.DecodedRecord>
    )

    fun extract(inputs: List<MeshRecords>): SomaMeshImportResult {
        val meshes = ArrayList<SomaMesh>()
        val globalWarnings = ArrayList<String>()
        var skipped = 0

        for ((meshIndex, input) in inputs.withIndex()) {
            val name = readString(input.mesh.fields["id"]) 
                ?: readString(input.mesh.fields["name"])
                ?: "Mesh ${meshIndex + 1}"
            val warnings = ArrayList<String>()
            val vertices = decodeVertices(input.vertices)
            if (vertices == null) {
                warnings.add("Vertex records did not contain supported finite co[3] values.")
                globalWarnings.add("$name: ${warnings.last()}")
                skipped++
                continue
            }
            if (vertices.size > MAX_VERTICES_PER_MESH) {
                globalWarnings.add("$name: vertex count exceeds the safe limit.")
                skipped++
                continue
            }
            if (input.polygons.size > MAX_POLYGONS_PER_MESH ||
                input.loops.size > MAX_CORNERS_PER_MESH) {
                globalWarnings.add("$name: topology exceeds safe limits.")
                skipped++
                continue
            }

            val decodedPolygons = ArrayList<SomaPolygon>(input.polygons.size)
            var topologyValid = true
            for (polygonRecord in input.polygons) {
                val start = readInt(polygonRecord.fields["loopstart"])
                val count = readInt(polygonRecord.fields["totloop"])
                    ?: readInt(polygonRecord.fields["loopCount"])
                if (start == null || count == null || start < 0 || count < 0 ||
                    start.toLong() + count.toLong() > input.loops.size.toLong()) {
                    topologyValid = false
                    break
                }
                val indices = ArrayList<Int>(count)
                for (loopIndex in start until start + count) {
                    val vertexIndex = readInt(input.loops[loopIndex].fields["v"])
                        ?: readInt(input.loops[loopIndex].fields["vertexIndex"])
                    if (vertexIndex == null || vertexIndex !in vertices.indices) {
                        topologyValid = false
                        break
                    }
                    indices.add(vertexIndex)
                }
                if (!topologyValid) break
                if (indices.size >= 3) decodedPolygons.add(SomaPolygon(indices))
                else warnings.add("A polygon with fewer than three corners was omitted.")
            }

            if (!topologyValid) {
                globalWarnings.add("$name: polygon loop range or vertex index is invalid; mesh skipped.")
                skipped++
                continue
            }
            if (decodedPolygons.size != input.polygons.size) {
                warnings.add("One or more degenerate polygons were omitted.")
            }
            meshes.add(SomaMesh(name, vertices, decodedPolygons, warnings))
            globalWarnings.addAll(warnings.map { "$name: $it" })
        }

        val message = "Normalized ${meshes.size} mesh(es); skipped $skipped. " +
            if (globalWarnings.isEmpty()) "No extraction warnings." else "${globalWarnings.size} warning(s)."
        return SomaMeshImportResult(meshes, globalWarnings, skipped, message)
    }

    private fun decodeVertices(records: List<BlendStructDecoder.DecodedRecord>): List<SomaVector3>? {
        if (records.size > MAX_VERTICES_PER_MESH) return null
        val result = ArrayList<SomaVector3>(records.size)
        for (record in records) {
            val coordinates = record.fields["co"] as? List<*> ?: return null
            if (coordinates.size < 3) return null
            val x = readDouble(coordinates[0]) ?: return null
            val y = readDouble(coordinates[1]) ?: return null
            val z = readDouble(coordinates[2]) ?: return null
            if (!x.isFinite() || !y.isFinite() || !z.isFinite()) return null
            result.add(SomaVector3(x, y, z))
        }
        return result
    }

    private fun readString(value: Any?): String? = (value as? String)?.takeIf { it.isNotBlank() }

    private fun readInt(value: Any?): Int? {
        val number = when (value) {
            is Byte -> value.toLong()
            is Short -> value.toLong()
            is Int -> value.toLong()
            is Long -> value
            is Double -> if (value.isFinite() && value % 1.0 == 0.0) value.toLong() else return null
            else -> return null
        }
        return number.takeIf { it in 0L..Int.MAX_VALUE.toLong() }?.toInt()
    }

    private fun readDouble(value: Any?): Double? = when (value) {
        is Number -> value.toDouble()
        else -> null
    }
}
