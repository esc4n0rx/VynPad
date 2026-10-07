//! O pipeline: [`apply`] transforma o input bruto no `XReport` final.
//!
//! Ordem: troca dos analógicos → deadzone, anti-deadzone, curva, sensibilidade e inversão de
//! cada analógico → gyro somado ao analógico alvo → clamp → gatilhos → botões (remapeamento e
//! turbo). Sem alocação e sem `unsafe`; o único estado entre chamadas é o [`MapperState`].

use std::sync::Arc;

use arc_swap::{ArcSwap, Guard};
use vynpad_driver::XReport;

use crate::compiled::{
    CompiledGyro, CompiledGyroMode, CompiledProfile, CompiledStick, CompiledTrigger, LUT_STEPS,
    Target,
};
use crate::filter::OneEuro;
use crate::input::{Accel, BUTTON_COUNT, Button, Gyro, RawInput};
use crate::profile::{DeadzoneShape, GyroAxis, GyroResponse, StickSide};

/// Maior `dt` aceito em segundos; acima disso (pausa, troca de aba) o intervalo é limitado.
const MAX_DT: f32 = 1.0;
/// Depois deste tempo o relógio do turbo dá a volta, para o `f32` não perder precisão.
const TURBO_WRAP_S: f32 = 1024.0;

/// Gravidade padrão, em m/s².
const GRAVITY: f32 = 9.806_65;
/// Quanto o módulo da aceleração pode fugir da gravidade (m/s²) e ainda valer como "para baixo".
/// Acima disso (solavanco, trepidação) só o giroscópio conduz o ângulo.
const ACCEL_TRUST_TOLERANCE: f32 = 3.0;
/// Componente mínima da gravidade (m/s²) no plano do eixo para o ângulo medido ser confiável: com
/// o aparelho quase perpendicular ao plano, a direção da gravidade vira ruído.
const ACCEL_MIN_PLANE: f32 = 3.0;
/// Constante de tempo (s) com que o acelerômetro corrige a deriva do giroscópio.
const TILT_TAU: f32 = 0.3;

/// O ângulo de inclinação de um eixo: giroscópio integrado e corrigido pela gravidade.
#[derive(Debug, Clone, Copy, Default)]
struct TiltAxis {
    /// Ângulo absoluto (rad) estimado.
    angle: f32,
    /// Ângulo tomado como neutro (o que o botão de recentralizar fixa).
    reference: f32,
    /// O ângulo já foi semeado pela gravidade (senão a próxima amostra o semeia).
    seeded: bool,
    /// Ao semear, também define a referência (pedido de recentralizar).
    rebase: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct GyroState {
    filter_x: OneEuro,
    filter_y: OneEuro,
    tilt_x: TiltAxis,
    tilt_y: TiltAxis,
    /// Modo alternado: ligado?
    toggled: bool,
    toggle_was_down: bool,
    recenter_was_down: bool,
}

impl GyroState {
    fn reset_filters(&mut self) {
        self.filter_x.reset();
        self.filter_y.reset();
        // O ângulo é semeado de novo pela gravidade (sem acumular deriva enquanto o gyro dormia).
        self.tilt_x.seeded = false;
        self.tilt_y.seeded = false;
    }

