package com.rantakit

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.activity.result.ActivityResult
import androidx.core.app.ActivityCompat
import app.tauri.PermissionState
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Scan Dokumen's camera (D3): the camera permission, and [CameraActivity],
 * which hands back the photos it took. Rust calls it; `camera.rs` says how.
 */
@TauriPlugin(
  permissions = [
    Permission(strings = [Manifest.permission.CAMERA], alias = "camera")
  ]
)
class CameraPlugin(private val activity: Activity) : Plugin(activity) {
  private val prefs = activity.getSharedPreferences("camera", Context.MODE_PRIVATE)

  /**
   * "granted", "ask", or "refused". Android lets an app ask again after one
   * refusal; after the second, or "Jangan tanya lagi", only Settings can
   * allow it.
   */
  @Command
  fun cameraPermission(invoke: Invoke) {
    val state = when {
      getPermissionState("camera") == PermissionState.GRANTED -> "granted"
      !prefs.getBoolean(ASKED, false) -> "ask"
      ActivityCompat.shouldShowRequestPermissionRationale(activity, Manifest.permission.CAMERA) -> "ask"
      else -> "refused"
    }
    invoke.resolve(JSObject().apply { put("state", state) })
  }

  /** Shows Android's prompt, after the app has said why; resolves with `granted`. */
  @Command
  fun askCameraPermission(invoke: Invoke) {
    prefs.edit().putBoolean(ASKED, true).apply()
    requestPermissionForAlias("camera", invoke, "cameraPermissionAnswered")
  }

  @PermissionCallback
  private fun cameraPermissionAnswered(invoke: Invoke) {
    val granted = getPermissionState("camera") == PermissionState.GRANTED
    invoke.resolve(JSObject().apply { put("granted", granted) })
  }

  /** Rantakit's page in the phone's Settings, where a refused permission can be allowed. */
  @Command
  fun openSettings(invoke: Invoke) {
    activity.startActivity(
      Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", activity.packageName, null))
    )
    invoke.resolve()
  }

  /**
   * Opens the camera. Takes `folder`, where the photos go, and `words`, the
   * camera's text; resolves with `paths`, the photos taken in order, and
   * `gallery`, whether "Dari Galeri" closed it.
   */
  @Command
  fun takePhotos(invoke: Invoke) {
    val args = invoke.getArgs()
    val intent = Intent(activity, CameraActivity::class.java)
      .putExtra(CameraActivity.FOLDER, args.getString("folder"))
      .putExtra(CameraActivity.WORDS, args.getJSObject("words")?.toString() ?: "{}")
    startActivityForResult(invoke, intent, "photosTaken")
  }

  @ActivityCallback
  private fun photosTaken(invoke: Invoke, result: ActivityResult) {
    val data = result.data
    val paths = data?.getStringArrayListExtra(CameraActivity.TAKEN).orEmpty()
    invoke.resolve(JSObject().apply {
      put("paths", JSArray(paths))
      put("gallery", data?.getBooleanExtra(CameraActivity.GALLERY, false) ?: false)
    })
  }

  private companion object {
    const val ASKED = "cameraPermissionAsked"
  }
}
