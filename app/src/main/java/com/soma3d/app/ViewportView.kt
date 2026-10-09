package com.soma3d.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Color
import android.view.MotionEvent
import android.view.View
import kotlin.math.*

/** Canvas viewport rendering Rust scene objects while preserving the tested camera controls. */
class ViewportView(context: Context) : View(context) {
    private data class V3(val x: Float, val y: Float, val z: Float)
    private data class P2(val x: Float, val y: Float, val depth: Float)
    private data class ProjectedFace(val meshId: Long, val depth: Float, val points: List<P2>)

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private var scene = NativeGeometry.loadScene() ?: fallbackScene()
    private var yaw = -0.65f
    private var pitch = 0.42f
    private var zoom = 1f
    private var showGrid = true
    private var showEdges = true
    private var downX = 0f
    private var downY = 0f
    private var lastX = 0f
    private var lastY = 0f
    private var lastPinch = 0f
    private var moved = false
    private var multiTouch = false
    private val touchSlop = dp(8f)

    fun resetView() {
        yaw = -0.65f
        pitch = 0.42f
        zoom = 1f
        invalidate()
    }

    fun toggleGrid() { showGrid = !showGrid; invalidate() }
    fun toggleEdges() { showEdges = !showEdges; invalidate() }

    fun addCube() {
        if (NativeGeometry.addCube()) refreshScene()
    }

    fun addSphere() {
        if (NativeGeometry.addSphere()) refreshScene()
    }

    private fun refreshScene() {
        scene = NativeGeometry.loadScene() ?: scene
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        canvas.drawColor(Color.rgb(22, 25, 31))
        val cx = width * 0.5f
        val cy = height * 0.53f
        val focal = min(width, height) * 0.92f * zoom
        if (showGrid) drawGrid(canvas, cx, cy, focal)
        drawAxes(canvas, cx, cy, focal)
        scene.meshes.forEachIndexed { index, mesh ->
            drawMesh(canvas, mesh, index, cx, cy, focal)
        }
        drawGizmo(canvas)
        paint.style = Paint.Style.FILL
        paint.color = Color.rgb(196, 205, 219)
        paint.textSize = dp(11f)
        val activeName = scene.meshes.firstOrNull { it.id == scene.activeObjectId }?.name ?: "No Selection"
        val source = if (scene.fromRust) "Rust Scene" else "Preview Mesh"
        canvas.drawText("$activeName  |  $source  |  ${scene.meshes.size} object(s)", dp(16f), height - dp(62f), paint)
    }

    private fun rotate(v: V3): V3 {
        val cy = cos(yaw)
        val sy = sin(yaw)
        val cp = cos(pitch)
        val sp = sin(pitch)
        val x1 = v.x * cy - v.z * sy
        val z1 = v.x * sy + v.z * cy
        return V3(x1, v.y * cp - z1 * sp, v.y * sp + z1 * cp)
    }

    private fun project(v: V3, cx: Float, cy: Float, focal: Float): P2? {
        val r = rotate(v)
        val depth = r.z + 5.2f
        if (depth <= 0.1f) return null
        return P2(cx + r.x * focal / depth, cy - r.y * focal / depth, depth)
    }

    private fun drawGrid(canvas: Canvas, cx: Float, cy: Float, focal: Float) {
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = dp(0.7f)
        for (i in -12..12) {
            paint.color = if (i % 4 == 0) Color.rgb(65, 75, 91) else Color.rgb(43, 50, 62)
            drawWorldLine(canvas, V3(i.toFloat(), -1.05f, -12f), V3(i.toFloat(), -1.05f, 12f), cx, cy, focal)
            drawWorldLine(canvas, V3(-12f, -1.05f, i.toFloat()), V3(12f, -1.05f, i.toFloat()), cx, cy, focal)
        }
    }

