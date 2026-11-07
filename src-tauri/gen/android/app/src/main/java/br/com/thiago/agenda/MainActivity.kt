package br.com.thiago.agenda

import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val content = findViewById<android.view.View>(android.R.id.content)
    val initialPadding = intArrayOf(
      content.paddingLeft,
      content.paddingTop,
      content.paddingRight,
      content.paddingBottom,
    )
    val insetTypes =
      WindowInsetsCompat.Type.statusBars() or WindowInsetsCompat.Type.displayCutout()

    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val systemInsets = windowInsets.getInsets(insetTypes)
      view.setPadding(
        initialPadding[0] + systemInsets.left,
        initialPadding[1] + systemInsets.top,
        initialPadding[2] + systemInsets.right,
        initialPadding[3] + systemInsets.bottom,
      )
      WindowInsetsCompat.Builder(windowInsets)
        .setInsets(insetTypes, Insets.NONE)
        .build()
    }
    ViewCompat.requestApplyInsets(content)
  }
}
