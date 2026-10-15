package com.soma3d.app

/**
 * Bounded importer for Blender Key/KeyBlock shape-key coordinates.
 *
 * This preserves shape-key names, values, relative-key links, and coordinate sets.
 * It never evaluates drivers or executes embedded Blender scripts. Unsupported key
 * layouts are reported per mesh rather than aborting the whole .blend import.
 */
data class SomaShapeKey(
    val name: String,
    val value: Double,
    val relativeKeyName: String?,
    val coordinates: List<SomaVector3>
)

data class SomaShapeKeySet(
    val meshAddress: Long,
    val meshName: String,
    val keys: List<SomaShapeKey>,
    val warnings: List<String> = emptyList()
)

data class BlendShapeKeyImportResult(
    val sets: List<SomaShapeKeySet>,
    val warnings: List<String>,
    val skippedMeshCount: Int,
    val message: String
)

object BlendShapeKeyImporter {
    private const val MAX_KEYS_PER_MESH = 256
    private const val MAX_VERTICES_PER_KEY = 2_000_000
    private const val MAX_BYTES_PER_KEY = 24 * 1024 * 1024

    fun readShapeKeys(
        meshRecords: List<BlendStructDecoder.DecodedRecord>,
        decoder: BlendDatablockDecoder,
        littleEndian: Boolean
    ): BlendShapeKeyImportResult {
        val sets = ArrayList<SomaShapeKeySet>()
        val warnings = ArrayList<String>()
        var skipped = 0

        for ((meshIndex, mesh) in meshRecords.withIndex()) {
            val meshAddress = mesh.fileAddress
            val keyAddress = pointer(mesh.fields["key"])
            if (keyAddress == null || keyAddress <= 0L) continue
            val meshName = readName(mesh.fields["name"]) ?: "Mesh ${meshIndex + 1}"
            val keyRecord = decoder.decodeRecordsAtAddress(keyAddress, "Key", 1)
                .records.firstOrNull()
            if (keyRecord == null) {
                warnings.add("$meshName: shape-key datablock could not be decoded.")
                skipped++
                continue
            }

            val firstBlock = pointer(keyRecord.fields["block"])
            if (firstBlock == null || firstBlock <= 0L) {
                warnings.add("$meshName: shape-key datablock has no supported KeyBlock list.")
                skipped++
                continue
            }

            val keys = ArrayList<SomaShapeKey>()
            val localWarnings = ArrayList<String>()
            val visited = HashSet<Long>()
            var address: Long? = firstBlock
            var baseVertexCount: Int? = null

            while (address != null && address > 0L && keys.size < MAX_KEYS_PER_MESH) {
                if (!visited.add(address)) {
                    localWarnings.add("KeyBlock linked list contains a cycle; traversal stopped.")
                    break
                }
                val record = decoder.decodeRecordsAtAddress(address, "KeyBlock", 1)
                    .records.firstOrNull()
                if (record == null) {
                    localWarnings.add("A KeyBlock record could not be decoded; traversal stopped.")
                    break
                }
                val name = readName(record.fields["name"]) ?: "Shape Key ${keys.size + 1}"
                val count = integer(record.fields["totelem"])
                val dataAddress = pointer(record.fields["data"])
                if (count == null || count !in 0..MAX_VERTICES_PER_KEY) {
                    localWarnings.add("$name: invalid or excessive coordinate count.")
                } else if (count > 0 && dataAddress != null && dataAddress > 0L) {
                    val byteCount = try { Math.multiplyExact(count, 12) } catch (_: ArithmeticException) { -1 }
                    if (byteCount <= 0 || byteCount > MAX_BYTES_PER_KEY) {
                        localWarnings.add("$name: shape-key payload exceeds safe limits.")
                    } else {
                        val raw = decoder.readBytesAtAddress(dataAddress, byteCount, MAX_BYTES_PER_KEY).bytes
                        val coordinates = raw?.let { decodeFloat3(it, count, littleEndian) }
                        if (coordinates == null) {
                            localWarnings.add("$name: coordinate payload was unavailable or invalid.")
                        } else if (baseVertexCount != null && coordinates.size != baseVertexCount) {
                            localWarnings.add("$name: vertex count differs from the preceding key; key retained but may not be compatible.")
                            keys.add(SomaShapeKey(name, finiteNumber(record.fields["value"]) ?: 0.0,
                                relativeName(record, keyRecord), coordinates))
                        } else {
                            if (baseVertexCount == null) baseVertexCount = coordinates.size
                            keys.add(SomaShapeKey(name, finiteNumber(record.fields["value"]) ?: 0.0,
                                relativeName(record, keyRecord), coordinates))
                        }
                    }
                } else {
                    localWarnings.add("$name: coordinate pointer or count is missing.")
                }
                address = pointer(record.fields["next"])
            }

            if (address != null && address > 0L && keys.size >= MAX_KEYS_PER_MESH) {
                localWarnings.add("Shape-key count exceeded the safe limit of $MAX_KEYS_PER_MESH.")
            }
            if (keys.isEmpty()) {
                warnings.add("$meshName: no usable shape-key coordinate sets were found.")
                warnings.addAll(localWarnings.map { "$meshName: $it" })
                skipped++
            } else {
                sets.add(SomaShapeKeySet(meshAddress ?: keyAddress, meshName, keys, localWarnings))
                warnings.addAll(localWarnings.map { "$meshName: $it" })
            }
        }
        return BlendShapeKeyImportResult(
            sets, warnings, skipped,
            "Imported shape-key coordinates for ${sets.size} mesh(es); skipped $skipped."
        )
    }

    internal fun decodeFloat3(bytes: ByteArray, count: Int, littleEndian: Boolean): List<SomaVector3>? {
        if (count < 0 || bytes.size != count.toLong() * 12L) return null
        val buffer = java.nio.ByteBuffer.wrap(bytes).order(
            if (littleEndian) java.nio.ByteOrder.LITTLE_ENDIAN else java.nio.ByteOrder.BIG_ENDIAN
        )
        val result = ArrayList<SomaVector3>(count)
        repeat(count) {
            val x = buffer.float.toDouble()
            val y = buffer.float.toDouble()
            val z = buffer.float.toDouble()
            if (!x.isFinite() || !y.isFinite() || !z.isFinite()) return null
            result.add(SomaVector3(x, y, z))
        }
        return result
    }

    private fun relativeName(block: BlendStructDecoder.DecodedRecord, key: BlendStructDecoder.DecodedRecord): String? {
        val relative = integer(block.fields["relative"])
        val names = key.fields["block"] as? List<*>
        // Blender's relative field is commonly an index, not a pointer. Preserve its index label
        // when a reliable name cannot be resolved instead of guessing a relationship.
        return relative?.let { "relative index $it" }
    }

    private fun readName(value: Any?): String? = (value as? String)?.trim()
        ?.trim('\u0000')?.takeIf { it.isNotBlank() }

    private fun integer(value: Any?): Int? = when (value) {
        is Int -> value
        is Short -> value.toInt()
        is Long -> value.takeIf { it in 0L..Int.MAX_VALUE.toLong() }?.toInt()
        else -> null
    }

    private fun pointer(value: Any?): Long? = when (value) {
        is Long -> value
        is Int -> value.toLong() and 0xffffffffL
        else -> null
    }

    private fun finiteNumber(value: Any?): Double? = (value as? Number)?.toDouble()?.takeIf { it.isFinite() }
}
