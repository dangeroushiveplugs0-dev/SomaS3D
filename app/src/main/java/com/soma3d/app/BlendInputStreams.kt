package com.soma3d.app

import com.github.luben.zstd.ZstdInputStream
import java.io.InputStream
import java.io.PushbackInputStream

/**
 * Blender may store a .blend file as a Zstandard frame. Detect the frame
 * without assuming document-provider streams support mark/reset.
 */
internal object BlendInputStreams {
    private val ZSTD_MAGIC = byteArrayOf(0x28, 0xB5.toByte(), 0x2F, 0xFD.toByte())

    fun open(input: InputStream): InputStream {
        val pushback = PushbackInputStream(input, ZSTD_MAGIC.size)
        val prefix = ByteArray(ZSTD_MAGIC.size)
        var count = 0
        while (count < prefix.size) {
            val read = pushback.read(prefix, count, prefix.size - count)
            if (read < 0) break
            if (read == 0) continue
            count += read
        }
        if (count > 0) pushback.unread(prefix, 0, count)
        val compressed = count == ZSTD_MAGIC.size && prefix.indices.all { prefix[it] == ZSTD_MAGIC[it] }
        return if (compressed) ZstdInputStream(pushback) else pushback
    }
}
