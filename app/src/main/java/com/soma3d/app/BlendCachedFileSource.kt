package com.soma3d.app

import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.RandomAccessFile
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Cache-backed, bounded payload access for a previously scanned .blend file.
 * The cache avoids assuming that Android document-provider streams are seekable.
 */
class BlendCachedFileSource private constructor(
    private val file: File,
    private val pointerBits: Int,
    private val littleEndian: Boolean
) {
    data class ReadResult(val payload: ByteArray?, val message: String) {
        val success: Boolean get() = payload != null
    }

    fun readPayload(block: BlendBlockReader.BlockSummary, maxPayloadBytes: Int = DEFAULT_MAX_PAYLOAD): ReadResult {
        if (maxPayloadBytes < 0 || block.payloadBytes < 0L || block.payloadBytes > maxPayloadBytes.toLong()) {
            return ReadResult(null, "Payload exceeds the selected safe read limit.")
        }
        if (block.payloadOffset < headerBytes(pointerBits).toLong()) {
            return ReadResult(null, "Block has no valid recorded payload offset.")
        }
        val length = block.payloadBytes.toInt()
        return try {
            RandomAccessFile(file, "r").use { input ->
                val headerSize = headerBytes(pointerBits)
                val headerOffset = block.payloadOffset - headerSize
                if (headerOffset < FILE_HEADER_BYTES || block.payloadOffset > input.length() ||
                    block.payloadBytes > input.length() - block.payloadOffset) {
                    return ReadResult(null, "Payload range is outside the cached file.")
                }
                input.seek(headerOffset)
                val header = ByteArray(headerSize)
                input.readFully(header)
                val code = String(header, 0, 4, Charsets.US_ASCII)
                val storedLength = readU32(header, 4, littleEndian)
                if (code != block.code || storedLength != block.payloadBytes) {
                    return ReadResult(null, "Cached block header does not match the scanned block.")
                }
                val payload = ByteArray(length)
                input.seek(block.payloadOffset)
                input.readFully(payload)
                ReadResult(payload, "Read $length payload bytes from cached block ${block.code}.")
            }
        } catch (_: Exception) {
            ReadResult(null, "Could not read the requested cached payload.")
        }
    }

    fun cachedFile(): File = file

    companion object {
        private const val FILE_HEADER_BYTES = 12L
        private const val DEFAULT_MAX_PAYLOAD = 16 * 1024 * 1024

        fun open(file: File, pointerBits: Int, littleEndian: Boolean): BlendCachedFileSource? {
            if (!file.isFile || !file.canRead() || (pointerBits != 32 && pointerBits != 64)) return null
            return BlendCachedFileSource(file, pointerBits, littleEndian)
        }

        /** Copies a document stream to a bounded temporary cache file without loading it into RAM. */
        fun copyToCache(input: java.io.InputStream, cacheDirectory: File, maxFileBytes: Long = 1024L * 1024L * 1024L): File? {
            if (maxFileBytes <= 0L || !cacheDirectory.exists() && !cacheDirectory.mkdirs()) return null
            val temporary = File.createTempFile("blend-import-", ".partial", cacheDirectory)
            val destination = File(cacheDirectory, temporary.name.removeSuffix(".partial") + ".blendcache")
            var total = 0L
            return try {
                FileOutputStream(temporary).use { output ->
                    val buffer = ByteArray(64 * 1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        if (read == 0) continue
                        if (total > maxFileBytes - read.toLong()) {
                            temporary.delete()
                            return null
                        }
                        output.write(buffer, 0, read)
                        total += read
                    }
                    output.fd.sync()
                }
                if (!temporary.renameTo(destination)) {
                    temporary.copyTo(destination, overwrite = true)
                    temporary.delete()
                }
                destination
            } catch (_: Exception) {
                temporary.delete()
                destination.delete()
                null
            }
        }

        private fun headerBytes(pointerBits: Int): Int = 4 + 4 + pointerBits / 8 + 4 + 4

        private fun readU32(bytes: ByteArray, offset: Int, littleEndian: Boolean): Long {
            val buffer = ByteBuffer.wrap(bytes, offset, 4)
                .order(if (littleEndian) ByteOrder.LITTLE_ENDIAN else ByteOrder.BIG_ENDIAN)
            return buffer.int.toLong() and 0xffffffffL
        }
    }
}
