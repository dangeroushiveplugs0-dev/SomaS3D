package com.soma3d.app

/**
 * Applies a decoded Blender Object.obmat matrix to normalized mesh vertices.
 *
 * Blender stores this 4x4 matrix as 16 row-major floats with translation in
 * indices 3, 7, and 11. This helper accepts only a complete finite matrix;
 * absent or malformed matrices are never silently treated as identity.
 */
object SomaObjectTransform {
    data class Result(
        val mesh: SomaMesh?,
        val applied: Boolean,
        val message: String
    )

    fun apply(mesh: SomaMesh, decodedMatrix: Any?): Result {
        val values = decodedMatrix as? List<*>
            ?: return Result(null, false, "Object transform is unavailable or not a numeric array.")
        if (values.size != 16) {
            return Result(null, false, "Object transform must contain exactly 16 matrix values.")
        }
        val matrix = values.map { value ->
            (value as? Number)?.toDouble()
        }
        if (matrix.any { it == null || !it.isFinite() }) {
            return Result(null, false, "Object transform contains a non-numeric or non-finite value.")
        }
        val m = matrix.filterNotNull()
        val transformed = mesh.vertices.map { point ->
            val x = m[0] * point.x + m[1] * point.y + m[2] * point.z + m[3]
            val y = m[4] * point.x + m[5] * point.y + m[6] * point.z + m[7]
            val z = m[8] * point.x + m[9] * point.y + m[10] * point.z + m[11]
            SomaVector3(x, y, z)
        }
        if (transformed.any { !it.x.isFinite() || !it.y.isFinite() || !it.z.isFinite() }) {
            return Result(null, false, "Object transform produced non-finite coordinates.")
        }
        return Result(mesh.copy(vertices = transformed), true, "Object transform applied.")
    }
}
