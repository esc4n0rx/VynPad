package app.vynpad.pairing

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.hardware.display.DisplayManager
import android.os.SystemClock
import android.view.Display
import android.view.Surface
import app.vynpad.input.MotionMath
import app.vynpad.input.Vec3

/** Peso de cada leitura nova no filtro da gravidade: suaviza o tremor da mão sem atrasar o volante. */
private const val SMOOTHING = 0.3f

private const val ROTATION_REFRESH_MILLIS = 200L

/**
 * Lê o acelerômetro e entrega a inclinação do aparelho como volante (ver [MotionMath.roll]), com os
 * eixos girados conforme a tela. Só fica registrado entre [start] e [stop]. É independente do gyro
 * que vai ao PC: serve ao volante, que vira um analógico no próprio celular.
 */
internal class TiltSensor(context: Context, private val onRoll: (Float) -> Unit) : SensorEventListener {
    private val appContext = context.applicationContext
    private val sensors = appContext.getSystemService(Context.SENSOR_SERVICE) as? SensorManager
    private val displays = appContext.getSystemService(Context.DISPLAY_SERVICE) as? DisplayManager
    private val accelerometer = sensors?.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)

    private var smooth: Vec3? = null
    private var quarterTurns = 0
    private var rotationReadAt = 0L

    val available: Boolean get() = accelerometer != null

    @Synchronized
    fun start() {
        val manager = sensors ?: return
        smooth = null
        accelerometer?.let { manager.registerListener(this, it, SensorManager.SENSOR_DELAY_GAME) }
    }

    @Synchronized
    fun stop() {
        sensors?.unregisterListener(this)
        smooth = null
    }

    override fun onSensorChanged(event: SensorEvent) {
        if (event.sensor.type != Sensor.TYPE_ACCELEROMETER) return
        val v = event.values
        val raw = Vec3(v[0], v[1], v[2])
        val previous = smooth
        val next = if (previous == null) {
            raw
        } else {
            Vec3(
                previous.x + (raw.x - previous.x) * SMOOTHING,
                previous.y + (raw.y - previous.y) * SMOOTHING,
                previous.z + (raw.z - previous.z) * SMOOTHING,
            )
        }
        smooth = next
        val now = SystemClock.elapsedRealtime()
        if (now - rotationReadAt > ROTATION_REFRESH_MILLIS) {
            quarterTurns = currentQuarterTurns()
            rotationReadAt = now
        }
        onRoll(MotionMath.roll(MotionMath.toControllerFrame(next, quarterTurns)))
    }

    private fun currentQuarterTurns(): Int = when (displays?.getDisplay(Display.DEFAULT_DISPLAY)?.rotation) {
        Surface.ROTATION_90 -> 1
        Surface.ROTATION_180 -> 2
        Surface.ROTATION_270 -> 3
        else -> 0
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) = Unit
}
