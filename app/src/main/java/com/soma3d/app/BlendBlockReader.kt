package com.soma3d.app

import java.io.EOFException
import java.io.InputStream

/**
 * Sequential, bounded reader for Blender's outer block stream.
 * It never treats file-stored pointer values as process addresses.
 * DNA1 payloads are retained only within a conservative size limit.
 */
object BlendBlockReader {
    private const val HEADER_BYTES = 12
    private const val MAX_BLOCK_BYTES = 256L * 1024L * 1024L
    private const val MAX_DNA_BYTES = 64 * 1024 * 1024
    private const val MAX_BLOCKS = 2_000_000
    private const val MAX_SCHEMA_ITEMS = 1_000_000

    data class BlockSummary(
        val code: String,
        val payloadBytes: Long,
        val oldAddress: Long,
        val sdnaIndex: Long,
        val count: Long,
        /** Absolute byte offset of this block's payload in the source file, when known. */
        val payloadOffset: Long = -1L
    )

    data class SchemaField(val typeName: String, val fieldName: String)

    data class SchemaStruct(
        val typeName: String,
        val fields: List<SchemaField>
    )

    data class Schema(
        val names: List<String>,
        val types: List<String>,
        val typeLengths: List<Int>,
        val structs: List<SchemaStruct>
    )

    data class Result(
        val validHeader: Boolean,
        val version: String?,
        val pointerBits: Int?,
        val littleEndian: Boolean?,
        val blocks: List<BlockSummary>,
        val schema: Schema?,
        val endedCleanly: Boolean,
        val message: String
    )

