package app.vynpad.discovery

import android.content.Context
import android.os.Build
import java.util.UUID

/** Identidade estável deste aparelho (o `device_id` do protocolo), criada na primeira abertura. */
internal class DeviceIdentity(context: Context) {
    private val prefs = context.getSharedPreferences("vynpad_device", Context.MODE_PRIVATE)

    val id: String = prefs.getString(KEY_ID, null) ?: UUID.randomUUID().toString().lowercase().also {
        prefs.edit().putString(KEY_ID, it).apply()
    }

    /** Nome exibido no server (1–64 caracteres). */
    val name: String = (Build.MODEL.ifBlank { "Android" }).take(64)

    val model: String = "${Build.MANUFACTURER} ${Build.MODEL}".trim().take(64)

    private companion object {
        const val KEY_ID = "device_id"
    }
}
