package com.soma3d.app

import java.io.InputStream

/**
 * Reads only the fixed 12-byte .blend header.
 *
 * This is validation and format detection, not a full Blender file parser.
 * Never executes embedded scripts, drivers, or add-ons.
 */
object BlendFileInspector {
    private const val HEADER_SIZE = 12
    private val MAGIC = "BLENDER".toByteArray(Charsets.US_ASCII)

    data class Inspection(
        val valid: Boolean,
        val version: String?,
        val pointerBits: Int?,
        val littleEndian: Boolean?,
        val fileSizeBytes: Long?,
        val message: String
    )

    fun inspect(input: InputStream, fileSizeBytes: Long? = null): Inspection {
        val header = ByteArray(HEADER_SIZE)
        var count = 0
        while (count < header.size) {
            val read = input.read(header, count, header.size - count)
            if (read < 0) break
            if (read == 0) continue
            count += read
        }

        if (count < HEADER_SIZE) {
            return Inspection(
                valid = false,
                version = null,
                pointerBits = null,
                littleEndian = null,
                fileSizeBytes = fileSizeBytes,
                message = "File is too short to contain a complete .blend header ($count/$HEADER_SIZE bytes)."
            )
        }

        for (index in MAGIC.indices) {
            if (header[index] != MAGIC[index]) {
                return Inspection(
                    valid = false,
                    version = null,
                    pointerBits = null,
                    littleEndian = null,
                    fileSizeBytes = fileSizeBytes,
                    message = "This file does not have the Blender .blend signature."
                )
            }
        }

        val pointerBits = when (header[7].toInt().toChar()) {
            '-' -> 32
            '_' -> 64
            else -> null
        }
        val littleEndian = when (header[8].toInt().toChar()) {
            'v' -> true
            'V' -> false
            else -> null
        }
        val versionChars = charArrayOf(
            header[9].toInt().toChar(),
            header[10].toInt().toChar(),
            header[11].toInt().toChar()
        )
        val version = String(versionChars)

        if (pointerBits == null || littleEndian == null || !version.all { it in '0'..'9' }) {
            return Inspection(
                valid = false,
                version = null,
                pointerBits = pointerBits,
                littleEndian = littleEndian,
                fileSizeBytes = fileSizeBytes,
                message = "Blender signature found, but the header fields are malformed or unsupported."
            )
        }

        val endianLabel = if (littleEndian) "little-endian" else "big-endian"
        val sizeLabel = fileSizeBytes?.let { " · " + formatBytes(it) } ?: ""
        return Inspection(
            valid = true,
            version = version,
            pointerBits = pointerBits,
            littleEndian = littleEndian,
            fileSizeBytes = fileSizeBytes,
            message = "Valid Blender header · version $version · $pointerBits-bit · $endianLabel$sizeLabel. Geometry parsing is the next stage."
        )
    }

    private fun formatBytes(bytes: Long): String {
        if (bytes < 1024L) return "$bytes B"
        val kib = bytes / 1024.0
        if (kib < 1024.0) return String.format(java.util.Locale.ROOT, "%.1f KiB", kib)
        return String.format(java.util.Locale.ROOT, "%.1f MiB", kib / 1024.0)
    }
}
