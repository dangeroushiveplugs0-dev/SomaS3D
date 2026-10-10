package com.soma3d.app

/**
 * Resolves Blender Object.data references to normalized mesh datablocks by file address,
 * then creates per-object transformed mesh instances. Never links by display name.
 */
object BlendObjectMeshLinker {
    data class Result(
        val meshes: List<SomaMesh>,
        val warnings: List<String>,
        val linkedObjectCount: Int,
        val skippedObjectCount: Int,
        val usedMeshFallback: Boolean
    )

    fun link(
        objects: List<BlendStructDecoder.DecodedRecord>,
        normalized: SomaMeshImportResult
    ): Result {
        val warnings = ArrayList<String>()
        val addresses = normalized.sourceAddresses
        if (addresses.size != normalized.meshes.size) {
            warnings.add("Mesh source-address list does not align with normalized meshes; object linking is disabled.")
            return Result(normalized.meshes, warnings, 0, objects.size, true)
        }

        val byAddress = addresses.indices
            .filter { addresses[it] != null && addresses[it]!! > 0L }
            .groupBy { addresses[it]!! }
        val sceneMeshes = ArrayList<SomaMesh>()
        var skipped = 0

        for ((index, objectRecord) in objects.withIndex()) {
            val objectLabel = (objectRecord.fields["name"] as? String)
                ?.takeIf { it.isNotBlank() } ?: "Object ${index + 1}"
            val dataAddress = readAddress(objectRecord.fields["data"])
            if (dataAddress == null || dataAddress <= 0L) {
                warnings.add("$objectLabel: mesh data reference is missing or invalid; object skipped.")
                skipped++
                continue
            }

            val matchingIndices = byAddress[dataAddress].orEmpty()
            if (matchingIndices.size != 1) {
                val reason = if (matchingIndices.isEmpty()) "does not resolve to an imported mesh"
                    else "matches multiple normalized meshes ambiguously"
                warnings.add("$objectLabel: data address $dataAddress $reason; object skipped.")
                skipped++
                continue
            }

            val sourceMesh = normalized.meshes[matchingIndices.single()]
            val instance = sourceMesh.copy(name = objectLabel)
            val transformed = SomaObjectTransform.apply(instance, objectRecord.fields["obmat"])
            if (!transformed.applied || transformed.mesh == null) {
                warnings.add("$objectLabel: transform was not applied (${transformed.message}); object skipped.")
                skipped++
                continue
            }
            sceneMeshes.add(transformed.mesh)
        }

        if (sceneMeshes.isEmpty()) {
            if (objects.isNotEmpty()) {
                warnings.add("No object instances could be linked; showing normalized meshes in mesh-local coordinates as a fallback.")
            } else {
                warnings.add("No Object datablocks were decoded; showing normalized meshes in mesh-local coordinates.")
            }
            return Result(normalized.meshes, warnings, 0, skipped, true)
        }

        if (skipped > 0) warnings.add("$skipped object instance(s) were not linked.")
        return Result(sceneMeshes, warnings, sceneMeshes.size, skipped, false)
    }

    private fun readAddress(value: Any?): Long? = when (value) {
        is Long -> value
        is Int -> value.toLong()
        is Short -> value.toLong()
        else -> null
    }
}
