package com.soma3d.app

/**
 * Version-independent mesh representation used after Blender datablocks have been decoded.
 * Blender-specific pointer values and SDNA field layouts must not escape into this layer.
 */
data class SomaVector3(val x: Double, val y: Double, val z: Double)

data class SomaPolygon(
    /** Indices into SomaMesh.vertices, in winding order. */
    val vertexIndices: List<Int>
)

data class SomaMesh(
    val name: String,
    val vertices: List<SomaVector3>,
    val polygons: List<SomaPolygon>,
    val warnings: List<String> = emptyList()
)

data class SomaMeshImportResult(
    val meshes: List<SomaMesh>,
    val warnings: List<String>,
    val skippedMeshCount: Int,
    val message: String,
    val sourceAddresses: List<Long?> = emptyList()
)
