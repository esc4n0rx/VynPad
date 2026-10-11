package app.vynpad.android

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

/**
 * Serviço em primeiro plano que mantém o processo vivo enquanto há sessão com um PC (conectada ou
 * reconectando), para o sistema não matar o app com a tela apagada ou em outro app. A sessão em si
 * vive no `AndroidSessionManager`; aqui só há a notificação ("Conectado como P1").
 *
 * É iniciado e parado pelo gerenciador da sessão, que passa `phase` (`connecting`, `connected` ou
 * `reconnecting`), `server` e `slot` nos extras do [Intent].
 */
class SessionService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        createChannel()
        val server = intent?.getStringExtra("server").orEmpty()
        val text = when (intent?.getStringExtra("phase")) {
            "connected" -> getString(R.string.session_connected, (intent.getIntExtra("slot", 0)) + 1)
            "reconnecting" -> getString(R.string.session_reconnecting)
            else -> getString(R.string.session_connecting)
        }
        val notification = build(server.ifEmpty { getString(R.string.app_name) }, text)
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        // Se o sistema matar o processo, a sessão se perdeu: não faz sentido ressuscitar o serviço.
        return START_NOT_STICKY
    }

    private fun build(title: String, text: String): Notification {
        val open = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_session)
            .setContentTitle(title)
            .setContentText(text)
            .setContentIntent(open)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setShowWhen(false)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .build()
    }

    private fun createChannel() {
        if (Build.VERSION.SDK_INT < 26) return
        val manager = getSystemService(NotificationManager::class.java)
        if (manager.getNotificationChannel(CHANNEL_ID) != null) return
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_ID, getString(R.string.session_channel), NotificationManager.IMPORTANCE_LOW).apply {
                setShowBadge(false)
            },
        )
    }

    private companion object {
        const val CHANNEL_ID = "session"
        const val NOTIFICATION_ID = 1
    }
}
