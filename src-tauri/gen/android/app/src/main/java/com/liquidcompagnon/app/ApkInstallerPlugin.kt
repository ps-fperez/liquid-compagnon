package com.liquidcompagnon.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class InstallArgs {
  lateinit var path: String
}

/**
 * Mise à jour depuis l'application : ouvre l'installeur Android sur l'APK téléchargé et vérifié
 * par le backend Rust (cache privé, partagé via le FileProvider). Android impose la confirmation
 * de l'utilisateur et refuse un APK signé avec une autre clé.
 */
@TauriPlugin
class ApkInstallerPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun install(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(InstallArgs::class.java)
      val apk = File(args.path)
      if (!apk.isFile) {
        invoke.reject("Fichier de mise à jour introuvable")
        return
      }
      val result = JSObject()
      if (!activity.packageManager.canRequestPackageInstalls()) {
        // Réglage « Installer des applis inconnues » propre à Liquid Compagnon.
        val settings = Intent(
          Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
          Uri.parse("package:" + activity.packageName)
        )
        activity.startActivity(settings)
        result.put("status", "permission_required")
        invoke.resolve(result)
        return
      }
      val uri = FileProvider.getUriForFile(activity, activity.packageName + ".fileprovider", apk)
      val installer = Intent(Intent.ACTION_VIEW).apply {
        setDataAndType(uri, "application/vnd.android.package-archive")
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
      }
      activity.startActivity(installer)
      result.put("status", "started")
      invoke.resolve(result)
    } catch (error: Exception) {
      invoke.reject("Installation impossible : " + (error.message ?: error.javaClass.simpleName))
    }
  }
}
