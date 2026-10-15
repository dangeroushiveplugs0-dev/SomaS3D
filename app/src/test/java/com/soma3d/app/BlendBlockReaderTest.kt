package com.soma3d.app

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
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
        assertEquals(36L, result.blocks[0].payloadOffset)
        assertEquals("ENDB", result.blocks[1].code)
        assertNotNull(result.schema)
    }

    @Test
    fun scansModernBlenderFiveBlockStream() {
        val result = BlendBlockReader.read(ByteArrayInputStream(minimalModernBlendFile()))

        assertTrue(result.validHeader)
        assertEquals("0500", result.version)
        assertEquals(64, result.pointerBits)
        assertTrue(result.endedCleanly)
        assertEquals(2, result.blocks.size)
        assertEquals("DNA1", result.blocks[0].code)
        assertEquals(49L, result.blocks[0].payloadOffset)
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
        val bytes = "BLENDER-v400".toByteArray(StandardCharsets.US_ASCII) + byteArrayOf(1, 2, 3)
        val result = BlendBlockReader.read(ByteArrayInputStream(bytes))

        assertTrue(result.validHeader)
        assertFalse(result.endedCleanly)
        assertTrue(result.message.contains("Unexpected end"))
    }


    @Test
    fun readsScannedPayloadFromCacheWithoutLoadingWholeFile() {
        val bytes = minimalBlendFile()
        val scan = BlendBlockReader.read(ByteArrayInputStream(bytes))
        val directory = createTempDir(prefix = "blend-cache-test")
        try {
            val cached = BlendCachedFileSource.copyToCache(ByteArrayInputStream(bytes), directory)
            assertNotNull(cached)
            val source = BlendCachedFileSource.open(cached!!, 64, true)
            assertNotNull(source)
            val result = source!!.readPayload(scan.blocks.first { it.code == "DNA1" })
            assertTrue(result.message, result.success)
            assertEquals(scan.blocks.first { it.code == "DNA1" }.payloadBytes.toInt(), result.payload!!.size)
        } finally {
            directory.deleteRecursively()
        }
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


    private fun minimalModernBlendFile(): ByteArray {
        val out = ByteArrayOutputStream()
        out.write("BLENDER17-01v0500".toByteArray(StandardCharsets.US_ASCII))
        val dna = ByteArrayOutputStream()
        dna.write("SDNANAME".toByteArray(StandardCharsets.US_ASCII))
        writeInt(dna, 0)
        dna.write("TYPE".toByteArray(StandardCharsets.US_ASCII))
        writeInt(dna, 0)
        dna.write("TLEN".toByteArray(StandardCharsets.US_ASCII))
        dna.write("STRC".toByteArray(StandardCharsets.US_ASCII))
        writeInt(dna, 0)
        writeModernBlockHeader(out, "DNA1", dna.size().toLong())
        out.write(dna.toByteArray())
        writeModernBlockHeader(out, "ENDB", 0L)
        return out.toByteArray()
    }

    private fun writeModernBlockHeader(out: ByteArrayOutputStream, code: String, length: Long) {
        out.write(code.toByteArray(StandardCharsets.US_ASCII))
        writeInt(out, 0) // SDNA index
        repeat(8) { out.write(0) } // stored file address
        writeLong(out, length)
        writeLong(out, 0L) // record count
    }

    private fun writeLong(out: ByteArrayOutputStream, value: Long) {
        repeat(8) { shift -> out.write(((value ushr (shift * 8)) and 0xff).toInt()) }
    }
\n    private fun writeBlockHeader(out: ByteArrayOutputStream, code: String, length: Long) {
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
