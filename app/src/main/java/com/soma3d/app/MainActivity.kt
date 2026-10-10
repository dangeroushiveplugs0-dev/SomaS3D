package com.soma3d.app

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.FrameLayout

class MainActivity : Activity() {
    private lateinit var viewport: ViewportView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = Color.rgb(18, 20, 25)
        window.navigationBarColor = Color.rgb(18, 20, 25)

        val root = FrameLayout(this).apply { setBackgroundColor(Color.rgb(18, 20, 25)) }
        viewport = ViewportView(this)
        root.addView(viewport, FrameLayout.LayoutParams(-1, -1))

        val top = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(12), dp(6), dp(12), dp(6))
            setBackgroundColor(Color.argb(225, 25, 28, 35))
        }
        val title = TextView(this).apply {
            text = "SOMA  /  VIEWPORT"
            setTextColor(Color.WHITE)
            textSize = 13f
            letterSpacing = 0.08f
            setPadding(0, 0, dp(8), 0)
        }
        top.addView(title, LinearLayout.LayoutParams(0, -2, 1f))
        top.addView(toolButton("CUBE+") { viewport.addCube() })
        top.addView(toolButton("SPH+") { viewport.addSphere() })
        top.addView(toolButton("FIT") { viewport.resetView() })
        top.addView(toolButton("GRID") { viewport.toggleGrid() })
        top.addView(toolButton("EDGE") { viewport.toggleEdges() })
        root.addView(top, FrameLayout.LayoutParams(-1, dp(54), Gravity.TOP))

        val bottom = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(8), dp(2), dp(8), dp(2))
            setBackgroundColor(Color.argb(230, 25, 28, 35))
        }
        val status = TextView(this).apply {
            text = "TAP TO SELECT / DESELECT  •  DRAG ORBIT"
            setTextColor(Color.rgb(177, 188, 204))
            textSize = 9f
            letterSpacing = 0.02f
            setPadding(dp(4), dp(8), dp(4), dp(8))
        }
        bottom.addView(status, LinearLayout.LayoutParams(0, -2, 1f))

        lateinit var selectionModeButton: Button
        selectionModeButton = toolButton("OBJECT") {
            val faceMode = viewport.toggleSelectionMode()
            selectionModeButton.text = if (faceMode) "FACE" else "OBJECT"
            status.text = if (faceMode) "TAP FACES TO ADD / REMOVE  •  DRAG ORBIT" else "TAP OBJECTS TO ADD / REMOVE  •  DRAG ORBIT"
        }
        bottom.addView(selectionModeButton)

        root.addView(bottom, FrameLayout.LayoutParams(-1, -2, Gravity.BOTTOM))
        setContentView(root)
    }

    private fun toolButton(label: String, action: () -> Unit): Button = Button(this).apply {
        text = label
        textSize = 10f
        isAllCaps = false
        setTextColor(Color.WHITE)
        backgroundTintList = android.content.res.ColorStateList.valueOf(Color.rgb(54, 62, 76))
        setPadding(dp(5), 0, dp(5), 0)
        minWidth = dp(44)
        minimumWidth = dp(44)
        setOnClickListener { action() }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density + 0.5f).toInt()
}
