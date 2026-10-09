package com.soma3d.app

import org.json.JSONObject

internal data class ViewportMeshData(
    val id: Long,
    val name: String,
    val vertices: List<FloatArray>,
    val polygons: List<IntArray>
)

internal data class ViewportSceneData(
    val meshes: List<ViewportMeshData>,
    val activeObjectId: Long?,
    val fromRust: Boolean
)

internal object NativeGeometry {
    private val libraryLoaded = runCatching {
        System.loadLibrary("soma_android_bridge")
    }.isSuccess

    private external fun sceneJson(): String
    private external fun nativeAddCube(): Long
    private external fun nativeAddSphere(): Long
    private external fun nativeSetActiveObject(id: Long): Long

    fun loadScene(): ViewportSceneData? {
        if (!libraryLoaded) return null
        return runCatching {
            val json = JSONObject(sceneJson())
            require(!json.has("error")) { json.optString("error", "Rust scene unavailable") }
            val array = json.getJSONArray("meshes")
            val meshes = (0 until array.length()).map { index ->
                val item = array.getJSONObject(index)
                val positionArray = item.getJSONArray("positions")
                val polygonArray = item.getJSONArray("polygons")
                val vertices = (0 until positionArray.length()).map { vertexIndex ->
                    val point = positionArray.getJSONArray(vertexIndex)
                    floatArrayOf(point.getDouble(0).toFloat(), point.getDouble(1).toFloat(), point.getDouble(2).toFloat())
                }
                val polygons = (0 until polygonArray.length()).map { faceIndex ->
                    val polygon = polygonArray.getJSONArray(faceIndex)
                    IntArray(polygon.length()) { vertexIndex -> polygon.getInt(vertexIndex) }
                }
                ViewportMeshData(item.getLong("id"), item.getString("name"), vertices, polygons)
            }
            ViewportSceneData(meshes, if (json.isNull("active")) null else json.getLong("active"), true)
        }.getOrNull()
    }

    fun addCube(): Boolean = libraryLoaded && runCatching { nativeAddCube() >= 0L }.getOrDefault(false)
    fun addSphere(): Boolean = libraryLoaded && runCatching { nativeAddSphere() >= 0L }.getOrDefault(false)
    fun selectObject(id: Long): Boolean = libraryLoaded && runCatching { nativeSetActiveObject(id) >= 0L }.getOrDefault(false)
}
