package com.soma3d.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SomaObjectTransformTest {
    @Test
    fun appliesTranslationAndScaleToVertices() {
        val mesh = SomaMesh(
            name = "Triangle",
            vertices = listOf(SomaVector3(1.0, 2.0, 3.0)),
            polygons = emptyList()
        )
        val matrix = listOf(
            2.0, 0.0, 0.0, 0.0,
            0.0, 3.0, 0.0, 0.0,
            0.0, 0.0, 4.0, 0.0,
            10.0, 20.0, 30.0, 1.0
        )

        val result = SomaObjectTransform.apply(mesh, matrix)

        assertTrue(result.applied)
        assertEquals(SomaVector3(12.0, 26.0, 42.0), result.mesh!!.vertices.single())
        assertEquals("Triangle", result.mesh.name)
    }

    @Test
    fun rejectsMissingMalformedAndNonFiniteMatrices() {
        val mesh = SomaMesh("Mesh", listOf(SomaVector3(0.0, 0.0, 0.0)), emptyList())

        val missing = SomaObjectTransform.apply(mesh, null)
        val short = SomaObjectTransform.apply(mesh, listOf(1.0, 2.0))
        val nonFinite = SomaObjectTransform.apply(mesh, List(16) { if (it == 4) Double.NaN else 0.0 })

        assertFalse(missing.applied)
        assertFalse(short.applied)
        assertFalse(nonFinite.applied)
        assertNull(missing.mesh)
        assertNull(short.mesh)
        assertNull(nonFinite.mesh)
    }
}
