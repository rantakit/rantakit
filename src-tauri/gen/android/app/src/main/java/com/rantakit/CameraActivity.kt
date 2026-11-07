package com.rantakit

import android.app.Activity
import android.content.Intent
import android.content.res.ColorStateList
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.media.MediaActionSound
import android.os.Bundle
import android.util.Size
import android.util.TypedValue
import android.view.Gravity
import android.view.OrientationEventListener
import android.view.Surface
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.AspectRatioStrategy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import org.json.JSONObject
import java.io.File

/**
 * Scan Dokumen's camera (D3): a large shutter, a flash switch, the last
 * page taken with the count, "Dari Galeri", and "Selesai". Photos go into
 * the folder Rust gives, never to Gallery; Rust removes them when the Task
 * ends or is left. Built on CameraX alone, so nothing comes from Google
 * Play Services. The words come from the frontend, in the app's language.
 */
class CameraActivity : AppCompatActivity() {
  private lateinit var words: JSONObject
  private lateinit var folder: File
  private val taken = ArrayList<String>()
  private var capture: ImageCapture? = null
  private var flashOn = false
  private var busy = false
  private val sound = MediaActionSound()

  private lateinit var flash: Button
  private lateinit var shutter: FrameLayout
  private lateinit var thumbnail: FrameLayout
  private lateinit var thumbnailImage: ImageView
  private lateinit var count: TextView
  private lateinit var done: Button
  private lateinit var blink: View

  /** Photos follow how the phone is held, though the screen stays upright. */
  private val orientation by lazy {
    object : OrientationEventListener(this) {
      override fun onOrientationChanged(degrees: Int) {
        if (degrees == ORIENTATION_UNKNOWN) return
        capture?.targetRotation = when (degrees) {
          in 45 until 135 -> Surface.ROTATION_270
          in 135 until 225 -> Surface.ROTATION_180
          in 225 until 315 -> Surface.ROTATION_90
          else -> Surface.ROTATION_0
        }
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    words = JSONObject(intent.getStringExtra(WORDS) ?: "{}")
    folder = File(intent.getStringExtra(FOLDER) ?: cacheDir.path).apply { mkdirs() }
    savedInstanceState?.getStringArrayList(TAKEN)?.let { taken += it }

    val preview = PreviewView(this).apply {
      scaleType = PreviewView.ScaleType.FIT_CENTER
      implementationMode = PreviewView.ImplementationMode.COMPATIBLE
      contentDescription = word("preview")
    }
    setContentView(layout(preview))
    sound.load(MediaActionSound.SHUTTER_CLICK)
    showTaken()
    start(preview)

    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() = finishWith(gallery = false)
    })
  }

  override fun onSaveInstanceState(outState: Bundle) {
    super.onSaveInstanceState(outState)
    outState.putStringArrayList(TAKEN, taken)
  }

  override fun onResume() {
    super.onResume()
    orientation.enable()
  }

  override fun onPause() {
    orientation.disable()
    super.onPause()
  }

  override fun onDestroy() {
    sound.release()
    super.onDestroy()
  }

  private fun start(preview: PreviewView) {
    val future = ProcessCameraProvider.getInstance(this)
    future.addListener({
      try {
        val provider = future.get()
        val shown = Preview.Builder().build().also { it.surfaceProvider = preview.surfaceProvider }
        // About 8 MP: sharp enough for text, and quick to save on a low-end phone.
        val size = ResolutionSelector.Builder()
          .setAspectRatioStrategy(AspectRatioStrategy.RATIO_4_3_FALLBACK_AUTO_STRATEGY)
          .setResolutionStrategy(
            ResolutionStrategy(Size(3264, 2448), ResolutionStrategy.FALLBACK_RULE_CLOSEST_LOWER_THEN_HIGHER)
          )
          .build()
        val photos = ImageCapture.Builder()
          .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
          .setResolutionSelector(size)
          .setFlashMode(if (flashOn) ImageCapture.FLASH_MODE_ON else ImageCapture.FLASH_MODE_OFF)
          .build()
        provider.unbindAll()
        provider.bindToLifecycle(this, CameraSelector.DEFAULT_BACK_CAMERA, shown, photos)
        capture = photos
      } catch (e: Exception) {
        Toast.makeText(this, word("noCamera"), Toast.LENGTH_LONG).show()
        finishWith(gallery = false)
      }
    }, ContextCompat.getMainExecutor(this))
  }

