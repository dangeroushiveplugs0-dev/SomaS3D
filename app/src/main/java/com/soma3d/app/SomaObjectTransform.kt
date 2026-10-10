package com.soma3d.app

/**
 * Applies a decoded Blender Object.obmat matrix to normalized mesh vertices.
 *
 * Blender stores this 4x4 matrix as 16 row-major floats with translation in
 * indices 12, 13, and 14. This helper accepts only a complete finite matrix;
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
        if (kotlin.math.abs(m[3]) > 1e-6 || kotlin.math.abs(m[7]) > 1e-6 ||
            kotlin.math.abs(m[11]) > 1e-6 || kotlin.math.abs(m[15] - 1.0) > 1e-6) {
            return Result(null, false, "Object matrix is not a supported affine transform.")
        }
        val transformed = mesh.vertices.map { point ->
            val x = m[0] * point.x + m[4] * point.y + m[8] * point.z + m[12]
            val y = m[1] * point.x + m[5] * point.y + m[9] * point.z + m[13]
            val z = m[2] * point.x + m[6] * point.y + m[10] * point.z + m[14]
            SomaVector3(x, y, z)
        }
        if (transformed.any { !it.x.isFinite() || !it.y.isFinite() || !it.z.isFinite() }) {
            return Result(null, false, "Object transform produced non-finite coordinates.")
        }
        return Result(mesh.copy(vertices = transformed), true, "Object transform applied.")
    }
}
