package com.soma3d.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Color
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.View
import kotlin.math.abs

/**
 * Lightweight custom-drawn viewport. Geometry will be supplied by the native
 * scene/import pipeline later; this first version establishes camera navigation.
 */
class ViewportView(context: Context) : View(context) {
    val camera = OrbitCamera()

    private val backgroundPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(18, 21, 27)
        style = Paint.Style.FILL
    }
    private val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(45, 51, 62)
        strokeWidth = dp(1f)
        style = Paint.Style.STROKE
    }
    private val majorGridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(60, 68, 81)
        strokeWidth = dp(1.2f)
        style = Paint.Style.STROKE
    }
    private val xAxisPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(232, 89, 92)
        strokeWidth = dp(2f)
    }
    private val zAxisPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(76, 158, 244)
        strokeWidth = dp(2f)
    }
    private val yAxisPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(93, 203, 133)
        strokeWidth = dp(2f)
    }
    private val originPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(242, 198, 88)
        style = Paint.Style.FILL
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(185, 194, 207)
        textSize = dp(11f)
    }
    private val cameraBadgePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(31, 37, 47)
        style = Paint.Style.FILL
    }
    private val cameraBadgeTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(218, 225, 235)
        textSize = dp(12f)
    }

    private val scaleDetector = ScaleGestureDetector(context,
        object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
            override fun onScale(detector: ScaleGestureDetector): Boolean {
                camera.zoom(detector.scaleFactor)
                invalidate()
                return true
            }
        }
    )

    private var previousX = 0f
    private var previousY = 0f
    private var moved = false

    init {
        isFocusable = true
        contentDescription = "3D viewport. Drag to orbit camera and pinch to zoom."
    }

    fun resetCamera() {
        camera.reset()
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), backgroundPaint)
        drawGrid(canvas)
        drawAxes(canvas)
        drawOrigin(canvas)
        drawAxisGizmo(canvas)
        drawCameraBadge(canvas)
    }

    private fun drawGrid(canvas: Canvas) {
        val extent = 12
        for (i in -extent..extent) {
            val paint = if (i % 5 == 0) majorGridPaint else gridPaint
            drawWorldLine(canvas, OrbitCamera.Vec3(i.toDouble(), 0.0, -extent.toDouble()),
                OrbitCamera.Vec3(i.toDouble(), 0.0, extent.toDouble()), paint)
            drawWorldLine(canvas, OrbitCamera.Vec3(-extent.toDouble(), 0.0, i.toDouble()),
                OrbitCamera.Vec3(extent.toDouble(), 0.0, i.toDouble()), paint)
        }
    }

    private fun drawAxes(canvas: Canvas) {
        drawWorldLine(canvas, OrbitCamera.Vec3(-12.0, 0.0, 0.0),
            OrbitCamera.Vec3(12.0, 0.0, 0.0), xAxisPaint)
        drawWorldLine(canvas, OrbitCamera.Vec3(0.0, 0.0, -12.0),
            OrbitCamera.Vec3(0.0, 0.0, 12.0), zAxisPaint)
        drawWorldLine(canvas, OrbitCamera.Vec3(0.0, 0.0, 0.0),
            OrbitCamera.Vec3(0.0, 3.2, 0.0), yAxisPaint)
    }

    private fun drawOrigin(canvas: Canvas) {
        val origin = camera.project(OrbitCamera.Vec3(0.0, 0.0, 0.0), width.toFloat(), height.toFloat())
            ?: return
        canvas.drawCircle(origin.x, origin.y, dp(4f), originPaint)
        canvas.drawCircle(origin.x, origin.y, dp(7f), Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.argb(100, 242, 198, 88)
            style = Paint.Style.STROKE
            strokeWidth = dp(1f)
        })
    }

    private fun drawAxisGizmo(canvas: Canvas) {
        val originX = width - dp(47f)
        val originY = dp(54f)
        val axisLength = dp(23f)
        canvas.drawLine(originX, originY, originX + axisLength, originY, xAxisPaint)
        canvas.drawLine(originX, originY, originX, originY - axisLength, yAxisPaint)
        canvas.drawLine(originX, originY, originX - dp(13f), originY + dp(13f), zAxisPaint)
        canvas.drawText("X", originX + axisLength + dp(3f), originY + dp(4f), textPaint)
        canvas.drawText("Y", originX - dp(4f), originY - axisLength - dp(4f), textPaint)
        canvas.drawText("Z", originX - dp(22f), originY + dp(24f), textPaint)
    }

    private fun drawCameraBadge(canvas: Canvas) {
        val label = "PERSPECTIVE"
        val left = dp(12f)
        val top = height - dp(34f)
        canvas.drawRoundRect(left, top, left + dp(106f), top + dp(22f), dp(6f), dp(6f), cameraBadgePaint)
        canvas.drawText(label, left + dp(9f), top + dp(15f), cameraBadgeTextPaint)
        canvas.drawText("Drag: orbit  •  Pinch: zoom", left, top - dp(9f), textPaint)
    }

    private fun drawWorldLine(
        canvas: Canvas,
        start: OrbitCamera.Vec3,
        end: OrbitCamera.Vec3,
        paint: Paint
    ) {
        val a = camera.project(start, width.toFloat(), height.toFloat()) ?: return
        val b = camera.project(end, width.toFloat(), height.toFloat()) ?: return
        // Skip huge off-screen segments to avoid spending time rasterizing far-away lines.
        if (abs(a.x - b.x) > width * 4f || abs(a.y - b.y) > height * 4f) return
        canvas.drawLine(a.x, a.y, b.x, b.y, paint)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        scaleDetector.onTouchEvent(event)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                previousX = event.x
                previousY = event.y
                moved = false
                return true
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                previousX = event.x
                previousY = event.y
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (!scaleDetector.isInProgress && event.pointerCount == 1) {
                    val dx = event.x - previousX
                    val dy = event.y - previousY
                    if (abs(dx) + abs(dy) > dp(1f)) moved = true
                    camera.orbit(dx, dy)
                    invalidate()
                }
                previousX = event.x
                previousY = event.y
                return true
            }
            MotionEvent.ACTION_POINTER_UP -> {
                val remaining = if (event.actionIndex == 0) 1 else 0
                if (remaining < event.pointerCount - 1 && event.pointerCount > 1) {
                    previousX = event.getX(if (event.actionIndex == 0) 1 else 0)
                    previousY = event.getY(if (event.actionIndex == 0) 1 else 0)
                }
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> return true
        }
        return true
    }

    private fun dp(value: Float): Float = value * resources.displayMetrics.density
}
