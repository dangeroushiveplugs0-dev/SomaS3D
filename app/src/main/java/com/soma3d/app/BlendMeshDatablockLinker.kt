package com.soma3d.app

/**
 * Resolves direct, legacy-style Mesh -> MVert/MLoop/MPoly arrays without executing Blender code.
 * Newer CustomData-based layouts may require a different recognizer; failures are reported rather
 * than guessed.
 */
object BlendMeshDatablockLinker {
    data class LinkResult(
        val inputs: List<BlendMeshExtractor.MeshRecords>,
        val warnings: List<String>,
        val skippedMeshCount: Int
    )

    private const val MAX_VERTICES = 2_000_000
    private const val MAX_LOOPS = 8_000_000
    private const val MAX_POLYGONS = 2_000_000

    fun link(
        meshRecords: List<BlendStructDecoder.DecodedRecord>,
        decoder: BlendDatablockDecoder
    ): LinkResult {
        val inputs = ArrayList<BlendMeshExtractor.MeshRecords>()
        val warnings = ArrayList<String>()
        var skipped = 0

        for ((index, mesh) in meshRecords.withIndex()) {
            val name = (mesh.fields["name"] as? String)?.takeIf { it.isNotBlank() }
                ?: "Mesh ${index + 1}"
            val vertexCount = readCount(mesh.fields["totvert"])
            val loopCount = readCount(mesh.fields["totloop"])
            val polygonCount = readCount(mesh.fields["totpoly"])
            if (vertexCount == null || loopCount == null || polygonCount == null ||
                vertexCount > MAX_VERTICES || loopCount > MAX_LOOPS || polygonCount > MAX_POLYGONS) {
                warnings.add("$name: required counts are missing, invalid, or above safe limits.")
                skipped++
                continue
            }

            val vertices = resolveArray(mesh.fields["mvert"], vertexCount, "MVert", decoder)
            if (vertices == null) {
                warnings.add("$name: vertex array could not be resolved using the supported direct-pointer layout.")
                skipped++
                continue
            }
            val loops = if (loopCount == 0) emptyList()
                else resolveArray(mesh.fields["mloop"], loopCount, "MLoop", decoder)
            if (loops == null) {
                warnings.add("$name: loop array could not be resolved; mesh skipped.")
                skipped++
                continue
            }
            val polygons = if (polygonCount == 0) emptyList()
                else resolveArray(mesh.fields["mpoly"], polygonCount, "MPoly", decoder)
            if (polygons == null) {
                warnings.add("$name: polygon array could not be resolved; mesh skipped.")
                skipped++
                continue
            }
            inputs.add(BlendMeshExtractor.MeshRecords(mesh, vertices, loops, polygons))
        }
        return LinkResult(inputs, warnings, skipped)
    }

    private fun resolveArray(
        pointerValue: Any?,
        count: Int,
        expectedType: String,
        decoder: BlendDatablockDecoder
    ): List<BlendStructDecoder.DecodedRecord>? {
        if (count == 0) return emptyList()
        val pointer = when (pointerValue) {
            is Long -> pointerValue
            is Int -> pointerValue.toLong()
            else -> return null
        }
        if (pointer <= 0L) return null
        val resolved = decoder.decodeRecordsAtAddress(pointer, expectedType, count)
        return resolved.records.takeIf { resolved.success && it.size == count }
    }

    private fun readCount(value: Any?): Int? {
        val longValue = when (value) {
            is Int -> value.toLong()
            is Long -> value
            is Short -> value.toLong()
            else -> return null
        }
        return longValue.takeIf { it in 0L..Int.MAX_VALUE.toLong() }?.toInt()
    }
}