  private fun shoot() {
    val photos = capture ?: return
    if (busy) return
    busy = true
    shutter.isEnabled = false
    val file = File(folder, "scan-${System.currentTimeMillis()}.jpg")
    sound.play(MediaActionSound.SHUTTER_CLICK)
    blink.animate().cancel()
    blink.alpha = 0.7f
    blink.animate().alpha(0f).setDuration(250).start()
    photos.takePicture(
      ImageCapture.OutputFileOptions.Builder(file).build(),
      ContextCompat.getMainExecutor(this),
      object : ImageCapture.OnImageSavedCallback {
        override fun onImageSaved(output: ImageCapture.OutputFileResults) {
          taken += file.path
          busy = false
          shutter.isEnabled = true
          showTaken()
          @Suppress("DEPRECATION")
          shutter.announceForAccessibility(word("taken").replace("{count}", taken.size.toString()))
        }

        override fun onError(error: ImageCaptureException) {
          file.delete()
          busy = false
          shutter.isEnabled = true
          Toast.makeText(this@CameraActivity, word("failed"), Toast.LENGTH_SHORT).show()
        }
      },
    )
  }

  private fun toggleFlash() {
    flashOn = !flashOn
    capture?.flashMode = if (flashOn) ImageCapture.FLASH_MODE_ON else ImageCapture.FLASH_MODE_OFF
    flash.text = word(if (flashOn) "flashOn" else "flashOff")
    flash.isActivated = flashOn
    ViewCompat.setStateDescription(flash, flash.text)
    flash.backgroundTintList = ColorStateList.valueOf(if (flashOn) TEAL else GLASS)
    flash.setTextColor(if (flashOn) ON_TEAL else WHITE)
    flash.compoundDrawablesRelative[0]?.setTint(if (flashOn) ON_TEAL else WHITE)
  }

  /** The last page taken, small, and how many there are. */
  private fun showTaken() {
    val any = taken.isNotEmpty()
    thumbnail.visibility = if (any) View.VISIBLE else View.INVISIBLE
    done.visibility = if (any) View.VISIBLE else View.INVISIBLE
    if (!any) return
    count.text = taken.size.toString()
    thumbnail.contentDescription = word("pages").replace("{count}", taken.size.toString())
    val options = BitmapFactory.Options().apply { inSampleSize = 16 }
    thumbnailImage.setImageBitmap(BitmapFactory.decodeFile(taken.last(), options))
  }

  /** Back to the page list with what was taken; `gallery` opens the picker there. */
  private fun finishWith(gallery: Boolean) {
    setResult(Activity.RESULT_OK, Intent().putStringArrayListExtra(TAKEN, taken).putExtra(GALLERY, gallery))
    finish()
  }

  private fun word(key: String) = words.optString(key, "")