    private fun drawAxes(canvas: Canvas, cx: Float, cy: Float, focal: Float) {
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = dp(1.5f)
        paint.color = Color.rgb(221, 82, 82)
        drawWorldLine(canvas, V3(-4f, -1.04f, 0f), V3(4f, -1.04f, 0f), cx, cy, focal)
        paint.color = Color.rgb(86, 191, 119)
        drawWorldLine(canvas, V3(0f, -1.04f, -4f), V3(0f, -1.04f, 4f), cx, cy, focal)
        paint.color = Color.rgb(83, 145, 235)
        drawWorldLine(canvas, V3(0f, -1f, 0f), V3(0f, 3f, 0f), cx, cy, focal)
    }

    private fun drawWorldLine(canvas: Canvas, a: V3, b: V3, cx: Float, cy: Float, focal: Float) {
        val pa = project(a, cx, cy, focal) ?: return
        val pb = project(b, cx, cy, focal) ?: return
        canvas.drawLine(pa.x, pa.y, pb.x, pb.y, paint)
    }

    private fun drawMesh(
        canvas: Canvas,
        mesh: ViewportMeshData,
        meshIndex: Int,
        cx: Float,
        cy: Float,
        focal: Float
    ) {
        val projected = mesh.vertices.map { project(V3(it[0], it[1], it[2]), cx, cy, focal) }
        val baseColors = intArrayOf(
            Color.rgb(65, 111, 173), Color.rgb(87, 143, 210),
            Color.rgb(48, 83, 128), Color.rgb(100, 158, 221),
            Color.rgb(55, 95, 148), Color.rgb(76, 128, 191)
        )
        val isActive = mesh.id == scene.activeObjectId
        val faces = mesh.polygons.mapIndexedNotNull { faceIndex, polygon ->
            if (polygon.size < 3 || polygon.any { it !in projected.indices }) return@mapIndexedNotNull null
            val points = polygon.map { vertexIndex -> projected[vertexIndex] ?: return@mapIndexedNotNull null }
            val depth = points.map { it.depth }.average().toFloat()
            Triple(depth, baseColors[(meshIndex + faceIndex) % baseColors.size], points)
        }.sortedByDescending { it.first }

        for ((_, faceColor, points) in faces) {
            val path = Path().apply {
                moveTo(points[0].x, points[0].y)
                for (i in 1 until points.size) lineTo(points[i].x, points[i].y)
                close()
            }
            paint.style = Paint.Style.FILL
            paint.color = if (isActive) brighten(faceColor) else faceColor
            canvas.drawPath(path, paint)
            if (showEdges) {
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = dp(if (isActive) 2f else 1.1f)
                paint.color = if (isActive) Color.rgb(255, 221, 124) else Color.rgb(190, 216, 245)
                canvas.drawPath(path, paint)
            }
        }
    }

    private fun brighten(color: Int): Int {
        val r = (Color.red(color) + 34).coerceAtMost(255)
        val g = (Color.green(color) + 24).coerceAtMost(255)
        val b = (Color.blue(color) + 8).coerceAtMost(255)
        return Color.rgb(r, g, b)
    }

    private fun drawGizmo(canvas: Canvas) {
        val originX = width - dp(43f)
        val originY = dp(83f)
        val length = dp(23f)
        val axes = listOf(
            Triple(V3(1f, 0f, 0f), Color.rgb(235, 91, 91), "X"),
            Triple(V3(0f, 1f, 0f), Color.rgb(98, 210, 137), "Y"),
            Triple(V3(0f, 0f, 1f), Color.rgb(100, 160, 250), "Z")
        )
        for ((axis, color, label) in axes) {
            val r = rotate(axis)
            val x = originX + r.x * length
            val y = originY - r.y * length
            paint.style = Paint.Style.STROKE
            paint.strokeWidth = dp(2f)
            paint.color = color
            canvas.drawLine(originX, originY, x, y, paint)
            paint.style = Paint.Style.FILL
            paint.textSize = dp(10f)
            canvas.drawText(label, x - dp(3f), y - dp(4f), paint)
        }
    }

