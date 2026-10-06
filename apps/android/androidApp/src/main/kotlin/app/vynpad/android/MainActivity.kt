package app.vynpad.android

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import app.vynpad.pairing.DeepLinks
import app.vynpad.ui.VynApp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        handle(intent)
        setContent { VynApp() }
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
