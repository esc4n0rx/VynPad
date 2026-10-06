package app.vynpad.input

import kotlin.test.Test
import kotlin.test.assertEquals

class RumbleMixTest {
    @Test
    fun zeroParaOsDoisMotoresDesliga() {
        assertEquals(0, RumbleMix.amplitude(0, 0, RumbleStrength.Strong))
    }

    @Test
    fun motorGrandeDominaQuandoMaior() {
        assertEquals(200, RumbleMix.amplitude(200, 100, RumbleStrength.Strong))
    }

    @Test
    fun motorPequenoEntraComPesoReduzido() {
        assertEquals(153, RumbleMix.amplitude(0, 255, RumbleStrength.Strong))
    }

    @Test
    fun forcaEscalaAAmplitude() {
        assertEquals(127, RumbleMix.amplitude(255, 0, RumbleStrength.Soft))
        assertEquals(191, RumbleMix.amplitude(255, 0, RumbleStrength.Medium))
    }

    @Test
    fun rumbleAtivoNuncaVira0PorArredondamento() {
        assertEquals(1, RumbleMix.amplitude(1, 0, RumbleStrength.Soft))
    }

    @Test
    fun valoresForaDaFaixaSaoLimitados() {
        assertEquals(255, RumbleMix.amplitude(999, 0, RumbleStrength.Strong))
        assertEquals(0, RumbleMix.amplitude(-5, -5, RumbleStrength.Strong))
    }
}
