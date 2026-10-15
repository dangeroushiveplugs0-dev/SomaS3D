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
    val warnings: List<String> = emptyList(),
    /** Shape-key coordinate sets attached to this mesh after address-based linking. */
    val shapeKeys: List<SomaShapeKey> = emptyList()
)

data class SomaMeshImportResult(
    val meshes: List<SomaMesh>,
    val warnings: List<String>,
    val skippedMeshCount: Int,
    val message: String,
    val sourceAddresses: List<Long?> = emptyList()
)


/**
 * Evaluates the imported current shape-key values into one display-ready vertex list.
 * Coordinates are absolute key coordinates; relative-key deltas are added to the mesh.
 * This is a safe preview evaluator, not a replacement for Blender's driver/dependency graph.
 */
object SomaShapeKeyEvaluator {
    private const val MAX_PREVIEW_VERTICES = 500_000
    private const val MAX_COORDINATE_OPERATIONS = 20_000_000L

    fun evaluate(mesh: SomaMesh): List<SomaVector3> {
        val base = mesh.vertices
        val keys = mesh.shapeKeys
        if (base.isEmpty() || keys.isEmpty() || base.size > MAX_PREVIEW_VERTICES) return base
        if (base.size.toLong() * keys.size.toLong() > MAX_COORDINATE_OPERATIONS) return base

        val keysByName = keys.associateBy { it.name }
        var evaluated = base
        for (key in keys) {
            val value = key.value.takeIf { it.isFinite() } ?: continue
            if (value == 0.0 || key.coordinates.size != base.size) continue
            val relative = key.relativeKeyName
                ?.let { keysByName[it]?.coordinates }
                ?.takeIf { it.size == base.size }
                ?: base
            if (key.name == key.relativeKeyName) continue

            val next = ArrayList<SomaVector3>(base.size)
            for (index in base.indices) {
                val current = evaluated[index]
                val target = key.coordinates[index]
                val origin = relative[index]
                next.add(SomaVector3(
                    current.x + (target.x - origin.x) * value,
                    current.y + (target.y - origin.y) * value,
                    current.z + (target.z - origin.z) * value
                ))
            }
            if (next.any { !it.x.isFinite() || !it.y.isFinite() || !it.z.isFinite() }) return base
            evaluated = next
        }
        return evaluated
    }
}
