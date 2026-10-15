package com.soma3d.app

import java.io.InputStream

/** Validates legacy 12-byte and modern 17-byte Blender file headers. */
object BlendFileInspector {
    private const val LEGACY_HEADER_SIZE = 12
    private const val MODERN_HEADER_SIZE = 17
    private val MAGIC = "BLENDER".toByteArray(Charsets.US_ASCII)

    data class Inspection(
        val valid: Boolean,
        val version: String?,
        val pointerBits: Int?,
        val littleEndian: Boolean?,
        val fileSizeBytes: Long?,
        val message: String,
        val fileFormatVersion: Int = 0
    )

    fun inspect(input: InputStream, fileSizeBytes: Long? = null): Inspection {
        val decodedInput = BlendInputStreams.open(input)
        val initial = readUpTo(decodedInput, LEGACY_HEADER_SIZE)
        if (initial.size < LEGACY_HEADER_SIZE) {
            return invalid(fileSizeBytes, "File is too short to contain a complete .blend header (${initial.size}/$LEGACY_HEADER_SIZE bytes), including after checking for Zstandard compression.")
        }
        if (!initial.copyOfRange(0, 7).contentEquals(MAGIC)) {
            return invalid(fileSizeBytes, "This file does not have the Blender .blend signature, either directly or after Zstandard decompression.")
        }

        val modern = initial[7].toInt().toChar().isDigit() && initial[8].toInt().toChar().isDigit()
        val header = if (modern) initial + readUpTo(decodedInput, MODERN_HEADER_SIZE - LEGACY_HEADER_SIZE) else initial
        if (modern) {
            if (header.size < MODERN_HEADER_SIZE) {
                return invalid(fileSizeBytes, "Blender 5.0+ header is truncated (${header.size}/$MODERN_HEADER_SIZE bytes).")
            }
            val headerSize = String(header, 7, 2, Charsets.US_ASCII).toIntOrNull()
            val formatVersion = String(header, 10, 2, Charsets.US_ASCII).toIntOrNull()
            val endian = header[12].toInt().toChar()
            val version = String(header, 13, 4, Charsets.US_ASCII)
            if (headerSize != MODERN_HEADER_SIZE || header[9].toInt().toChar() != '-' ||
                formatVersion != 1 || endian != 'v' || !version.all { it in '0'..'9' }) {
                return invalid(fileSizeBytes, "Blender signature found, but the modern header fields are malformed or unsupported.", 64)
            }
            val label = formatBytesSuffix(fileSizeBytes)
            return Inspection(
                true, version, 64, true, fileSizeBytes,
                "Valid Blender 5.0+ header · version $version · 64-bit · little-endian$label.",
                fileFormatVersion = formatVersion
            )
        }

        val pointerBits = when (header[7].toInt().toChar()) {
            '-' -> 64
            '_' -> 32
            else -> null
        }
        val littleEndian = when (header[8].toInt().toChar()) {
            'v' -> true
            'V' -> false
            else -> null
        }
        val version = String(header, 9, 3, Charsets.US_ASCII)
        if (pointerBits == null || littleEndian == null || !version.all { it in '0'..'9' }) {
            return invalid(fileSizeBytes, "Blender signature found, but the header fields are malformed or unsupported.", pointerBits)
        }
        val endianLabel = if (littleEndian) "little-endian" else "big-endian"
        val sizeLabel = formatBytesSuffix(fileSizeBytes)
        return Inspection(
            true, version, pointerBits, littleEndian, fileSizeBytes,
            "Valid Blender header · version $version · $pointerBits-bit · $endianLabel$sizeLabel.",
            fileFormatVersion = 0
        )
    }

    private fun readUpTo(input: InputStream, size: Int): ByteArray {
        val result = ByteArray(size)
        var count = 0
        while (count < size) {
            val read = input.read(result, count, size - count)
            if (read < 0) break
            if (read == 0) continue
            count += read
        }
        return result.copyOf(count)
    }

    private fun invalid(fileSizeBytes: Long?, message: String, pointerBits: Int? = null) =
        Inspection(false, null, pointerBits, null, fileSizeBytes, message)

    private fun formatBytesSuffix(bytes: Long?): String =
        bytes?.let { " · " + formatBytes(it) } ?: ""

    private fun formatBytes(bytes: Long): String {
        if (bytes < 1024L) return "$bytes B"
        val kib = bytes / 1024.0
        if (kib < 1024.0) return String.format(java.util.Locale.ROOT, "%.1f KiB", kib)
        return String.format(java.util.Locale.ROOT, "%.1f MiB", kib / 1024.0)
    }
}
