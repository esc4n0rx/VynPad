package app.vynpad.android

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import app.vynpad.pairing.DeepLinks
import app.vynpad.ui.VynApp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        handle(intent)
        askNotificationPermission()
        setContent { VynApp() }
    }

    /** Android 13+: sem a permissão a sessão funciona, mas a notificação "Conectado como P1" não aparece. */
    private fun askNotificationPermission() {
        if (Build.VERSION.SDK_INT < 33) return
        if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) return
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {}.launch(Manifest.permission.POST_NOTIFICATIONS)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handle(intent)
    }

    /** `vynpad://pair?...` vira um pedido de pareamento para a UI. */
    private fun handle(intent: Intent?) {
        val uri = intent?.data?.takeIf { it.scheme == "vynpad" } ?: return
        DeepLinks.pairingUri.value = uri.toString()
    }
}
