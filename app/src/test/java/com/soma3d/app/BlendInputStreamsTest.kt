package com.soma3d.app

import com.github.luben.zstd.ZstdOutputStream
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Test

class BlendInputStreamsTest {
    @Test
    fun readsPlainBlendHeaderWithoutChangingBytes() {
        val bytes = "BLENDER_v400".toByteArray(Charsets.US_ASCII)
        BlendInputStreams.open(ByteArrayInputStream(bytes)).use { decoded ->
            assertArrayEquals(bytes, decoded.readBytes())
        }
    }

    @Test
    fun decompressesZstandardWrappedBlendHeader() {
        val original = "BLENDER_v400".toByteArray(Charsets.US_ASCII)
        val compressed = ByteArrayOutputStream().also { output ->
            ZstdOutputStream(output).use { it.write(original) }
        }.toByteArray()

        BlendInputStreams.open(ByteArrayInputStream(compressed)).use { decoded ->
            assertArrayEquals(original, decoded.readBytes())
        }
    }
}
