package com.soma3d.app

import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * Camera state and projection math kept separate from Android drawing.
 * World convention: Y is up; the ground grid lies on X/Z.
 */
class OrbitCamera {
    var targetX = 0.0
        private set
    var targetY = 0.0
        private set
    var targetZ = 0.0
        private set

    var yaw = Math.toRadians(35.0)
        private set
    var pitch = Math.toRadians(28.0)
        private set
    var distance = 10.0
        private set

    data class Point2(val x: Float, val y: Float)
    data class Vec3(val x: Double, val y: Double, val z: Double)

    fun orbit(deltaX: Float, deltaY: Float) {
        yaw -= deltaX * 0.008
        pitch = (pitch + deltaY * 0.008).coerceIn(
            Math.toRadians(-5.0),
            Math.toRadians(82.0)
        )
    }

    fun zoom(scaleFactor: Float) {
        if (scaleFactor > 0f) {
            distance = (distance / scaleFactor).coerceIn(1.8, 80.0)
        }
    }

    fun reset() {
        targetX = 0.0
        targetY = 0.0
        targetZ = 0.0
        yaw = Math.toRadians(35.0)
        pitch = Math.toRadians(28.0)
        distance = 10.0
    }

    fun project(point: Vec3, width: Float, height: Float): Point2? {
        if (width <= 0f || height <= 0f) return null

        val cp = cos(pitch)
        val camera = Vec3(
            targetX + sin(yaw) * cp * distance,
            targetY + sin(pitch) * distance,
            targetZ + cos(yaw) * cp * distance
        )

        val forward = normalize(Vec3(
            targetX - camera.x,
            targetY - camera.y,
            targetZ - camera.z
        ))
        val right = normalize(cross(forward, Vec3(0.0, 1.0, 0.0)))
        val up = normalize(cross(right, forward))

        val relative = Vec3(point.x - camera.x, point.y - camera.y, point.z - camera.z)
        val depth = dot(relative, forward)
        if (depth <= 0.08) return null

        val focal = min(width, height) * 0.95
        val screenX = width * 0.5 + dot(relative, right) * focal / depth
        val screenY = height * 0.52 - dot(relative, up) * focal / depth
        if (!screenX.isFinite() || !screenY.isFinite()) return null
        return Point2(screenX.toFloat(), screenY.toFloat())
    }

    private fun dot(a: Vec3, b: Vec3) = a.x * b.x + a.y * b.y + a.z * b.z

    private fun cross(a: Vec3, b: Vec3) = Vec3(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x
    )

    private fun normalize(v: Vec3): Vec3 {
        val length = sqrt(v.x * v.x + v.y * v.y + v.z * v.z).coerceAtLeast(1e-9)
        return Vec3(v.x / length, v.y / length, v.z / length)
    }
}
