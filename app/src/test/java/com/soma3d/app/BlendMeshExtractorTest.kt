package com.soma3d.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendMeshExtractorTest {
    @Test
    fun normalizesVertexPositionsAndPolygonLoops() {
        val mesh = record("Mesh", mapOf("name" to "Triangle"))
        val vertices = listOf(
            record("MVert", mapOf("co" to listOf(0.0, 0.0, 0.0))),
            record("MVert", mapOf("co" to listOf(1.0, 0.0, 0.0))),
            record("MVert", mapOf("co" to listOf(0.0, 1.0, 0.0)))
        )
        val loops = listOf(
            record("MLoop", mapOf("v" to 0)),
            record("MLoop", mapOf("v" to 1)),
            record("MLoop", mapOf("v" to 2))
        )
        val polygons = listOf(record("MPoly", mapOf("loopstart" to 0, "totloop" to 3)))

        val result = BlendMeshExtractor.extract(
            listOf(BlendMeshExtractor.MeshRecords(mesh, vertices, loops, polygons))
        )

        assertEquals(1, result.meshes.size)
        assertEquals("Triangle", result.meshes.single().name)
        assertEquals(SomaVector3(1.0, 0.0, 0.0), result.meshes.single().vertices[1])
        assertEquals(listOf(0, 1, 2), result.meshes.single().polygons.single().vertexIndices)
        assertEquals(0, result.skippedMeshCount)
    }

    @Test
    fun rejectsOutOfRangeLoopIndicesAndNonFiniteVertices() {
        val mesh = record("Mesh", mapOf("name" to "Broken"))
        val vertices = listOf(record("MVert", mapOf("co" to listOf(Double.NaN, 0.0, 0.0))))
        val result = BlendMeshExtractor.extract(
            listOf(BlendMeshExtractor.MeshRecords(mesh, vertices, emptyList(), emptyList()))
        )
        assertTrue(result.meshes.isEmpty())
        assertEquals(1, result.skippedMeshCount)
        assertTrue(result.warnings.single().contains("finite"))
    }

    private fun record(type: String, fields: Map<String, Any>) =
        BlendStructDecoder.DecodedRecord(type, fields)
}
