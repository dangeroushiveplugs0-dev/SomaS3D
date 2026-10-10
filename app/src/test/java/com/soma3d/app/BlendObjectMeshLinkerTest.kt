package com.soma3d.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendObjectMeshLinkerTest {
    private val identity = listOf(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    )

    private fun translated(x: Double) = identity.toMutableList().also { it[3] = x }

    private fun objectRecord(name: String, address: Long, matrix: List<Double>) =
        BlendStructDecoder.DecodedRecord(
            typeName = "Object",
            fields = mapOf("name" to name, "data" to address, "obmat" to matrix)
        )

    private fun normalized() = SomaMeshImportResult(
        meshes = listOf(SomaMesh(
            name = "SharedMesh",
            vertices = listOf(SomaVector3(1.0, 2.0, 3.0)),
            polygons = emptyList()
        )),
        warnings = emptyList(),
        skippedMeshCount = 0,
        message = "ok",
        sourceAddresses = listOf(0x1000L)
    )

    @Test
    fun linksByDataAddressAndAppliesPerObjectTransformsForSharedMesh() {
        val result = BlendObjectMeshLinker.link(
            listOf(
                objectRecord("Left", 0x1000L, translated(-5.0)),
                objectRecord("Right", 0x1000L, translated(8.0))
            ),
            normalized()
        )

        assertEquals(2, result.linkedObjectCount)
        assertEquals(2, result.meshes.size)
        assertEquals("Left", result.meshes[0].name)
        assertEquals("Right", result.meshes[1].name)
        assertEquals(SomaVector3(-4.0, 2.0, 3.0), result.meshes[0].vertices.single())
        assertEquals(SomaVector3(9.0, 2.0, 3.0), result.meshes[1].vertices.single())
        assertFalse(result.usedMeshFallback)
    }

    @Test
    fun doesNotLinkByMatchingNameWhenDataAddressIsUnknown() {
        val source = normalized().copy(
            meshes = listOf(normalized().meshes.single().copy(name = "SameName")),
            sourceAddresses = listOf(0x1000L)
        )
        val result = BlendObjectMeshLinker.link(
            listOf(objectRecord("SameName", 0x2000L, identity)),
            source
        )

        assertEquals(0, result.linkedObjectCount)
        assertTrue(result.usedMeshFallback)
        assertEquals("SharedMesh", result.meshes.single().name)
        assertTrue(result.warnings.any { it.contains("does not resolve") })
    }

    @Test
    fun malformedObjectTransformIsReportedAndMeshFallbackRemainsVisible() {
        val result = BlendObjectMeshLinker.link(
            listOf(objectRecord("Broken", 0x1000L, listOf(1.0, 2.0))),
            normalized()
        )

        assertEquals(0, result.linkedObjectCount)
        assertEquals(1, result.skippedObjectCount)
        assertTrue(result.usedMeshFallback)
        assertTrue(result.warnings.any { it.contains("transform was not applied") })
    }
}
