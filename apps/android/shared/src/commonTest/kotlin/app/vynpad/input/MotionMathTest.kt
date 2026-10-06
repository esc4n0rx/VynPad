package app.vynpad.input

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class MotionMathTest {
    private val v = Vec3(1f, 2f, 3f)

    @Test
    fun retratoNaoMudaOsEixos() {
        assertEquals(v, MotionMath.toControllerFrame(v, 0))
    }

    @Test
    fun rotation90GiraOsEixosComoOAndroid() {
        assertEquals(Vec3(-2f, 1f, 3f), MotionMath.toControllerFrame(v, 1))
    }

    @Test
    fun rotation270EOOpostoDaRotation90() {
        assertEquals(Vec3(2f, -1f, 3f), MotionMath.toControllerFrame(v, 3))
    }

    @Test
    fun rotation180InverteXeY() {
        assertEquals(Vec3(-1f, -2f, 3f), MotionMath.toControllerFrame(v, 2))
    }

    @Test
    fun quatroGiros90VoltamAoInicio() {
        var r = v
        repeat(4) { r = MotionMath.toControllerFrame(r, 1) }
        assertEquals(v, r)
    }

    @Test
    fun unidadesDoProtocolo() {
        val w = MotionMath.toWire(Vec3(0.5f, -1.2347f, 0f), Vec3(0f, 9.81f, -0.004f))
        assertEquals(500, w.gx)
        assertEquals(-1235, w.gy)
        assertEquals(981, w.ay)
        assertEquals(0, w.az)
    }

    @Test
    fun saturaEm16Bits() {
        val w = MotionMath.toWire(Vec3(100f, -100f, 0f), Vec3(1000f, -1000f, 0f))
        assertEquals(32767, w.gx)
        assertEquals(-32768, w.gy)
        assertEquals(32767, w.ax)
        assertEquals(-32768, w.ay)
    }

    @Test
    fun estimaOVieseComAparelhoParado() {
        val e = GyroBiasEstimator()
        var bias: Vec3? = null
        var t = 0L
        while (bias == null && t < 5_000) {
            bias = e.feed(Vec3(0.012f, -0.007f, 0.003f), t)
            t += 4
        }
        val b = assertNotNull(bias)
        assertTrue(t in 2_000..2_100)
        assertEquals(0.012f, b.x, 1e-4f)
        assertEquals(-0.007f, b.y, 1e-4f)
    }

    @Test
    fun movimentoReiniciaAJanela() {
        val e = GyroBiasEstimator()
        var t = 0L
        // 1,9 s parado e um tranco: a janela não pode fechar com esses dados.
        while (t < 1_900) {
            assertNull(e.feed(Vec3(0.01f, 0f, 0f), t))
            t += 4
        }
        assertNull(e.feed(Vec3(0.8f, 0f, 0f), t))
        t += 4
        // Mais 1 s parado depois do tranco ainda não basta.
        val end = t + 1_000
        while (t < end) {
            assertNull(e.feed(Vec3(0.01f, 0f, 0f), t))
            t += 4
        }
    }
}
