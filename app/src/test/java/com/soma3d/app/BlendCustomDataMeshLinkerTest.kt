package com.soma3d.app

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertNotNull
import org.junit.Test

class BlendCustomDataMeshLinkerTest {
    @Test
    fun decodesFloat3PositionsInDeclaredByteOrder() {
        val bytes = ByteBuffer.allocate(24).order(ByteOrder.LITTLE_ENDIAN)
            .putFloat(1f).putFloat(2f).putFloat(3f)
            .putFloat(-4f).putFloat(5.5f).putFloat(6f)
            .array()
        val result = BlendCustomDataMeshLinker.decodeFloat3Payload(bytes, 2, true)
        assertNotNull(result)
        assertEquals(SomaVector3(1.0, 2.0, 3.0), result!![0])
        assertEquals(SomaVector3(-4.0, 5.5, 6.0), result[1])
    }

    @Test
    fun rejectsWrongStrideAndNonFinitePositions() {
        assertNull(BlendCustomDataMeshLinker.decodeFloat3Payload(ByteArray(11), 1, true))
        val bytes = ByteBuffer.allocate(12).order(ByteOrder.LITTLE_ENDIAN)
            .putFloat(Float.NaN).putFloat(0f).putFloat(1f).array()
        assertNull(BlendCustomDataMeshLinker.decodeFloat3Payload(bytes, 1, true))
    }

    @Test
    fun buildsFacesFromMonotonicCornerOffsets() {
        val polygons = BlendCustomDataMeshLinker.buildPolygons(
            offsets = listOf(0, 3, 7),
            cornerVertices = listOf(0, 1, 2, 2, 3, 4, 5),
            vertexCount = 6
        )
        assertNotNull(polygons)
        assertEquals(listOf(0, 1, 2), polygons!![0].vertexIndices)
        assertEquals(listOf(2, 3, 4, 5), polygons[1].vertexIndices)
    }

    @Test
    fun rejectsInvalidOffsetsAndVertexIndices() {
        assertNull(BlendCustomDataMeshLinker.buildPolygons(listOf(0, 4, 3), listOf(0, 1, 2), 3))
        assertNull(BlendCustomDataMeshLinker.buildPolygons(listOf(0, 3), listOf(0, 1, 9), 3))
        assertNull(BlendCustomDataMeshLinker.buildPolygons(listOf(1, 3), listOf(0, 1, 2), 3))
    }
}
