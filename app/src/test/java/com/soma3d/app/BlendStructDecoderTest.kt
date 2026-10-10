package com.soma3d.app

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendStructDecoderTest {
    @Test
    fun computesAndDecodesValidatedFloatStruct() {
        val schema = pointSchema()
        val layout = BlendStructDecoder.layout(schema, "Point", 64)
        assertTrue(layout.supported)
        assertEquals(12, layout.byteSize)
        assertEquals(listOf(0, 4, 8), layout.fields.map { it.offset })

        val payload = ByteBuffer.allocate(12).order(ByteOrder.LITTLE_ENDIAN)
            .putFloat(1.25f).putFloat(-2.0f).putFloat(3.5f).array()
        val result = BlendStructDecoder.decodeRecords(schema, 0, 1, payload, 64, true)

        assertTrue(result.success)
        assertEquals(1.25, result.records.single().fields["x"] as Double, 0.00001)
        assertEquals(-2.0, result.records.single().fields["y"] as Double, 0.00001)
        assertEquals(3.5, result.records.single().fields["z"] as Double, 0.00001)
    }

    @Test
    fun refusesLayoutThatDoesNotMatchSdnaDeclaredSize() {
        val original = pointSchema()
        val wrong = original.copy(
            typeLengths = listOf(1, 4, 16)
        )
        val layout = BlendStructDecoder.layout(wrong, "Point", 64)
        assertFalse(layout.supported)
        assertTrue(layout.message.contains("TLEN"))
    }

    @Test
    fun refusesPayloadShorterThanDeclaredRecords() {
        val result = BlendStructDecoder.decodeRecords(
            pointSchema(), 0, 2, ByteArray(12), 64, true
        )
        assertFalse(result.success)
        assertTrue(result.message.contains("too short"))
    }

    private fun pointSchema() = BlendBlockReader.Schema(
        names = listOf("x", "y", "z"),
        types = listOf("char", "float", "Point"),
        typeLengths = listOf(1, 4, 12),
        structs = listOf(
            BlendBlockReader.SchemaStruct(
                "Point",
                listOf(
                    BlendBlockReader.SchemaField("float", "x"),
                    BlendBlockReader.SchemaField("float", "y"),
                    BlendBlockReader.SchemaField("float", "z")
                )
            )
        )
    )
}

class BlendBlockIndexTest {
    @Test
    fun findsAddressesInsideKnownBlockRanges() {
        val block = BlendBlockReader.BlockSummary("DATA", 32, 1000, 0, 1)
        val result = BlendBlockIndex.create(listOf(block))
        assertEquals(1, result.index.entries().size)
        assertEquals("DATA", result.index.findContaining(1000)?.block?.code)
        assertEquals("DATA", result.index.findContaining(1031)?.block?.code)
        assertNull(result.index.findContaining(1032))
        assertNull(result.index.findContaining(0))
    }

    @Test
    fun rejectsOverlappingRangesRatherThanChoosingAmbiguously() {
        val first = BlendBlockReader.BlockSummary("DATA", 32, 1000, 0, 1)
        val second = BlendBlockReader.BlockSummary("DATA", 16, 1020, 0, 1)
        val result = BlendBlockIndex.create(listOf(first, second))
        assertEquals(1, result.rejectedBlocks)
        assertEquals(1, result.index.entries().size)
        assertEquals("DATA", result.index.findContaining(1025)?.block?.code)
    }
}
