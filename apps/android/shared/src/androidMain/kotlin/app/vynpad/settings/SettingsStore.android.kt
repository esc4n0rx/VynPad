package app.vynpad.settings

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext

private const val PREFS = "vynpad_settings"
private const val KEY = "app_settings"

private class AndroidSettingsStore(context: Context) : SettingsStore {
    private val prefs = context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    override fun load(): String? = prefs.getString(KEY, null)

    override fun save(json: String) {
        prefs.edit().putString(KEY, json).apply()
    }
}

@Composable
actual fun rememberSettingsStore(): SettingsStore {
    val context = LocalContext.current
    return remember(context) { AndroidSettingsStore(context) }
}
