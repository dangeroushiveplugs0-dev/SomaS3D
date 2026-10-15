package com.soma3d.app

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class BlendShapeKeyImporterTest {
    @Test
    fun decodesFiniteFloat3Coordinates() {
        val bytes = ByteBuffer.allocate(24).order(ByteOrder.LITTLE_ENDIAN)
            .putFloat(1f).putFloat(2f).putFloat(3f)
            .putFloat(-1f).putFloat(0.5f).putFloat(0f)
            .array()

        val result = BlendShapeKeyImporter.decodeFloat3(bytes, 2, true)

        assertEquals(listOf(SomaVector3(1.0, 2.0, 3.0), SomaVector3(-1.0, 0.5, 0.0)), result)
    }

    @Test
    fun rejectsWrongLengthAndNonFiniteCoordinates() {
        assertNull(BlendShapeKeyImporter.decodeFloat3(ByteArray(11), 1, true))
        val bytes = ByteBuffer.allocate(12).order(ByteOrder.LITTLE_ENDIAN)
            .putFloat(Float.NaN).putFloat(0f).putFloat(0f).array()
        assertNull(BlendShapeKeyImporter.decodeFloat3(bytes, 1, true))
    }
}
