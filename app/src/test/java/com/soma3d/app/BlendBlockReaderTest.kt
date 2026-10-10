package com.soma3d.app

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.nio.charset.StandardCharsets
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendBlockReaderTest {
    @Test
    fun scansBlockStreamAndReadsSchema() {
        val result = BlendBlockReader.read(ByteArrayInputStream(minimalBlendFile()))

        assertTrue(result.validHeader)
        assertEquals("400", result.version)
        assertEquals(64, result.pointerBits)
        assertTrue(result.endedCleanly)
        assertEquals(2, result.blocks.size)
        assertEquals("DNA1", result.blocks[0].code)
        assertEquals("ENDB", result.blocks[1].code)
        assertNotNull(result.schema)
    }

    @Test
    fun rejectsFileWithoutBlenderHeader() {
        val result = BlendBlockReader.read(ByteArrayInputStream("not blender!!".toByteArray()))

        assertFalse(result.validHeader)
        assertTrue(result.message.contains("signature"))
    }

    @Test
    fun reportsTruncatedBlockHeader() {
        val bytes = "BLENDER_v400".toByteArray(StandardCharsets.US_ASCII) + byteArrayOf(1, 2, 3)
        val result = BlendBlockReader.read(ByteArrayInputStream(bytes))

        assertTrue(result.validHeader)
        assertFalse(result.endedCleanly)
        assertTrue(result.message.contains("Unexpected end"))
    }

    private fun minimalBlendFile(): ByteArray {
        val out = ByteArrayOutputStream()
        out.write("BLENDER_v400".toByteArray(StandardCharsets.US_ASCII))

        val dna = ByteArrayOutputStream()
        dna.write("SDNANAME".toByteArray(StandardCharsets.US_ASCII))
        writeInt(dna, 0)
        dna.write("TYPE".toByteArray(StandardCharsets.US_ASCII))
        writeInt(dna, 0)
        dna.write("TLEN".toByteArray(StandardCharsets.US_ASCII))
        dna.write("STRC".toByteArray(StandardCharsets.US_ASCII))
        writeInt(dna, 0)

        writeBlockHeader(out, "DNA1", dna.size().toLong())
        out.write(dna.toByteArray())
        writeBlockHeader(out, "ENDB", 0L)
        return out.toByteArray()
    }

    private fun writeBlockHeader(out: ByteArrayOutputStream, code: String, length: Long) {
        out.write(code.toByteArray(StandardCharsets.US_ASCII))
        writeInt(out, length.toInt())
        repeat(8) { out.write(0) }
        writeInt(out, 0)
        writeInt(out, 0)
    }

    private fun writeInt(out: ByteArrayOutputStream, value: Int) {
        out.write(value and 0xff)
        out.write((value ushr 8) and 0xff)
        out.write((value ushr 16) and 0xff)
        out.write((value ushr 24) and 0xff)
    }
}