    private fun selectAt(x: Float, y: Float) {
        val cx = width * 0.5f
        val cy = height * 0.53f
        val focal = min(width, height) * 0.92f * zoom
        val hits = mutableListOf<ProjectedFace>()
        for (mesh in scene.meshes) {
            val projected = mesh.vertices.map { project(V3(it[0], it[1], it[2]), cx, cy, focal) }
            for (polygon in mesh.polygons) {
                if (polygon.size < 3 || polygon.any { it !in projected.indices }) continue
                val points = polygon.map { projected[it] }.filterNotNull()
                if (points.size != polygon.size) continue
                if (pointInPolygon(x, y, points)) {
                    hits.add(ProjectedFace(mesh.id, points.map { it.depth }.average().toFloat(), points))
                }
            }
        }
        val hit = hits.minByOrNull { it.depth } ?: return
        if (hit.meshId != scene.activeObjectId && NativeGeometry.selectObject(hit.meshId)) refreshScene()
    }

    private fun pointInPolygon(x: Float, y: Float, points: List<P2>): Boolean {
        var inside = false
        var j = points.lastIndex
        for (i in points.indices) {
            val a = points[i]
            val b = points[j]
            if ((a.y > y) != (b.y > y) &&
                x < (b.x - a.x) * (y - a.y) / ((b.y - a.y).takeIf { abs(it) > 0.0001f } ?: 0.0001f) + a.x
            ) inside = !inside
            j = i
        }
        return inside
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = event.x
                downY = event.y
                lastX = event.x
                lastY = event.y
                moved = false
                multiTouch = false
                return true
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                multiTouch = true
                if (event.pointerCount >= 2) lastPinch = pointerDistance(event)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (event.pointerCount >= 2) {
                    multiTouch = true
                    val distance = pointerDistance(event)
                    if (lastPinch > 0f && distance > 0f) zoom = (zoom * distance / lastPinch).coerceIn(0.35f, 3.2f)
                    lastPinch = distance
                } else {
                    if (hypot(event.x - downX, event.y - downY) > touchSlop) moved = true
                    if (moved) {
                        yaw += (event.x - lastX) * 0.008f
                        pitch = (pitch + (event.y - lastY) * 0.008f).coerceIn(-1.35f, 1.35f)
                    }
                }
                lastX = event.x
                lastY = event.y
                invalidate()
                return true
            }
            MotionEvent.ACTION_POINTER_UP -> {
                lastPinch = 0f
                return true
            }
            MotionEvent.ACTION_UP -> {
                if (!moved && !multiTouch) selectAt(event.x, event.y)
                lastPinch = 0f
                performClick()
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                lastPinch = 0f
                return true
            }
        }
        return true
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    private fun pointerDistance(event: MotionEvent): Float {
        if (event.pointerCount < 2) return 0f
        return hypot(event.getX(0) - event.getX(1), event.getY(0) - event.getY(1))
    }

    private fun fallbackScene(): ViewportSceneData = ViewportSceneData(
        meshes = listOf(
            ViewportMeshData(
                id = 1L,
                name = "Cube",
                vertices = listOf(
                    floatArrayOf(-1f, -1f, -1f), floatArrayOf(1f, -1f, -1f),
                    floatArrayOf(1f, 1f, -1f), floatArrayOf(-1f, 1f, -1f),
                    floatArrayOf(-1f, -1f, 1f), floatArrayOf(1f, -1f, 1f),
                    floatArrayOf(1f, 1f, 1f), floatArrayOf(-1f, 1f, 1f)
                ),
                polygons = listOf(
                    intArrayOf(0, 1, 2, 3), intArrayOf(4, 7, 6, 5),
                    intArrayOf(0, 4, 5, 1), intArrayOf(3, 2, 6, 7),
                    intArrayOf(0, 3, 7, 4), intArrayOf(1, 5, 6, 2)
                )
            )
        ),
        activeObjectId = 1L,
        fromRust = false
    )

    private fun dp(value: Float): Float = value * resources.displayMetrics.density
}
