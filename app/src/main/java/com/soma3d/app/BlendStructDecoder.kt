package com.soma3d.app

import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Conservative SDNA-driven struct layout and record decoder.
 *
 * Layouts are accepted only when computed field offsets fit the SDNA TLEN size exactly.
 * Unsupported declarations and ABI layouts fail closed instead of guessing offsets.
 */
object BlendStructDecoder {
    data class FieldLayout(
        val name: String,
        val typeName: String,
        val offset: Int,
        val byteSize: Int,
        val pointerDepth: Int,
        val arrayDimensions: List<Int>
    )

    data class StructLayout(
        val typeName: String,
        val byteSize: Int,
        val fields: List<FieldLayout>,
        val supported: Boolean,
        val message: String
    )

    data class DecodedRecord(val typeName: String, val fields: Map<String, Any>)

    data class DecodeResult(
        val records: List<DecodedRecord>,
        val success: Boolean,
        val message: String
    )

    private data class Declarator(
        val identifier: String,
        val pointerDepth: Int,
        val dimensions: List<Int>,
        val supported: Boolean
    )

    fun layout(
        schema: BlendBlockReader.Schema,
        structTypeName: String,
        pointerBits: Int
    ): StructLayout {
        if (pointerBits != 32 && pointerBits != 64) {
            return unsupported(structTypeName, "Pointer width must be 32 or 64 bits.")
        }
        val context = LayoutContext(schema, pointerBits / 8)
        return context.structLayout(structTypeName)
    }

    fun decodeRecords(
        schema: BlendBlockReader.Schema,
        sdnaIndex: Long,
        count: Long,
        payload: ByteArray,
        pointerBits: Int,
        littleEndian: Boolean,
        maxRecords: Int = 100_000
    ): DecodeResult {
        if (sdnaIndex < 0L || sdnaIndex >= schema.structs.size.toLong()) {
            return DecodeResult(emptyList(), false, "SDNA structure index is out of range.")
        }
        if (count < 0L || count > maxRecords.toLong()) {
            return DecodeResult(emptyList(), false, "Record count exceeds the safe decode limit.")
        }
        val struct = schema.structs[sdnaIndex.toInt()]
        val layout = layout(schema, struct.typeName, pointerBits)
        if (!layout.supported) return DecodeResult(emptyList(), false, layout.message)
        val requiredBytes = try {
            Math.multiplyExact(count, layout.byteSize.toLong())
        } catch (_: ArithmeticException) {
            return DecodeResult(emptyList(), false, "Record byte count overflowed.")
        }
        if (requiredBytes > payload.size.toLong()) {
            return DecodeResult(emptyList(), false, "Payload is too short for the declared records.")
        }

        val result = ArrayList<DecodedRecord>(count.toInt())
        repeat(count.toInt()) { recordIndex ->
            val recordStart = recordIndex * layout.byteSize
            val fields = LinkedHashMap<String, Any>()
            for (field in layout.fields) {
                val value = decodeField(
                    schema, field, payload, recordStart, pointerBits / 8, littleEndian
                ) ?: continue
                fields[field.name] = value
            }
            result.add(DecodedRecord(layout.typeName, fields))
        }
        return DecodeResult(result, true, "Decoded ${result.size} record(s) as ${layout.typeName}.")
    }

