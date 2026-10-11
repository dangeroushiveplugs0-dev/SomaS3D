package com.soma3d.app

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendCustomDataInspectorTest {
    @Test
    fun reportsLayerNamesTypeIdsAndWhetherPayloadAddressExists() {
        val records = listOf(
            BlendStructDecoder.DecodedRecord("CustomDataLayer", mapOf("name" to ".position", "type" to 48, "data" to 0x2000L)),
            BlendStructDecoder.DecodedRecord("CustomDataLayer", mapOf("name" to ".corner_vert", "type" to 11, "data" to 0L)),
            BlendStructDecoder.DecodedRecord("CustomDataLayer", emptyMap())
        )
        val layers = BlendCustomDataInspector.summarizeLayerRecords("vertex", records)
        assertEquals(3, layers.size)
        assertEquals(".position", layers[0].name)
        assertEquals(48, layers[0].typeId)
        assertTrue(layers[0].hasDataAddress)
        assertEquals(".corner_vert", layers[1].name)
        assertFalse(layers[1].hasDataAddress)
        assertEquals("(unnamed layer)", layers[2].name)
        assertEquals(null, layers[2].typeId)
    }

    @Test
    fun decodesInlineCustomDataStructWithinMeshRecord() {
        val schema = BlendBlockReader.Schema(
            names = listOf("type", "name", "data", "layers", "totlayer", "maxlayer", "vdata"),
            types = listOf("char", "int", "CustomDataLayer", "CustomData", "Mesh"),
            typeLengths = listOf(1, 4, 16, 16, 16),
            structs = listOf(
                BlendBlockReader.SchemaStruct("CustomDataLayer", listOf(
                    BlendBlockReader.SchemaField("int", "type"),
                    BlendBlockReader.SchemaField("char", "name[4]"),
                    BlendBlockReader.SchemaField("CustomDataLayer", "*data")
                )),
                BlendBlockReader.SchemaStruct("CustomData", listOf(
                    BlendBlockReader.SchemaField("CustomDataLayer", "*layers"),
                    BlendBlockReader.SchemaField("int", "totlayer"),
                    BlendBlockReader.SchemaField("int", "maxlayer")
                )),
                BlendBlockReader.SchemaStruct("Mesh", listOf(
                    BlendBlockReader.SchemaField("CustomData", "vdata")
                ))
            )
        )
        val payload = ByteBuffer.allocate(16).order(ByteOrder.LITTLE_ENDIAN)
            .putLong(0x1000L).putInt(1).putInt(1).array()
        val result = BlendStructDecoder.decodeRecords(
            schema, sdnaIndex = 2, count = 1, payload = payload,
            pointerBits = 64, littleEndian = true
        )
        assertTrue(result.success)
        val vertexData = result.records.single().fields["vdata"] as Map<*, *>
        assertEquals(0x1000L, vertexData["layers"])
        assertEquals(1, vertexData["totlayer"])
        assertEquals(1, vertexData["maxlayer"])
    }
}