  private fun dp(value: Int) =
    TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value.toFloat(), resources.displayMetrics).toInt()

  private fun pill(color: Int) = GradientDrawable().apply {
    cornerRadius = dp(999).toFloat()
    setColor(color)
  }

  private fun textButton(text: String, icon: Int?, background: Int, textColor: Int) = Button(this).apply {
    this.text = text
    isAllCaps = false
    setTextColor(textColor)
    setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
    typeface = Typeface.DEFAULT_BOLD
    minHeight = dp(48)
    minimumHeight = dp(48)
    stateListAnimator = null
    this.background = pill(Color.WHITE)
    backgroundTintList = ColorStateList.valueOf(background)
    setPadding(dp(14), 0, dp(16), 0)
    if (icon != null) {
      val drawable = ContextCompat.getDrawable(context, icon)?.mutate()?.apply {
        setBounds(0, 0, dp(20), dp(20))
        setTint(textColor)
      }
      setCompoundDrawablesRelative(drawable, null, null, null)
      compoundDrawablePadding = dp(8)
    }
  }

  private fun layout(preview: PreviewView): View {
    val close = ImageButton(this).apply {
      setImageResource(R.drawable.ic_camera_close)
      imageTintList = ColorStateList.valueOf(WHITE)
      background = pill(GLASS)
      contentDescription = word("close")
      setOnClickListener { finishWith(gallery = false) }
    }
    flash = textButton(word("flashOff"), R.drawable.ic_camera_flash, GLASS, WHITE).apply {
      ViewCompat.setStateDescription(this, text)
      setOnClickListener { toggleFlash() }
    }
    val header = FrameLayout(this).apply {
      setPadding(dp(12), dp(8), dp(12), dp(8))
      addView(close, FrameLayout.LayoutParams(dp(48), dp(48), Gravity.START or Gravity.CENTER_VERTICAL))
      addView(flash, FrameLayout.LayoutParams(WRAP_CONTENT, dp(48), Gravity.END or Gravity.CENTER_VERTICAL))
    }

    val hint = TextView(this).apply {
      text = word("hint")
      setTextColor(WHITE)
      setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
      typeface = Typeface.DEFAULT_BOLD
      background = pill(Color.argb(166, 0, 0, 0))
      setPadding(dp(14), dp(6), dp(14), dp(6))
    }
    blink = View(this).apply {
      setBackgroundColor(WHITE)
      alpha = 0f
      importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    val viewfinder = FrameLayout(this).apply {
      addView(preview, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
      addView(blink, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
      addView(hint, FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply {
        topMargin = dp(16)
      })
    }

    val gallery = textButton(word("gallery"), R.drawable.ic_camera_gallery, GLASS, WHITE).apply {
      setOnClickListener { finishWith(gallery = true) }
    }

    thumbnailImage = ImageView(this).apply {
      scaleType = ImageView.ScaleType.CENTER_CROP
      background = GradientDrawable().apply {
        cornerRadius = dp(10).toFloat()
        setColor(Color.rgb(248, 250, 252))
        setStroke(dp(2), WHITE)
      }
      clipToOutline = true
      setPadding(dp(2), dp(2), dp(2), dp(2))
    }
    count = TextView(this).apply {
      setTextColor(ON_TEAL)
      setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
      typeface = Typeface.DEFAULT_BOLD
      gravity = Gravity.CENTER
      background = pill(TEAL)
      minWidth = dp(26)
      setPadding(dp(6), 0, dp(6), 0)
      importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    thumbnail = FrameLayout(this).apply {
      clipChildren = false
      clipToPadding = false
      isClickable = true
      isFocusable = true
      setOnClickListener { finishWith(gallery = false) }
      addView(thumbnailImage, FrameLayout.LayoutParams(dp(60), dp(72), Gravity.BOTTOM or Gravity.START))
      addView(count, FrameLayout.LayoutParams(WRAP_CONTENT, dp(26), Gravity.TOP or Gravity.END))
      setPadding(0, dp(10), dp(10), 0)
    }

    shutter = FrameLayout(this).apply {
      background = GradientDrawable().apply {
        shape = GradientDrawable.OVAL
        setStroke(dp(4), WHITE)
      }
      contentDescription = word("shutter")
      isClickable = true
      isFocusable = true
      setOnClickListener { shoot() }
      addView(View(context).apply {
        background = GradientDrawable().apply {
          shape = GradientDrawable.OVAL
          setColor(WHITE)
        }
      }, FrameLayout.LayoutParams(dp(62), dp(62), Gravity.CENTER))
    }

    done = textButton(word("done"), null, TEAL, ON_TEAL).apply {
      setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
      minWidth = dp(76)
      background = GradientDrawable().apply {
        cornerRadius = dp(14).toFloat()
        setColor(Color.WHITE)
      }
      setOnClickListener { finishWith(gallery = false) }
    }

    val controls = FrameLayout(this).apply {
      addView(thumbnail, FrameLayout.LayoutParams(dp(70), dp(82), Gravity.START or Gravity.CENTER_VERTICAL))
      addView(shutter, FrameLayout.LayoutParams(dp(80), dp(80), Gravity.CENTER))
      addView(done, FrameLayout.LayoutParams(WRAP_CONTENT, dp(48), Gravity.END or Gravity.CENTER_VERTICAL))
    }
    val bottom = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setPadding(dp(20), dp(14), dp(20), dp(28))
      addView(gallery, LinearLayout.LayoutParams(WRAP_CONTENT, dp(44)).apply { gravity = Gravity.CENTER_HORIZONTAL })
      addView(controls, LinearLayout.LayoutParams(MATCH_PARENT, dp(86)).apply { topMargin = dp(14) })
    }

    return LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setBackgroundColor(Color.BLACK)
      addView(header, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
      addView(viewfinder, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
      addView(bottom, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
      ViewCompat.setOnApplyWindowInsetsListener(this) { view, insets ->
        val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
        view.updatePadding(left = bars.left, top = bars.top, right = bars.right, bottom = bars.bottom)
        insets
      }
    }
  }

  companion object {
    const val WORDS = "words"
    const val FOLDER = "folder"
    const val TAKEN = "taken"
    const val GALLERY = "gallery"
    private val WHITE = Color.rgb(245, 245, 245)
    private val GLASS = Color.argb(31, 255, 255, 255)
    private val TEAL = Color.rgb(45, 212, 191)
    private val ON_TEAL = Color.rgb(4, 47, 46)
  }
}
