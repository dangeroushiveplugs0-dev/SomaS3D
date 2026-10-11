package com.soma3d.app

import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Imports the explicitly recognized Blender 4.0-style CustomData mesh representation:
 * vertex-domain Float3 "position", corner-domain Int32 ".corner_vert", and Mesh face offsets.
 * Unknown/missing layers are skipped with warnings; no enum IDs or payload layouts are guessed.
 */
object BlendCustomDataMeshLinker {
    private const val MAX_VERTICES = 2_000_000
    private const val MAX_CORNERS = 8_000_000
    private const val MAX_FACES = 2_000_000
    private const val MAX_LAYERS = 256
    private const val MAX_RAW_BYTES = 64 * 1024 * 1024

    data class Result(
        val meshes: List<SomaMesh>,
        val sourceAddresses: List<Long?>,
        val warnings: List<String>,
        val skippedMeshCount: Int,
        val message: String
    )

    fun link(
        meshRecords: List<BlendStructDecoder.DecodedRecord>,
        decoder: BlendDatablockDecoder,
        littleEndian: Boolean,
        blenderVersion: String?
    ): Result {
        val meshes = ArrayList<SomaMesh>()
        val addresses = ArrayList<Long?>()
        val warnings = ArrayList<String>()
        var skipped = 0

        for ((index, mesh) in meshRecords.withIndex()) {
            val vertexData = mesh.fields["vdata"] as? Map<*, *>
            val cornerData = mesh.fields["ldata"] as? Map<*, *>
            // Avoid counting legacy-only meshes as failed attempts.
            if (vertexData == null && cornerData == null) continue

            val name = meshName(mesh, index)
            val vertexCount = readCount(mesh.fields["totvert"])
            val cornerCount = readCount(mesh.fields["totloop"])
            val faceCount = readCount(mesh.fields["faces_num"]) ?: readCount(mesh.fields["totpoly"])
            if (vertexCount == null || cornerCount == null || faceCount == null ||
                vertexCount > MAX_VERTICES || cornerCount > MAX_CORNERS || faceCount > MAX_FACES) {
                warnings.add("$name: CustomData mesh counts are missing, invalid, or above safe limits.")
                skipped++
                continue
            }

            val positionLayer = findLayer(vertexData, decoder, setOf(48), setOf("position", ".position"))
            val cornerVertexLayer = findLayer(cornerData, decoder, setOf(11), setOf(".corner_vert", "corner_vert"))
            if (positionLayer == null || cornerVertexLayer == null) {
                warnings.add("$name: required position(Float3) or .corner_vert(Int32) layer was not found.")
                skipped++
                continue
            }

            val positionsPointer = readPointer(positionLayer.fields["data"])
            val cornerPointer = readPointer(cornerVertexLayer.fields["data"])
            val positionsBytes = positionsPointer?.let {
                decoder.readBytesAtAddress(it, checkedByteCount(vertexCount, 12) ?: return@let null, MAX_RAW_BYTES)
            }?.bytes
            val cornerBytes = cornerPointer?.let {
                decoder.readBytesAtAddress(it, checkedByteCount(cornerCount, 4) ?: return@let null, MAX_RAW_BYTES)
            }?.bytes
            val vertices = positionsBytes?.let { decodeFloat3Payload(it, vertexCount, littleEndian) }
            val cornerVertices = cornerBytes?.let { decodeInt32Payload(it, cornerCount, littleEndian) }
            if (vertices == null || cornerVertices == null) {
                warnings.add("$name: position or corner-vertex payload could not be read and validated.")
                skipped++
                continue
            }

            val offsetsPointer = readPointer(mesh.fields["face_offset_indices"])
            val offsetCount = try { Math.addExact(faceCount, 1) } catch (_: ArithmeticException) { null }
            val offsetBytes = if (offsetsPointer != null && offsetCount != null) {
                decoder.readBytesAtAddress(
                    offsetsPointer, checkedByteCount(offsetCount, 4) ?: -1, MAX_RAW_BYTES
                ).bytes
            } else null
            val offsets = if (offsetBytes != null && offsetCount != null) {
                decodeInt32Payload(offsetBytes, offsetCount, littleEndian)
            } else null
            val polygons = offsets?.let { buildPolygons(it, cornerVertices, vertices.size) }
            if (polygons == null) {
                warnings.add("$name: face offsets are missing, malformed, non-monotonic, or reference invalid vertices.")
                skipped++
                continue
            }

            meshes.add(SomaMesh(name, vertices, polygons))
            addresses.add(mesh.fileAddress)
        }

        val message = "Imported ${meshes.size} CustomData mesh(es); skipped $skipped."
        return Result(meshes, addresses, warnings, skipped, message)
    }