    fun read(input: InputStream): Result {
        val header = readExact(input, HEADER_BYTES)
            ?: return failure("File is too short for a Blender header.")
        if (String(header, 0, 7, Charsets.US_ASCII) != "BLENDER") {
            return failure("Invalid Blender signature.")
        }

        val pointerBits = when (header[7].toInt().toChar()) {
            '-' -> 32
            '_' -> 64
            else -> null
        } ?: return failure("Unknown Blender pointer-size marker.")
        val littleEndian = when (header[8].toInt().toChar()) {
            'v' -> true
            'V' -> false
            else -> null
        } ?: return failure("Unknown Blender byte-order marker.")
        val version = String(header, 9, 3, Charsets.US_ASCII)
        if (!version.all { it in '0'..'9' }) return failure("Invalid Blender version field.")

        val blocks = ArrayList<BlockSummary>()
        var schema: Schema? = null
        var endedCleanly = false
        var streamOffset = HEADER_BYTES.toLong()

        try {
            while (blocks.size < MAX_BLOCKS) {
                val blockHeader = readExact(input, 4 + 4 + pointerBits / 8 + 4 + 4)
                    ?: return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                        "Unexpected end of file before ENDB block.")
                streamOffset = Math.addExact(streamOffset, blockHeader.size.toLong())
                val payloadOffset = streamOffset
                val code = String(blockHeader, 0, 4, Charsets.US_ASCII)
                var offset = 4
                val length = unsignedInt(blockHeader, offset, littleEndian); offset += 4
                val oldAddress = if (pointerBits == 32) {
                    unsignedInt(blockHeader, offset, littleEndian).also { offset += 4 }
                } else {
                    longPointer(blockHeader, offset, littleEndian).also { offset += 8 }
                }
                val sdnaIndex = unsignedInt(blockHeader, offset, littleEndian); offset += 4
                val count = unsignedInt(blockHeader, offset, littleEndian)

                if (length > MAX_BLOCK_BYTES) {
                    return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                        "Block " + code + " exceeds the safe 256 MiB per-block limit.")
                }

                if (code == "DNA1") {
                    if (length > MAX_DNA_BYTES) {
                        return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                            "DNA1 schema exceeds the safe 64 MiB limit.")
                    }
                    val payload = readExact(input, length.toInt())
                        ?: return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                            "DNA1 payload is truncated.")
                    schema = parseSchema(payload, littleEndian)
                    if (schema == null) {
                        return Result(true, version, pointerBits, littleEndian, blocks, null, false,
                            "DNA1 block was found, but its SDNA schema is malformed or unsupported.")
                    }
                } else {
                    if (!skipExact(input, length)) {
                        return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                            "Block " + code + " payload is truncated.")
                    }
                }
                streamOffset = Math.addExact(streamOffset, length)

                blocks.add(BlockSummary(code, length, oldAddress, sdnaIndex, count, payloadOffset))
                if (code == "ENDB") {
                    endedCleanly = true
                    break
                }
            }
        } catch (_: Exception) {
            return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                "Malformed or truncated Blender block stream.")
        }

        if (blocks.size >= MAX_BLOCKS && !endedCleanly) {
            return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                "Block count exceeded the safety limit.")
        }
        if (schema == null) {
            return Result(true, version, pointerBits, littleEndian, blocks, null, endedCleanly,
                "Block stream scanned, but no usable DNA1 schema was found.")
        }
        if (!endedCleanly) {
            return Result(true, version, pointerBits, littleEndian, blocks, schema, false,
                "Schema found, but the file did not end with an ENDB block.")
        }
        return Result(true, version, pointerBits, littleEndian, blocks, schema, true,
            "Scanned " + blocks.size + " blocks; SDNA contains " + schema.types.size +
                " types and " + schema.structs.size + " structures. Object and mesh extraction is not implemented yet.")
    }

    private fun parseSchema(bytes: ByteArray, littleEndian: Boolean): Schema? {
        val reader = SchemaReader(bytes, littleEndian)
        if (reader.readTag() != "SDNA" || reader.readTag() != "NAME") return null
        val nameCount = reader.readCount() ?: return null
        val names = reader.readCStringTable(nameCount) ?: return null
        reader.align4()
        if (reader.readTag() != "TYPE") return null
        val typeCount = reader.readCount() ?: return null
        val types = reader.readCStringTable(typeCount) ?: return null
        reader.align4()
        if (reader.readTag() != "TLEN") return null
        val lengths = ArrayList<Int>(typeCount)
        repeat(typeCount) {
            val len = reader.readU16() ?: return null
            lengths.add(len)
        }
        reader.align4()
        if (reader.readTag() != "STRC") return null
        val structCount = reader.readCount() ?: return null
        val structs = ArrayList<SchemaStruct>(structCount)
        repeat(structCount) {
            val typeIndex = reader.readU16() ?: return null
            val fieldCount = reader.readU16() ?: return null
            if (typeIndex !in types.indices || fieldCount > MAX_SCHEMA_ITEMS) return null
            val fields = ArrayList<SchemaField>(fieldCount)
            repeat(fieldCount) {
                val fieldTypeIndex = reader.readU16() ?: return null
                val fieldNameIndex = reader.readU16() ?: return null
                if (fieldTypeIndex !in types.indices || fieldNameIndex !in names.indices) return null
                fields.add(SchemaField(types[fieldTypeIndex], names[fieldNameIndex]))
            }
            structs.add(SchemaStruct(types[typeIndex], fields))
        }
        return Schema(names, types, lengths, structs)
    }

    private class SchemaReader(private val bytes: ByteArray, private val littleEndian: Boolean) {
        private var position = 0

        fun readTag(): String? {
            if (position + 4 > bytes.size) return null
            val value = String(bytes, position, 4, Charsets.US_ASCII)
            position += 4
            return value
        }

        fun readCount(): Int? {
            val value = readU32() ?: return null
            if (value < 0 || value > MAX_SCHEMA_ITEMS) return null
            return value
        }

        fun readCStringTable(count: Int): List<String>? {
            val result = ArrayList<String>(count)
            repeat(count) {
                val start = position
                while (position < bytes.size && bytes[position].toInt() != 0) position++
                if (position >= bytes.size) return null
                result.add(String(bytes, start, position - start, Charsets.UTF_8))
                position++
            }
            return result
        }

        fun readU16(): Int? {
            if (position + 2 > bytes.size) return null
            val a = bytes[position++].toInt() and 0xff
            val b = bytes[position++].toInt() and 0xff
            return if (littleEndian) a or (b shl 8) else (a shl 8) or b
        }

        private fun readU32(): Int? {
            if (position + 4 > bytes.size) return null
            val a = bytes[position++].toLong() and 0xff
            val b = bytes[position++].toLong() and 0xff
            val c = bytes[position++].toLong() and 0xff
            val d = bytes[position++].toLong() and 0xff
            val value = if (littleEndian) a or (b shl 8) or (c shl 16) or (d shl 24)
                else (a shl 24) or (b shl 16) or (c shl 8) or d
            if (value > Int.MAX_VALUE) return null
            return value.toInt()
        }

        fun align4() {
            position = (position + 3) and -4
        }
    }

    private fun readExact(input: InputStream, size: Int): ByteArray? {
        val result = ByteArray(size)
        var offset = 0
        while (offset < size) {
            val read = input.read(result, offset, size - offset)
            if (read < 0) return null
            if (read == 0) continue
            offset += read
        }
        return result
    }

    private fun skipExact(input: InputStream, length: Long): Boolean {
        var remaining = length
        val buffer = ByteArray(8192)
        while (remaining > 0) {
            val read = input.read(buffer, 0, minOf(buffer.size.toLong(), remaining).toInt())
            if (read < 0) return false
            if (read == 0) continue
            remaining -= read
        }
        return true
    }

    private fun unsignedInt(bytes: ByteArray, offset: Int, littleEndian: Boolean): Long {
        var result = 0L
        if (littleEndian) {
            for (i in 3 downTo 0) result = (result shl 8) or (bytes[offset + i].toLong() and 0xff)
        } else {
            for (i in 0..3) result = (result shl 8) or (bytes[offset + i].toLong() and 0xff)
        }
        return result
    }

    private fun longPointer(bytes: ByteArray, offset: Int, littleEndian: Boolean): Long {
        var result = 0L
        if (littleEndian) {
            for (i in 7 downTo 0) result = (result shl 8) or (bytes[offset + i].toLong() and 0xff)
        } else {
            for (i in 0..7) result = (result shl 8) or (bytes[offset + i].toLong() and 0xff)
        }
        return result
    }

    private fun failure(message: String) =
        Result(false, null, null, null, emptyList(), null, false, message)
}