    /// Recentralizar no modo de inclinação: a posição atual do aparelho vira o neutro.
    fn recenter(&mut self) {
        self.reset_filters();
        self.tilt_x.rebase = true;
        self.tilt_y.rebase = true;
    }
}

/// Estado que o pipeline carrega entre chamadas (um por slot).
#[derive(Debug, Clone, Copy, Default)]
pub struct MapperState {
    gyro: GyroState,
    /// Há quanto tempo (s) cada botão está pressionado, para o turbo.
    turbo_hold: [f32; BUTTON_COUNT],
}

impl MapperState {
    /// Estado inicial.
    pub fn new() -> Self {
        Self::default()
    }
}

fn normalize(v: i16) -> f32 {
    if v < 0 {
        f32::from(v) / 32768.0
    } else {
        f32::from(v) / 32767.0
    }
}

fn to_i16(v: f32) -> i16 {
    let v = v.clamp(-1.0, 1.0);
    // `as` satura e converte NaN em 0.
    if v < 0.0 {
        (v * 32768.0).round() as i16
    } else {
        (v * 32767.0).round() as i16
    }
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Curva da LUT; acima de 1 (cantos do quadrado) a saída acompanha a entrada.
fn shape(lut: Option<&[f32]>, m: f32) -> f32 {
    let Some(lut) = lut else { return m };
    if m >= 1.0 {
        return m;
    }
    let pos = m * LUT_STEPS as f32;
    let i = pos as usize; // m < 1 => i < LUT_STEPS
    let frac = pos - i as f32;
    lut[i] + (lut[i + 1] - lut[i]) * frac
}

/// Magnitude (0–1, ou um pouco mais nos cantos) → saída, com deadzone, anti-deadzone, curva
/// e sensibilidade.
fn respond(s: &CompiledStick, m: f32) -> f32 {
    if m <= s.inner {
        return 0.0;
    }
    let mut scaled = (m - s.inner) / s.span;
    if s.anti > 0.0 {
        scaled = s.anti + (1.0 - s.anti) * scaled;
    }
    shape(s.lut.as_deref(), scaled) * s.sensitivity
}

fn stick(s: &CompiledStick, x: f32, y: f32) -> (f32, f32) {
    let (x, y) = match s.shape {
        DeadzoneShape::Radial => {
            let mag = x.hypot(y);
            if mag == 0.0 {
                (0.0, 0.0)
            } else {
                let k = respond(s, mag) / mag;
                (x * k, y * k)
            }
        }
        DeadzoneShape::Axial => (
            respond(s, x.abs()).copysign(x),
            respond(s, y.abs()).copysign(y),
        ),
    };
    (x * s.sign_x, y * s.sign_y)
}

fn trigger(t: &CompiledTrigger, raw: u8) -> f32 {
    let v = f32::from(raw) / 255.0;
    if t.digital {
        return if v >= t.full_point && v > t.deadzone {
            1.0
        } else {
            0.0
        };
    }
    if v <= t.deadzone {
        0.0
    } else {
        ((v - t.deadzone) / t.span).min(1.0)
    }
}

/// Velocidade angular que alimenta um eixo do analógico (positivo = direita / cima).
fn gyro_signal(axis: GyroAxis, g: &Gyro) -> f32 {
    match axis {
        GyroAxis::Yaw => -g.y,
        GyroAxis::Pitch => -g.x,
        GyroAxis::Roll => -g.z,
    }
}

/// Leva um ângulo para o intervalo (−π, π].
fn wrap_angle(a: f32) -> f32 {
    let mut a = a % core::f32::consts::TAU;
    if a > core::f32::consts::PI {
        a -= core::f32::consts::TAU;
    } else if a <= -core::f32::consts::PI {
        a += core::f32::consts::TAU;
    }
    a
}

/// Ângulo (rad) do eixo medido só pela gravidade, e o tamanho da gravidade no plano dele.
///
/// - `Roll`: girar como um volante (em torno de Z), positivo = anti-horário.
/// - `Pitch`: topo do aparelho para o jogador (em torno de X), positivo = topo para o jogador.
/// - `Yaw`: com o aparelho deitado, inclinar para os lados (em torno de Y), positivo = lado
///   direito para baixo.
fn tilt_measure(axis: GyroAxis, a: &Accel) -> (f32, f32) {
    let (y, x) = match axis {
        GyroAxis::Roll => (a.x, a.y),
        GyroAxis::Pitch => (-a.z, a.y),
        GyroAxis::Yaw => (-a.x, a.z),
    };
    (y.atan2(x), y.hypot(x))
}

/// Velocidade de giro (rad/s) em torno do eixo, com o mesmo sinal de [`tilt_measure`].
fn tilt_rate(axis: GyroAxis, g: &Gyro) -> f32 {
    match axis {
        GyroAxis::Roll => g.z,
        GyroAxis::Pitch => g.x,
        GyroAxis::Yaw => g.y,
    }
}

/// Sinal do analógico (positivo = direita / cima) para um ângulo positivo do eixo: girar o volante
/// no sentido horário, inclinar o topo para longe e baixar o lado direito levam à direita / para cima.
const fn tilt_polarity(axis: GyroAxis) -> f32 {
    match axis {
        GyroAxis::Roll | GyroAxis::Pitch => -1.0,
        GyroAxis::Yaw => 1.0,
    }
}

/// Avança o ângulo de um eixo: integra o giroscópio e o puxa para o que a gravidade indica.
fn tilt_update(t: &mut TiltAxis, axis: GyroAxis, accel: &Accel, gyro: &Gyro, dt: f32) {
    let (measured, plane) = tilt_measure(axis, accel);
    let magnitude = accel.x.hypot(accel.y).hypot(accel.z);
    let gravity_like =
        (magnitude - GRAVITY).abs() <= ACCEL_TRUST_TOLERANCE && plane >= ACCEL_MIN_PLANE;
    if !t.seeded {
        t.angle = if gravity_like { measured } else { 0.0 };
        if t.rebase {
            t.reference = t.angle;
            t.rebase = false;
        }
        t.seeded = true;
        return;
    }
    t.angle = wrap_angle(t.angle + tilt_rate(axis, gyro) * dt);
    if gravity_like {
        let k = dt / (TILT_TAU + dt);
        t.angle = wrap_angle(t.angle + k * wrap_angle(measured - t.angle));
    }
}

/// O ângulo do eixo em relação ao neutro, normalizado pelo fim de curso e com a polaridade do analógico.
fn tilt_signal(t: &TiltAxis, axis: GyroAxis, range_rad: f32) -> f32 {
    tilt_polarity(axis) * wrap_angle(t.angle - t.reference) / range_rad
}

fn soft_deadzone(v: f32, dz: f32) -> f32 {
    let a = v.abs();
    if a <= dz { 0.0 } else { (a - dz).copysign(v) }
}

/// Contribuição do gyro ao analógico alvo, ou `None` se ele não age neste quadro.
fn gyro_step(g: &CompiledGyro, st: &mut GyroState, raw: &RawInput, dt: f32) -> Option<(f32, f32)> {
    if !g.enabled {
        return None;
    }
    // Bordas dos botões são acompanhadas mesmo sem pacote de motion.
    if let CompiledGyroMode::Toggle(mask) = g.mode {
        let down = raw.buttons & mask != 0;
        if down && !st.toggle_was_down {
            st.toggled = !st.toggled;
        }
        st.toggle_was_down = down;
    }
    let recenter_down = g.recenter_mask != 0 && raw.buttons & g.recenter_mask != 0;
    if recenter_down && !st.recenter_was_down {
        st.recenter();
        st.toggled = false;
    }
    st.recenter_was_down = recenter_down;

    let active = match g.mode {
        CompiledGyroMode::Always => true,
        CompiledGyroMode::WhileHeld(mask) => raw.buttons & mask != 0,
        CompiledGyroMode::Toggle(_) => st.toggled,
    };
    let (true, Some(gyro)) = (active, raw.gyro.as_ref()) else {
        st.reset_filters();
        return None;
    };

    let (mut x, mut y) = match g.response {
        GyroResponse::Rate => (
            soft_deadzone(gyro_signal(g.axis_x, gyro), g.deadzone),
            soft_deadzone(gyro_signal(g.axis_y, gyro), g.deadzone),
        ),
        GyroResponse::Tilt => {
            // Sem acelerômetro no pacote não há ângulo: o modo fica quieto.
            let Some(accel) = raw.accel.as_ref() else {
                st.reset_filters();
                return None;
            };
            tilt_update(&mut st.tilt_x, g.axis_x, accel, gyro, dt);
            tilt_update(&mut st.tilt_y, g.axis_y, accel, gyro, dt);
            (
                soft_deadzone(
                    tilt_signal(&st.tilt_x, g.axis_x, g.tilt_range_rad),
                    g.deadzone,
                ),
                soft_deadzone(
                    tilt_signal(&st.tilt_y, g.axis_y, g.tilt_range_rad),
                    g.deadzone,
                ),
            )
        }
    };
    if g.smoothing.enabled {
        x = st
            .filter_x
            .filter(x, dt, g.smoothing.min_cutoff, g.smoothing.beta);
        y = st
            .filter_y
            .filter(y, dt, g.smoothing.min_cutoff, g.smoothing.beta);
    }
    let (x, y) = (x * g.gain_x, y * g.gain_y);
    // Inclinação além do fim de curso (ou sensibilidade alta) trava no máximo, em vez de passar.
    Some(match g.response {
        GyroResponse::Rate => (x, y),
        GyroResponse::Tilt => (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)),
    })
}

/// Aplica o perfil a um quadro de input.
///
/// `dt` é o tempo desde o quadro anterior, em segundos (filtro do gyro e turbo). Valores
/// negativos, infinitos ou NaN são tratados como 0, e acima de 1 s como 1 s.
pub fn apply(
    profile: &CompiledProfile,
    raw: &RawInput,
    state: &mut MapperState,
    dt: f32,
) -> XReport {
    let dt = if dt.is_finite() {
        dt.clamp(0.0, MAX_DT)
    } else {
        0.0
    };

    // Analógicos.
    let (mut left_in, mut right_in) = (
        (normalize(raw.lx), normalize(raw.ly)),
        (normalize(raw.rx), normalize(raw.ry)),
    );
    if profile.swap {
        std::mem::swap(&mut left_in, &mut right_in);
    }
    let mut left = stick(&profile.left, left_in.0, left_in.1);
    let mut right = stick(&profile.right, right_in.0, right_in.1);

    // Gyro no analógico alvo.
    if let Some((gx, gy)) = gyro_step(&profile.gyro, &mut state.gyro, raw, dt) {
        let target = match profile.gyro.target {
            StickSide::Left => &mut left,
            StickSide::Right => &mut right,
        };
        *target = if profile.gyro.mix {
            (target.0 + gx, target.1 + gy)
        } else {
            (gx, gy)
        };
    }

    // Gatilhos.
    let mut lt = trigger(&profile.triggers[0], raw.lt);
    let mut rt = trigger(&profile.triggers[1], raw.rt);

    // Botões: remapeamento e turbo.
    let mut buttons = 0u16;
    for (i, button) in Button::ALL.into_iter().enumerate() {
        let held = raw.buttons & button.mask() != 0;
        if !held {
            state.turbo_hold[i] = 0.0;
            continue;
        }
        let hz = profile.turbo_hz[i];
        let pressed = if hz > 0.0 {
            let pressed_for = state.turbo_hold[i];
            state.turbo_hold[i] = (pressed_for + dt) % TURBO_WRAP_S;
            // Começa pressionado e alterna a cada meio período.
            ((pressed_for * hz * 2.0) as u32).is_multiple_of(2)
        } else {
            true
        };
        if pressed {
            match profile.targets[i] {
                Target::Mask(mask) => buttons |= mask,
                Target::Nothing => {}
                Target::LeftTrigger => lt = 1.0,
                Target::RightTrigger => rt = 1.0,
            }
        }
    }

    XReport {
        buttons,
        lt: to_u8(lt),
        rt: to_u8(rt),
        lx: to_i16(left.0),
        ly: to_i16(left.1),
        rx: to_i16(right.0),
        ry: to_i16(right.1),
    }
}

/// O perfil ativo de um slot, trocável em tempo real sem travar o thread de input.
#[derive(Debug)]
pub struct ProfileCell {
    inner: ArcSwap<CompiledProfile>,
}

impl ProfileCell {
    /// Cria a célula com o perfil inicial.
    pub fn new(profile: CompiledProfile) -> Self {
        Self {
            inner: ArcSwap::from_pointee(profile),
        }
    }

    /// Troca o perfil; a próxima chamada de `apply` já usa o novo.
    pub fn store(&self, profile: Arc<CompiledProfile>) {
        self.inner.store(profile);
    }

    /// Lê o perfil atual, sem lock e sem alocação.
    pub fn load(&self) -> Guard<Arc<CompiledProfile>> {
        self.inner.load()
    }
}

/// Um slot: o perfil (compartilhado com a UI) e o estado do pipeline.
#[derive(Debug)]
pub struct Mapper {
    cell: Arc<ProfileCell>,
    state: MapperState,
}

impl Mapper {
    /// Cria um mapper que segue `cell`.
    pub fn new(cell: Arc<ProfileCell>) -> Self {
        Self {
            cell,
            state: MapperState::new(),
        }
    }

    /// Mapeia um quadro com o perfil atual da célula.
    pub fn map(&mut self, raw: &RawInput, dt: f32) -> XReport {
        let profile = self.cell.load();
        apply(&profile, raw, &mut self.state, dt)
    }
}