    private fun findLayer(
        customData: Map<*, *>?,
        decoder: BlendDatablockDecoder,
        typeIds: Set<Int>,
        names: Set<String>
    ): BlendStructDecoder.DecodedRecord? {
        if (customData == null) return null
        val count = readCount(customData["totlayer"]) ?: return null
        if (count !in 1..MAX_LAYERS) return null
        val pointer = readPointer(customData["layers"]) ?: return null
        if (pointer <= 0L) return null
        val decoded = decoder.decodeRecordsAtAddress(pointer, "CustomDataLayer", count)
        if (!decoded.success || decoded.records.size != count) return null
        return decoded.records.firstOrNull { layer ->
            val type = readCount(layer.fields["type"])
            val name = (layer.fields["name"] as? String)?.trim()
            type != null && type in typeIds && name != null && name in names &&
                (readPointer(layer.fields["data"]) ?: 0L) > 0L
        }
    }

    internal fun decodeFloat3Payload(bytes: ByteArray, count: Int, littleEndian: Boolean): List<SomaVector3>? {
        if (count < 0 || checkedByteCount(count, 12) != bytes.size) return null
        val buffer = ByteBuffer.wrap(bytes).order(if (littleEndian) ByteOrder.LITTLE_ENDIAN else ByteOrder.BIG_ENDIAN)
        val vertices = ArrayList<SomaVector3>(count)
        repeat(count) {
            val x = buffer.float.toDouble()
            val y = buffer.float.toDouble()
            val z = buffer.float.toDouble()
            if (!x.isFinite() || !y.isFinite() || !z.isFinite()) return null
            vertices.add(SomaVector3(x, y, z))
        }
        return vertices
    }

    internal fun decodeInt32Payload(bytes: ByteArray, count: Int, littleEndian: Boolean): List<Int>? {
        if (count < 0 || checkedByteCount(count, 4) != bytes.size) return null
        val buffer = ByteBuffer.wrap(bytes).order(if (littleEndian) ByteOrder.LITTLE_ENDIAN else ByteOrder.BIG_ENDIAN)
        return List(count) { buffer.int }
    }

    internal fun buildPolygons(
        offsets: List<Int>,
        cornerVertices: List<Int>,
        vertexCount: Int
    ): List<SomaPolygon>? {
        if (offsets.isEmpty() || offsets.first() != 0 || offsets.last() != cornerVertices.size) return null
        if (offsets.any { it < 0 || it > cornerVertices.size }) return null
        if (offsets.zipWithNext().any { (a, b) -> b < a }) return null
        if (cornerVertices.any { it < 0 || it >= vertexCount }) return null
        val polygons = ArrayList<SomaPolygon>(offsets.size - 1)
        for (face in 0 until offsets.size - 1) {
            val start = offsets[face]
            val end = offsets[face + 1]
            if (end - start >= 3) polygons.add(SomaPolygon(cornerVertices.subList(start, end).toList()))
        }
        return polygons
    }

    private fun meshName(mesh: BlendStructDecoder.DecodedRecord, index: Int): String {
        (mesh.fields["name"] as? String)?.takeIf { it.isNotBlank() }?.let { return it }
        val id = mesh.fields["id"] as? Map<*, *>
        (id?.get("name") as? String)?.trim()?.takeIf { it.isNotBlank() }?.let { return it }
        return "Mesh ${index + 1}"
    }

    private fun checkedByteCount(count: Int, stride: Int): Int? {
        if (count < 0) return null
        return try {
            Math.multiplyExact(count, stride).takeIf { it <= MAX_RAW_BYTES }
        } catch (_: ArithmeticException) { null }
    }

    private fun readCount(value: Any?): Int? {
        val number = when (value) {
            is Int -> value.toLong()
            is Long -> value
            is Short -> value.toLong()
            else -> return null
        }
        return number.takeIf { it in 0L..Int.MAX_VALUE.toLong() }?.toInt()
    }

    private fun readPointer(value: Any?): Long? = when (value) {
        is Int -> value.toLong() and 0xffffffffL
        is Long -> value
        else -> null
    }
}
