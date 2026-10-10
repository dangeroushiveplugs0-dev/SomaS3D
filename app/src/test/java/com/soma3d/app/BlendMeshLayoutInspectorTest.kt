package com.soma3d.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendMeshLayoutInspectorTest {
    private fun schema(meshFields: List<String>, extraTypes: Set<String> = emptySet()) =
        BlendBlockReader.Schema(
            names = emptyList(),
            types = emptyList(),
            typeLengths = emptyList(),
            structs = listOf(
                BlendBlockReader.SchemaStruct(
                    "Mesh",
                    meshFields.map { BlendBlockReader.SchemaField("int", it) }
                )
            ) + extraTypes.map { BlendBlockReader.SchemaStruct(it, emptyList()) }
        )

    @Test
    fun identifiesLegacyDirectArrayLayoutFromSdnaFields() {
        val report = BlendMeshLayoutInspector.inspect(schema(
            listOf("*mvert", "*mloop", "*mpoly", "totvert", "totloop", "totpoly")
        ))

        assertEquals(BlendMeshLayoutInspector.Kind.LEGACY_DIRECT_ARRAYS, report.kind)
        assertTrue(report.meshFields.contains("mvert"))
    }

    @Test
    fun detectsCustomDataLayoutWithoutClaimingGeometrySupport() {
        val report = BlendMeshLayoutInspector.inspect(schema(
            listOf("vdata", "edata", "ldata", "pdata"),
            setOf("CustomData", "CustomDataLayer")
        ))

        assertEquals(BlendMeshLayoutInspector.Kind.CUSTOM_DATA_LAYOUT, report.kind)
        assertTrue(report.explanation.contains("geometry remains unsupported"))
    }

    @Test
    fun unknownLayoutFailsClosed() {
        val report = BlendMeshLayoutInspector.inspect(schema(listOf("unknownPayload")))

        assertEquals(BlendMeshLayoutInspector.Kind.INCOMPLETE_OR_UNKNOWN, report.kind)
        assertTrue(report.explanation.contains("will not be guessed"))
    }
}