    private class LayoutContext(
        private val schema: BlendBlockReader.Schema,
        private val pointerBytes: Int
    ) {
        private val typeLength = schema.types.indices.associate { schema.types[it] to schema.typeLengths[it] }
        private val structs = schema.structs.associateBy { it.typeName }
        private val cache = HashMap<String, StructLayout>()
        private val visiting = HashSet<String>()

        fun structLayout(typeName: String): StructLayout {
            cache[typeName]?.let { return it }
            val struct = structs[typeName] ?: return unsupported(typeName, "No SDNA structure named $typeName.")
            if (!visiting.add(typeName)) return unsupported(typeName, "Recursive by-value structure layout is unsupported.")
            try {
                var offset = 0
                var structAlignment = 1
                val fields = ArrayList<FieldLayout>(struct.fields.size)
                for (field in struct.fields) {
                    val declarator = parseDeclarator(field.fieldName)
                    if (!declarator.supported) {
                        return unsupported(typeName, "Unsupported field declaration: ${field.fieldName}.")
                    }
                    val baseSize = if (declarator.pointerDepth > 0) pointerBytes
                        else typeLength[field.typeName] ?: return unsupported(typeName, "Unknown SDNA type ${field.typeName}.")
                    if (baseSize <= 0) return unsupported(typeName, "Invalid size for SDNA type ${field.typeName}.")
                    val count = try {
                        declarator.dimensions.fold(1L) { acc, n -> Math.multiplyExact(acc, n.toLong()) }
                    } catch (_: ArithmeticException) {
                        return unsupported(typeName, "Array dimension overflow in ${field.fieldName}.")
                    }
                    val fieldBytesLong = try {
                        Math.multiplyExact(baseSize.toLong(), count)
                    } catch (_: ArithmeticException) {
                        return unsupported(typeName, "Field size overflow in ${field.fieldName}.")
                    }
                    if (fieldBytesLong > Int.MAX_VALUE) {
                        return unsupported(typeName, "Field is too large: ${field.fieldName}.")
                    }
                    val alignment = when {
                        declarator.pointerDepth > 0 -> pointerBytes
                        field.typeName in structs -> {
                            val nested = structLayout(field.typeName)
                            if (!nested.supported) return unsupported(typeName, "Nested field ${field.fieldName}: ${nested.message}")
                            alignmentOf(nested.fields, nested.byteSize, pointerBytes)
                        }
                        else -> primitiveAlignment(baseSize, pointerBytes)
                    }
                    offset = align(offset, alignment)
                    fields.add(FieldLayout(
                        declarator.identifier, field.typeName, offset, fieldBytesLong.toInt(),
                        declarator.pointerDepth, declarator.dimensions
                    ))
                    offset = try {
                        Math.addExact(offset, fieldBytesLong.toInt())
                    } catch (_: ArithmeticException) {
                        return unsupported(typeName, "Struct size overflow.")
                    }
                    structAlignment = maxOf(structAlignment, alignment)
                }
                val computedSize = align(offset, structAlignment)
                val declaredSize = typeLength[typeName] ?: return unsupported(typeName, "No SDNA TLEN for $typeName.")
                if (computedSize != declaredSize) {
                    return unsupported(typeName, "Computed layout is $computedSize bytes but SDNA TLEN declares $declaredSize; refusing guessed offsets.")
                }
                val result = StructLayout(typeName, declaredSize, fields, true, "Layout validated against SDNA TLEN.")
                cache[typeName] = result
                return result
            } finally {
                visiting.remove(typeName)
            }
        }

        private fun alignmentOf(fields: List<FieldLayout>, size: Int, pointerBytes: Int): Int {
            if (fields.isEmpty()) return 1
            return fields.maxOf { field ->
                if (field.pointerDepth > 0) pointerBytes
                else primitiveAlignment(field.byteSize.coerceAtLeast(1), pointerBytes)
            }.coerceAtMost(pointerBytes).coerceAtLeast(1)
        }
    }

