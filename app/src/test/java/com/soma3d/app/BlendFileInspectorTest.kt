package com.soma3d.app

import java.io.ByteArrayInputStream
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendFileInspectorTest {
    @Test
    fun recognizes32BitLittleEndianHeader() {
        val result = inspect("BLENDER-v300")

        assertTrue(result.valid)
        assertEquals("300", result.version)
        assertEquals(32, result.pointerBits)
        assertEquals(true, result.littleEndian)
    }

    @Test
    fun recognizes64BitLittleEndianHeader() {
        val result = inspect("BLENDER_v400")

        assertTrue(result.valid)
        assertEquals("400", result.version)
        assertEquals(64, result.pointerBits)
        assertEquals(true, result.littleEndian)
    }

    @Test
    fun recognizesBigEndianHeader() {
        val result = inspect("BLENDER_V280")

        assertTrue(result.valid)
        assertEquals("280", result.version)
        assertEquals(false, result.littleEndian)
    }

    @Test
    fun rejectsIncorrectSignature() {
        val result = inspect("NOTBLND-v300")

        assertFalse(result.valid)
        assertTrue(result.message.contains("signature"))
    }

    @Test
    fun rejectsTruncatedHeader() {
        val result = inspect("BLENDER-")

        assertFalse(result.valid)
        assertTrue(result.message.contains("too short"))
    }

    @Test
    fun rejectsMalformedHeaderMarkers() {
        val result = inspect("BLENDER?v300")

        assertFalse(result.valid)
        assertTrue(result.message.contains("malformed"))
    }

    @Test
    fun rejectsNonNumericVersion() {
        val result = inspect("BLENDER-v3x0")

        assertFalse(result.valid)
        assertTrue(result.message.contains("malformed"))
    }

    private fun inspect(header: String): BlendFileInspector.Inspection =
        BlendFileInspector.inspect(ByteArrayInputStream(header.toByteArray(Charsets.US_ASCII)))
}
