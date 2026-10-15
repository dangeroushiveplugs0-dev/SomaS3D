package com.soma3d.app

/** Normalized armature metadata. Coordinates are preserved in Blender armature-local space. */
data class SomaBone(
    val name: String,
    val parentName: String?,
    val head: SomaVector3?,
    val tail: SomaVector3?,
    val roll: Double?
)

data class SomaRig(
    val objectName: String,
    val bones: List<SomaBone>,
    val warnings: List<String> = emptyList()
)

data class BlendRigImportResult(
    val rigs: List<SomaRig>,
    val warnings: List<String>,
    val skippedArmatureCount: Int,
    val message: String
)

/**
 * Reads the Bone hierarchy from armature datablocks without evaluating pose drivers,
 * constraints, actions, or embedded scripts. The importer is deliberately metadata-first.
 */
object BlendRigImporter {
    private const val MAX_BONES_PER_RIG = 100_000

    fun readRigs(
        objects: List<BlendStructDecoder.DecodedRecord>,
        decoder: BlendDatablockDecoder
    ): BlendRigImportResult {
        val rigs = ArrayList<SomaRig>()
        val warnings = ArrayList<String>()
        val seenArmatures = HashSet<Long>()
        var skipped = 0

        for ((index, objectRecord) in objects.withIndex()) {
            val dataAddress = pointer(objectRecord.fields["data"]) ?: continue
            if (dataAddress <= 0L || !seenArmatures.add(dataAddress)) continue
            val armature = decoder.decodeRecordsAtAddress(dataAddress, "bArmature", 1)
                .records.firstOrNull() ?: continue
            val objectName = (objectRecord.fields["name"] as? String)?.trim()
                ?.takeIf { it.isNotBlank() } ?: "Armature ${index + 1}"
            val firstBone = pointer(armature.fields["bone"])
            if (firstBone == null || firstBone <= 0L) {
                warnings.add("$objectName: armature has no supported Bone list.")
                skipped++
                continue
            }

            val records = ArrayList<BlendStructDecoder.DecodedRecord>()
            val visited = HashSet<Long>()
            var address: Long? = firstBone
            while (address != null && address > 0L && records.size < MAX_BONES_PER_RIG) {
                if (!visited.add(address)) {
                    warnings.add("$objectName: Bone linked list contains a cycle; traversal stopped.")
                    break
                }
                val bone = decoder.decodeRecordsAtAddress(address, "Bone", 1).records.firstOrNull()
                if (bone == null) {
                    warnings.add("$objectName: a Bone record could not be decoded; traversal stopped.")
                    break
                }
                records.add(bone)
                address = pointer(bone.fields["next"])
            }
            if (address != null && address > 0L && records.size >= MAX_BONES_PER_RIG) {
                warnings.add("$objectName: bone count exceeded the safety limit.")
            }
            if (records.isEmpty()) {
                skipped++
                continue
            }

            val namesByAddress = records.mapIndexedNotNull { boneIndex, record ->
                record.fileAddress?.let { it to boneName(record, boneIndex) }
            }.toMap()
            val bones = records.mapIndexed { boneIndex, record ->
                val parentAddress = pointer(record.fields["parent"])
                SomaBone(
                    name = boneName(record, boneIndex),
                    parentName = parentAddress?.let { namesByAddress[it] },
                    head = vector3(record.fields["head"]),
                    tail = vector3(record.fields["tail"]),
                    roll = number(record.fields["roll"])
                )
            }
            rigs.add(SomaRig(objectName, bones))
        }

        return BlendRigImportResult(
            rigs, warnings, skipped,
            "Imported ${rigs.size} armature(s) with ${rigs.sumOf { it.bones.size }} bone(s); skipped $skipped."
        )
    }

    private fun boneName(record: BlendStructDecoder.DecodedRecord, index: Int): String =
        (record.fields["name"] as? String)?.trim()?.trim('\u0000')
            ?.takeIf { it.isNotBlank() } ?: "Bone ${index + 1}"

    private fun pointer(value: Any?): Long? = when (value) {
        is Long -> value
        is Int -> value.toLong() and 0xffffffffL
        else -> null
    }

    private fun number(value: Any?): Double? = (value as? Number)?.toDouble()?.takeIf { it.isFinite() }

    private fun vector3(value: Any?): SomaVector3? {
        val parts = value as? List<*> ?: return null
        if (parts.size < 3) return null
        val x = number(parts[0]) ?: return null
        val y = number(parts[1]) ?: return null
        val z = number(parts[2]) ?: return null
        return SomaVector3(x, y, z)
    }
}
