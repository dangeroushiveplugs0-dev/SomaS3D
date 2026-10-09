package com.soma3d.app

import org.json.JSONObject

/** Geometry payload supplied by the Rust scene snapshot bridge. */
internal data class ViewportMeshData(
    val name: String,
    val vertices: List<FloatArray>,
    val polygons: List<IntArray>,
    val fromRust: Boolean
)

internal object NativeGeometry {
    private val libraryLoaded: Boolean = runCatching {
        System.loadLibrary("soma_android_bridge")
    }.isSuccess

    @JvmStatic
    private external fun sceneJson(): String

    fun loadDefaultMesh(): ViewportMeshData? {
        if (!libraryLoaded) return null
        return runCatching {
            val json = JSONObject(sceneJson())
            val positionArray = json.getJSONArray("positions")
            val polygonArray = json.getJSONArray("polygons")
            val vertices = (0 until positionArray.length()).map { index ->
                val point = positionArray.getJSONArray(index)
                floatArrayOf(
                    point.getDouble(0).toFloat(),
                    point.getDouble(1).toFloat(),
                    point.getDouble(2).toFloat()
                )
            }
            val polygons = (0 until polygonArray.length()).map { index ->
                val polygon = polygonArray.getJSONArray(index)
                IntArray(polygon.length()) { vertexIndex -> polygon.getInt(vertexIndex) }
            }
            require(vertices.isNotEmpty() && polygons.isNotEmpty()) {
                "Rust scene snapshot contained no drawable geometry"
            }
            ViewportMeshData(json.getString("name"), vertices, polygons, fromRust = true)
        }.getOrNull()
    }
}
