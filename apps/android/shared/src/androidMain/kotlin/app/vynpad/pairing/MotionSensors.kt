package app.vynpad.pairing

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.hardware.display.DisplayManager
import android.os.Handler
import android.os.HandlerThread
import android.os.SystemClock
import android.view.Display
import android.view.Surface
import app.vynpad.input.GyroBiasEstimator
import app.vynpad.input.MotionMath
import app.vynpad.input.Vec3
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import uniffi.vynpad_client_core.MotionSample

/** A rotação da tela é lida a cada tanto (não a cada amostra de 400 Hz). */
private const val ROTATION_REFRESH_MILLIS = 200L

private const val PREFS = "vynpad_motion"

/**
 * Lê giroscópio e acelerômetro numa thread própria e entrega amostras no referencial do controle
 * (eixos girados conforme a tela, viés do gyro descontado).
 *
 * Só fica registrado nos sensores entre [start] e [stop]: com o gyro desligado no PC, nenhum
 * sensor roda (economia de bateria). O viés calibrado é guardado por aparelho.
 */
internal class MotionSensors(context: Context, private val onSample: (MotionSample) -> Unit) : SensorEventListener {
    private val appContext = context.applicationContext
    private val sensors = appContext.getSystemService(Context.SENSOR_SERVICE) as? SensorManager
    private val displays = appContext.getSystemService(Context.DISPLAY_SERVICE) as? DisplayManager
    private val prefs = appContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private val gyro = sensors?.getDefaultSensor(Sensor.TYPE_GYROSCOPE)
    private val accelerometer = sensors?.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)

    private val _calibrating = MutableStateFlow(false)

    /** `true` enquanto mede o viés (o usuário deve segurar o celular parado). */
    val calibrating: StateFlow<Boolean> = _calibrating.asStateFlow()

    private var thread: HandlerThread? = null

    // Tudo abaixo só é tocado na thread dos sensores (ou antes de registrar).
    private var bias = loadBias()
    private var accel = Vec3.Zero
    private var quarterTurns = 0
    private var rotationReadAt = 0L
    private val estimator = GyroBiasEstimator()

    @Volatile
    private var recalibrate = false

    /** Pedido (da UI) para o estimador recomeçar; quem mexe nele é só a thread dos sensores. */
    @Volatile
    private var restartEstimator = false

    val available: Boolean get() = gyro != null

    @Synchronized
    fun start() {
        val manager = sensors ?: return
        if (thread != null || gyro == null) return
        val worker = HandlerThread("vynpad-motion").also { it.start() }
        thread = worker
        val handler = Handler(worker.looper)
        // Sem viés guardado, mede já na primeira vez.
        recalibrate = prefs.getString(KEY_BIAS, null) == null
        _calibrating.value = recalibrate
        estimator.reset()
        manager.registerListener(this, gyro, SensorManager.SENSOR_DELAY_FASTEST, handler)
        accelerometer?.let { manager.registerListener(this, it, SensorManager.SENSOR_DELAY_FASTEST, handler) }
    }

    @Synchronized
    fun stop() {
        sensors?.unregisterListener(this)
        thread?.quitSafely()
        thread = null
        recalibrate = false
        _calibrating.value = false
    }

    /** Mede o viés de novo: o celular precisa ficar parado por ~2 s. */
    fun recalibrate() {
        restartEstimator = true
        recalibrate = true
        _calibrating.value = true
    }

    override fun onSensorChanged(event: SensorEvent) {
        val v = event.values
        when (event.sensor.type) {
            Sensor.TYPE_ACCELEROMETER -> accel = Vec3(v[0], v[1], v[2])
            Sensor.TYPE_GYROSCOPE -> onGyro(Vec3(v[0], v[1], v[2]))
        }
    }

    private fun onGyro(raw: Vec3) {
        val now = SystemClock.elapsedRealtime()
        if (recalibrate) {
            if (restartEstimator) {
                restartEstimator = false
                estimator.reset()
            }
            estimator.feed(raw, now)?.let { measured ->
                bias = measured
                saveBias(measured)
                recalibrate = false
                _calibrating.value = false
            }
        }
        if (now - rotationReadAt > ROTATION_REFRESH_MILLIS) {
            quarterTurns = currentQuarterTurns()
            rotationReadAt = now
        }
        val g = MotionMath.toControllerFrame(raw - bias, quarterTurns)
        val a = MotionMath.toControllerFrame(accel, quarterTurns)
        val w = MotionMath.toWire(g, a)
        onSample(
            MotionSample(
                gx = w.gx.toShort(),
                gy = w.gy.toShort(),
                gz = w.gz.toShort(),
                ax = w.ax.toShort(),
                ay = w.ay.toShort(),
                az = w.az.toShort(),
            ),
        )
    }

    private fun currentQuarterTurns(): Int = when (displays?.getDisplay(Display.DEFAULT_DISPLAY)?.rotation) {
        Surface.ROTATION_90 -> 1
        Surface.ROTATION_180 -> 2
        Surface.ROTATION_270 -> 3
        else -> 0
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) = Unit

    private fun loadBias(): Vec3 {
        val parts = prefs.getString(KEY_BIAS, null)?.split(',')?.mapNotNull { it.toFloatOrNull() }
        return if (parts != null && parts.size == 3) Vec3(parts[0], parts[1], parts[2]) else Vec3.Zero
    }

    private fun saveBias(b: Vec3) {
        prefs.edit().putString(KEY_BIAS, "${b.x},${b.y},${b.z}").apply()
    }

    private companion object {
        const val KEY_BIAS = "gyro_bias"
    }
}
