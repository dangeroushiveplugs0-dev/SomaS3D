package com.soma3d.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast

class MainActivity : Activity() {
    private lateinit var viewport: ViewportView
    private lateinit var statusText: TextView
    private var selectedBlendUri: Uri? = null

    private val pickBlendFile = 4101

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = android.graphics.Color.rgb(18, 21, 27)
        window.navigationBarColor = android.graphics.Color.rgb(18, 21, 27)
        window.decorView.systemUiVisibility = 0

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(android.graphics.Color.rgb(18, 21, 27))
            fitsSystemWindows = true
        }

        val header = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(16), dp(8), dp(12), dp(8))
            setBackgroundColor(android.graphics.Color.rgb(25, 29, 37))
        }
        val titleStack = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        titleStack.addView(TextView(this).apply {
            text = "SomaS3D"
            setTextColor(android.graphics.Color.rgb(241, 244, 249))
            textSize = 19f
            setTypeface(null, android.graphics.Typeface.BOLD)
        })
        titleStack.addView(TextView(this).apply {
            text = "VIEWPORT FOUNDATION"
            setTextColor(android.graphics.Color.rgb(135, 149, 169))
            textSize = 10f
        })
        header.addView(titleStack, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        header.addView(makeButton("Reset camera") { viewport.resetCamera() })
        root.addView(header, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, dp(62)
        ))

        viewport = ViewportView(this)
        root.addView(viewport, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f
        ))

        val bottom = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(10), dp(12), dp(12))
            setBackgroundColor(android.graphics.Color.rgb(25, 29, 37))
        }
        statusText = TextView(this).apply {
            text = "Scene empty · choose a .blend file to validate its header"
            setTextColor(android.graphics.Color.rgb(168, 181, 198))
            textSize = 12f
            maxLines = 3
            setPadding(dp(2), 0, dp(2), dp(8))
        }
        bottom.addView(statusText)

        val buttons = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        buttons.addView(makeButton("Import .blend") { openBlendPicker() },
            LinearLayout.LayoutParams(0, dp(46), 1f).apply { marginEnd = dp(8) })
        buttons.addView(makeButton("ShofterUI") {
            Toast.makeText(
                this@MainActivity,
                "ShofterUI is planned; normalized character data integration comes next.",
                Toast.LENGTH_SHORT
            ).show()
        }, LinearLayout.LayoutParams(0, dp(46), 1f))
        bottom.addView(buttons)
        root.addView(bottom, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ))

        setContentView(root)
    }

    private fun openBlendPicker() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            putExtra(Intent.EXTRA_MIME_TYPES, arrayOf(
                "application/octet-stream",
                "application/x-blender"
            ))
        }
        try {
            startActivityForResult(intent, pickBlendFile)
        } catch (_: Exception) {
            Toast.makeText(this, "Unable to open file picker", Toast.LENGTH_SHORT).show()
        }
    }

    @Deprecated("Legacy callback kept for compatibility with the minimum Android API")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != pickBlendFile || resultCode != RESULT_OK) return
        val uri = data?.data ?: return
        val displayName = queryDisplayName(uri) ?: (uri.lastPathSegment ?: "Selected file")
        val inspection = try {
            val stream = contentResolver.openInputStream(uri)
            if (stream == null) {
                null
            } else {
                stream.use { BlendFileInspector.inspect(it, queryFileSize(uri)) }
            }
        } catch (error: Exception) {
            selectedBlendUri = null
            statusText.text = "Could not read $displayName · ${error.message ?: "file access failed"}"
            Toast.makeText(this, "Unable to read selected file", Toast.LENGTH_LONG).show()
            return
        }

        if (inspection == null) {
            selectedBlendUri = null
            statusText.text = "Could not open $displayName · the document provider returned no data"
            return
        }

        selectedBlendUri = if (inspection.valid) uri else null
        statusText.text = if (inspection.valid) {
            "Header validated: $displayName\n${inspection.message}"
        } else {
            "Not imported: $displayName\n${inspection.message}"
        }
        Toast.makeText(
            this,
            if (inspection.valid) "Blender header validated; mesh parsing is not implemented yet."
            else "This file did not pass Blender header validation.",
            Toast.LENGTH_LONG
        ).show()
    }

    private fun queryDisplayName(uri: Uri): String? {
        val cursor = contentResolver.query(uri, null, null, null, null) ?: return null
        cursor.use {
            val nameIndex = it.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
            if (nameIndex >= 0 && it.moveToFirst()) return it.getString(nameIndex)
        }
        return null
    }

    private fun queryFileSize(uri: Uri): Long? {
        val cursor = contentResolver.query(uri, null, null, null, null) ?: return null
        cursor.use {
            val sizeIndex = it.getColumnIndex(android.provider.OpenableColumns.SIZE)
            if (sizeIndex >= 0 && it.moveToFirst() && !it.isNull(sizeIndex)) {
                return it.getLong(sizeIndex).takeIf { size -> size >= 0L }
            }
        }
        return null
    }

    private fun makeButton(label: String, action: () -> Unit): Button {
        return Button(this).apply {
            text = label
            textSize = 12f
            isAllCaps = false
            setTextColor(android.graphics.Color.rgb(235, 240, 247))
            backgroundTintList = android.content.res.ColorStateList.valueOf(
                android.graphics.Color.rgb(49, 59, 75)
            )
            setOnClickListener { action() }
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}
