package com.soma3d.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Color
import android.view.MotionEvent
import android.view.View
import kotlin.math.*

/** Canvas viewport using the Rust scene snapshot when the native bridge is available. */
class ViewportView(context: Context) : View(context) {
    private data class V3(val x: Float, val y: Float, val z: Float)
    private data class P2(val x: Float, val y: Float, val depth: Float)
    private data class Face(val indices: IntArray, val color: Int, val depth: Double, val points: List<P2>)

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val mesh = NativeGeometry.loadDefaultMesh() ?: fallbackMesh()
    private var yaw = -0.65f
    private var pitch = 0.42f
    private var zoom = 1f
    private var showGrid = true
    private var showEdges = true
    private var lastX = 0f
    private var lastY = 0f
    private var lastPinch = 0f

    fun resetView() {
        yaw = -0.65f
        pitch = 0.42f
        zoom = 1f
        invalidate()
    }

    fun toggleGrid() { showGrid = !showGrid; invalidate() }
    fun toggleEdges() { showEdges = !showEdges; invalidate() }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        canvas.drawColor(Color.rgb(22, 25, 31))
        val cx = width * 0.5f
        val cy = height * 0.53f
        val focal = min(width, height) * 0.92f * zoom
        if (showGrid) drawGrid(canvas, cx, cy, focal)
        drawAxes(canvas, cx, cy, focal)
        drawMesh(canvas, cx, cy, focal)
        drawGizmo(canvas)
        paint.style = Paint.Style.FILL
        paint.color = Color.rgb(196, 205, 219)
        paint.textSize = dp(11f)
        val source = if (mesh.fromRust) "Rust Scene" else "Preview Mesh"
        canvas.drawText("${mesh.name}  |  ${source}", dp(16f), height - dp(62f), paint)
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

    private fun drawMesh(canvas: Canvas, cx: Float, cy: Float, focal: Float) {
        val vertices = mesh.vertices.map { V3(it[0], it[1], it[2]) }
        val projected = vertices.map { project(it, cx, cy, focal) }
        val colors = intArrayOf(
            Color.rgb(65, 111, 173), Color.rgb(87, 143, 210),
            Color.rgb(48, 83, 128), Color.rgb(100, 158, 221),
            Color.rgb(55, 95, 148), Color.rgb(76, 128, 191)
        )
        val faces = mesh.polygons.mapIndexedNotNull { index, polygon ->
            if (polygon.size < 3 || polygon.any { it !in projected.indices }) return@mapIndexedNotNull null
            val points = polygon.map { vertexIndex -> projected[vertexIndex] ?: return@mapIndexedNotNull null }
            Face(polygon, colors[index % colors.size], points.map { it.depth }.average(), points)
        }.sortedByDescending { it.depth }

        for (face in faces) {
            val path = Path().apply {
                moveTo(face.points[0].x, face.points[0].y)
                for (i in 1 until face.points.size) lineTo(face.points[i].x, face.points[i].y)
                close()
            }
            paint.style = Paint.Style.FILL
            paint.color = face.color
            canvas.drawPath(path, paint)
            if (showEdges) {
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = dp(1.4f)
                paint.color = Color.rgb(190, 216, 245)
                canvas.drawPath(path, paint)
            }
        }
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

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                lastX = event.x
                lastY = event.y
                return true
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (event.pointerCount >= 2) lastPinch = pointerDistance(event)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (event.pointerCount >= 2) {
                    val distance = pointerDistance(event)
                    if (lastPinch > 0f && distance > 0f) {
                        zoom = (zoom * distance / lastPinch).coerceIn(0.35f, 3.2f)
                    }
                    lastPinch = distance
                } else {
                    yaw += (event.x - lastX) * 0.008f
                    pitch = (pitch + (event.y - lastY) * 0.008f).coerceIn(-1.35f, 1.35f)
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
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                lastPinch = 0f
                performClick()
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

    private fun fallbackMesh() = ViewportMeshData(
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
        ),
        fromRust = false
    )

    private fun dp(value: Float): Float = value * resources.displayMetrics.density
}