    private fun decodeField(
        schema: BlendBlockReader.Schema,
        field: FieldLayout,
        payload: ByteArray,
        recordStart: Int,
        pointerBytes: Int,
        littleEndian: Boolean
    ): Any? {
        val start = recordStart + field.offset
        val end = start.toLong() + field.byteSize.toLong()
        if (start < 0 || end > payload.size.toLong()) return null
        val raw = payload.copyOfRange(start, end.toInt())
        val byteOrder = if (littleEndian) ByteOrder.LITTLE_ENDIAN else ByteOrder.BIG_ENDIAN

        if (field.pointerDepth > 0) {
            if (field.arrayDimensions.isNotEmpty()) {
                val width = pointerBytes
                if (raw.size % width != 0) return null
                return (raw.indices step width).map { offset -> readUnsigned(raw, offset, width, byteOrder) }
            }
            return readUnsigned(raw, 0, pointerBytes, byteOrder)
        }
        if (field.arrayDimensions.isNotEmpty()) {
            if (field.typeName == "char" || field.typeName == "uchar") {
                val nul = raw.indexOf(0).let { if (it < 0) raw.size else it }
                return String(raw, 0, nul, Charsets.UTF_8)
            }
            val width = schema.types.indices.firstOrNull { schema.types[it] == field.typeName }
                ?.let { schema.typeLengths[it] } ?: return null
            if (width !in 1..8 || raw.size % width != 0) return null
            return (raw.indices step width).map { offset -> readNumber(raw, offset, width, field.typeName, byteOrder) ?: return null }
        }
        val width = schema.types.indices.firstOrNull { schema.types[it] == field.typeName }
            ?.let { schema.typeLengths[it] } ?: return null
        if (width !in 1..8) return null
        return readNumber(raw, 0, width, field.typeName, byteOrder)
    }

    private fun readUnsigned(bytes: ByteArray, offset: Int, width: Int, order: ByteOrder): Long {
        val buffer = ByteBuffer.wrap(bytes, offset, width).order(order)
        return when (width) {
            4 -> buffer.int.toLong() and 0xffffffffL
            8 -> buffer.long
            2 -> buffer.short.toLong() and 0xffffL
            else -> buffer.get().toLong() and 0xffL
        }
    }

    private fun readNumber(bytes: ByteArray, offset: Int, width: Int, typeName: String, order: ByteOrder): Any? {
        val buffer = ByteBuffer.wrap(bytes, offset, width).order(order)
        return when {
            typeName.contains("float", ignoreCase = true) && width == 4 -> buffer.float.toDouble()
            typeName.contains("double", ignoreCase = true) && width == 8 -> buffer.double
            width == 1 -> if (typeName.startsWith("u") || typeName == "bool") {
                bytes[offset].toInt() and 0xff
            } else bytes[offset].toInt()
            width == 2 -> if (typeName.startsWith("u")) buffer.short.toInt() and 0xffff else buffer.short.toInt()
            width == 4 -> if (typeName.startsWith("u")) buffer.int.toLong() and 0xffffffffL else buffer.int
            width == 8 -> buffer.long
            else -> null
        }
    }

    private fun parseDeclarator(rawName: String): Declarator {
        if (rawName.contains('(') || rawName.contains(')')) {
            return Declarator("", 0, emptyList(), false)
        }
        val name = rawName.trim()
        val pointerDepth = name.count { it == '*' }
        val identifier = Regex("[A-Za-z_][A-Za-z0-9_]*").find(name)?.value
            ?: return Declarator("", 0, emptyList(), false)
        val dimensions = Regex("\\[([0-9]+)\\]").findAll(name).map {
            it.groupValues[1].toIntOrNull() ?: 0
        }.toList()
        if (dimensions.any { it <= 0 }) return Declarator(identifier, pointerDepth, dimensions, false)
        val residue = name.replace("*", "").replace(identifier, "").replace(Regex("\\[[0-9]+\\]"), "")
        return Declarator(identifier, pointerDepth, dimensions, residue.isEmpty())
    }

    private fun primitiveAlignment(size: Int, pointerBytes: Int): Int {
        var alignment = 1
        val cap = minOf(size, pointerBytes)
        while (alignment <= cap / 2) alignment *= 2
        return alignment
    }

    private fun align(value: Int, alignment: Int): Int {
        val remainder = value % alignment
        return if (remainder == 0) value else Math.addExact(value, alignment - remainder)
    }

    private fun unsupported(typeName: String, message: String) =
        StructLayout(typeName, 0, emptyList(), false, message)
}
