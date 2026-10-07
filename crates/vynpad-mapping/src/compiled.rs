//! Perfil compilado: tudo que dá para calcular antes fica pré-calculado (LUTs das curvas,
//! tabelas de botões), para o `apply` do hot path só ler.

use crate::input::{BUTTON_COUNT, Button};
use crate::profile::{
    ButtonTarget, Curve, DeadzoneShape, GyroAxis, GyroConfig, GyroMode, GyroResponse, Profile,
    ProfileError, RumbleConfig, Smoothing, StickConfig, StickSide, TriggerConfig,
};

/// Divisões da LUT das curvas (a tabela tem `LUT_STEPS + 1` entradas).
pub const LUT_STEPS: usize = 1024;

/// Analógico compilado.
#[derive(Debug, Clone)]
pub struct CompiledStick {
    pub(crate) inner: f32,
    /// `1 - deadzone_outer - deadzone_inner`: largura do curso útil.
    pub(crate) span: f32,
    pub(crate) shape: DeadzoneShape,
    pub(crate) anti: f32,
    /// `None` para a curva linear (identidade exata, sem interpolação).
    pub(crate) lut: Option<Box<[f32]>>,
    pub(crate) sensitivity: f32,
    pub(crate) sign_x: f32,
    pub(crate) sign_y: f32,
}

/// Gatilho compilado.
#[derive(Debug, Clone, Copy)]
pub struct CompiledTrigger {
    pub(crate) deadzone: f32,
    pub(crate) span: f32,
    pub(crate) digital: bool,
    pub(crate) full_point: f32,
}

/// Modo do gyro compilado.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CompiledGyroMode {
    Always,
    WhileHeld(u16),
    Toggle(u16),
}

/// Gyro compilado.
#[derive(Debug, Clone, Copy)]
pub struct CompiledGyro {
    pub(crate) enabled: bool,
    pub(crate) mode: CompiledGyroMode,
    pub(crate) target: StickSide,
    pub(crate) axis_x: GyroAxis,
    pub(crate) axis_y: GyroAxis,
    pub(crate) response: GyroResponse,
    /// O ângulo (rad) que leva o analógico ao fim do curso, no modo de inclinação.
    pub(crate) tilt_range_rad: f32,
    /// Já inclui o sinal da inversão e a divisão pela escala de fundo.
    pub(crate) gain_x: f32,
    pub(crate) gain_y: f32,
    pub(crate) deadzone: f32,
    pub(crate) smoothing: Smoothing,
    pub(crate) recenter_mask: u16,
    pub(crate) mix: bool,
}

/// Destino compilado de um botão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Mask(u16),
    Nothing,
    LeftTrigger,
    RightTrigger,
}

/// Perfil pronto para o `apply`.
#[derive(Debug, Clone)]
pub struct CompiledProfile {
    pub(crate) name: String,
    pub(crate) left: CompiledStick,
    pub(crate) right: CompiledStick,
    pub(crate) swap: bool,
    pub(crate) triggers: [CompiledTrigger; 2],
    pub(crate) gyro: CompiledGyro,
    /// Destino de cada botão, na ordem de `Button::ALL`.
    pub(crate) targets: [Target; BUTTON_COUNT],
    /// Frequência do turbo por botão (0 = sem turbo).
    pub(crate) turbo_hz: [f32; BUTTON_COUNT],
    pub(crate) rumble: RumbleConfig,
}

/// Rad/s que levam o analógico ao fim do curso com sensibilidade 1,0 (~115°/s).
pub const GYRO_FULL_SCALE_RAD_S: f32 = 2.0;

impl CompiledProfile {
    /// Nome do perfil.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// O gyro está ligado neste perfil (o server só pede pacotes com motion se estiver).
    pub const fn gyro_enabled(&self) -> bool {
        self.gyro.enabled
    }

    /// O perfil repassa a vibração ao celular.
    pub const fn rumble_enabled(&self) -> bool {
        self.rumble.enabled
    }

    /// A vibração pedida pelo jogo (`large`, `small`) depois da intensidade do perfil. Desligada,
    /// devolve `(0, 0)`; acima de 100% satura em 255.
    pub fn scale_rumble(&self, large: u8, small: u8) -> (u8, u8) {
        if !self.rumble.enabled {
            return (0, 0);
        }
        let scale = |v: u8| {
            // `intensity` ≤ 2 e `v` ≤ 255: o resultado cabe em u8 depois do `min`.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let scaled = (f32::from(v) * self.rumble.intensity).round().min(255.0) as u8;
            scaled
        };
        (scale(large), scale(small))
    }

    /// Valida e compila um perfil.
    pub fn compile(profile: &Profile) -> Result<Self, ProfileError> {
        profile.validate()?;
        let mut targets = Button::ALL.map(|b| Target::Mask(b.mask()));
        for (button, target) in &profile.buttons.remap {
            targets[button.index()] = match *target {
                ButtonTarget::None => Target::Nothing,
                ButtonTarget::LeftTrigger => Target::LeftTrigger,
                ButtonTarget::RightTrigger => Target::RightTrigger,
                ButtonTarget::Button(b) => Target::Mask(b.mask()),
            };
        }
        let mut turbo_hz = [0.0; BUTTON_COUNT];
        for turbo in &profile.buttons.turbo {
            turbo_hz[turbo.button.index()] = turbo.hz;
        }
        Ok(Self {
            name: profile.name.clone(),
            left: compile_stick(&profile.sticks.left),
            right: compile_stick(&profile.sticks.right),
            swap: profile.sticks.swap,
            triggers: [
                compile_trigger(&profile.triggers.left),
                compile_trigger(&profile.triggers.right),
            ],
            gyro: compile_gyro(&profile.gyro),
            targets,
            turbo_hz,
            rumble: profile.rumble,
        })
    }

    /// O perfil "Padrão" embutido, já compilado.
    pub fn standard() -> Self {
        Self::compile(&Profile::standard()).expect("perfil padrão é válido")
    }

    /// O perfil identidade, já compilado.
    pub fn identity() -> Self {
        Self::compile(&Profile::identity()).expect("perfil identidade é válido")
    }
}

fn compile_stick(s: &StickConfig) -> CompiledStick {
    CompiledStick {
        inner: s.deadzone_inner,
        span: 1.0 - s.deadzone_outer - s.deadzone_inner,
        shape: s.deadzone_shape,
        anti: s.anti_deadzone,
        lut: match s.curve {
            Curve::Linear => None,
            curve => Some(build_lut(curve)),
        },
        sensitivity: s.sensitivity,
        sign_x: if s.invert_x { -1.0 } else { 1.0 },
        sign_y: if s.invert_y { -1.0 } else { 1.0 },
    }
}

fn compile_trigger(t: &TriggerConfig) -> CompiledTrigger {
    CompiledTrigger {
        deadzone: t.deadzone,
        span: t.full_point - t.deadzone,
        digital: t.digital,
        full_point: t.full_point,
    }
}

fn compile_gyro(g: &GyroConfig) -> CompiledGyro {
    // Em `rate` o ganho já traz a divisão pela escala de fundo (rad/s); em `tilt` o sinal sai
    // normalizado pelo ângulo de fim de curso, então o ganho é só sinal e sensibilidade.
    let gain = |sensitivity: f32, invert: bool| {
        let sign = if invert { -1.0 } else { 1.0 };
        match g.response {
            GyroResponse::Rate => sign * sensitivity / GYRO_FULL_SCALE_RAD_S,
            GyroResponse::Tilt => sign * sensitivity,
        }
    };
    CompiledGyro {
        enabled: g.enabled,
        mode: match g.mode {
            GyroMode::Always => CompiledGyroMode::Always,
            GyroMode::WhileHeld { button } => CompiledGyroMode::WhileHeld(button.mask()),
            GyroMode::Toggle { button } => CompiledGyroMode::Toggle(button.mask()),
        },
        target: g.target,
        axis_x: g.axis_x,
        axis_y: g.axis_y,
        response: g.response,
        tilt_range_rad: g.tilt_range_deg.to_radians(),
        gain_x: gain(g.sensitivity_x, g.invert_x),
        gain_y: gain(g.sensitivity_y, g.invert_y),
        deadzone: g.deadzone,
        smoothing: g.smoothing,
        recenter_mask: g.recenter_button.map_or(0, Button::mask),
        mix: g.mix_with_stick,
    }
}

/// Amostra a curva em `LUT_STEPS + 1` pontos de 0 a 1.
fn build_lut(curve: Curve) -> Box<[f32]> {
    (0..=LUT_STEPS)
        .map(|i| eval_curve(curve, i as f32 / LUT_STEPS as f32))
        .collect()
}

/// Avalia a curva em `x` (0–1); passa sempre por (0,0) e (1,1).
fn eval_curve(curve: Curve, x: f32) -> f32 {
    match curve {
        Curve::Linear => x,
        Curve::Exponential { exponent } => x.powf(exponent),
        Curve::Custom { points } => {
            let mut prev = (0.0_f32, 0.0_f32);
            for next in points.into_iter().chain([(1.0, 1.0)]) {
                if x <= next.0 {
                    let t = (x - prev.0) / (next.0 - prev.0);
                    return prev.1 + (next.1 - prev.1) * t;
                }
                prev = next;
            }
            1.0
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // comparações exatas são intencionais nestes testes
mod tests {
    use super::*;
    use crate::profile::Turbo;

    #[test]
    fn linear_curve_has_no_lut_and_others_pass_through_the_corners() {
        assert!(CompiledProfile::identity().left.lut.is_none());
        for curve in [
            Curve::Exponential { exponent: 2.0 },
            Curve::Custom {
                points: [(0.2, 0.1), (0.4, 0.3), (0.6, 0.6), (0.8, 0.85)],
            },
        ] {
            let lut = build_lut(curve);
            assert_eq!(lut.len(), LUT_STEPS + 1);
            assert_eq!(lut[0], 0.0);
            assert!((lut[LUT_STEPS] - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn custom_curve_interpolates_between_its_points() {
        let curve = Curve::Custom {
            points: [(0.25, 0.5), (0.5, 0.5), (0.75, 0.5), (0.9, 0.5)],
        };
        assert!((eval_curve(curve, 0.125) - 0.25).abs() < 1e-6);
        assert!((eval_curve(curve, 0.6) - 0.5).abs() < 1e-6);
        assert!((eval_curve(curve, 1.0) - 1.0).abs() < 1e-6);
        assert!((eval_curve(curve, 0.95) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn remap_and_turbo_tables_are_built() {
        let mut profile = Profile::identity();
        profile
            .buttons
            .remap
            .insert(Button::A, ButtonTarget::Button(Button::B));
        profile.buttons.remap.insert(Button::X, ButtonTarget::None);
        profile
            .buttons
            .remap
            .insert(Button::Y, ButtonTarget::LeftTrigger);
        profile.buttons.turbo.push(Turbo {
            button: Button::Start,
            hz: 10.0,
        });
        let compiled = CompiledProfile::compile(&profile).unwrap();
        assert_eq!(
            compiled.targets[Button::A.index()],
            Target::Mask(Button::B.mask())
        );
        assert_eq!(compiled.targets[Button::X.index()], Target::Nothing);
        assert_eq!(compiled.targets[Button::Y.index()], Target::LeftTrigger);
        assert_eq!(
            compiled.targets[Button::B.index()],
            Target::Mask(Button::B.mask())
        );
        assert_eq!(compiled.turbo_hz[Button::Start.index()], 10.0);
        assert_eq!(compiled.turbo_hz[Button::A.index()], 0.0);
        assert_eq!(compiled.name(), "Identidade");
    }

    #[test]
    fn invalid_profiles_do_not_compile() {
        let mut profile = Profile::identity();
        profile.sticks.left.sensitivity = 0.0;
        assert!(CompiledProfile::compile(&profile).is_err());
    }

    #[test]
    fn gyro_gain_carries_sign_and_scale() {
        let mut profile = Profile::identity();
        profile.gyro.sensitivity_x = 2.0;
        profile.gyro.invert_y = true;
        let gyro = CompiledProfile::compile(&profile).unwrap().gyro;
        assert_eq!(gyro.gain_x, 2.0 / GYRO_FULL_SCALE_RAD_S);
        assert_eq!(gyro.gain_y, -1.0 / GYRO_FULL_SCALE_RAD_S);
    }
}

#[cfg(test)]
mod rumble_tests {
    use super::*;

    fn with(rumble: RumbleConfig) -> CompiledProfile {
        CompiledProfile::compile(&Profile {
            rumble,
            ..Profile::identity()
        })
        .unwrap()
    }

    #[test]
    fn the_default_profile_passes_the_vibration_through() {
        let p = CompiledProfile::identity();
        assert!(p.rumble_enabled());
        assert_eq!(p.scale_rumble(200, 17), (200, 17));
        assert_eq!(p.scale_rumble(0, 0), (0, 0));
    }

    #[test]
    fn intensity_scales_and_saturates_at_255() {
        let half = with(RumbleConfig {
            enabled: true,
            intensity: 0.5,
        });
        assert_eq!(half.scale_rumble(200, 101), (100, 51));
        let double = with(RumbleConfig {
            enabled: true,
            intensity: 2.0,
        });
        assert_eq!(double.scale_rumble(100, 200), (200, 255));
        assert_eq!(double.scale_rumble(255, 0), (255, 0));
    }

    #[test]
    fn turning_it_off_silences_the_motors() {
        let off = with(RumbleConfig {
            enabled: false,
            intensity: 1.0,
        });
        assert!(!off.rumble_enabled());
        assert_eq!(off.scale_rumble(255, 255), (0, 0));
    }

    #[test]
    fn the_intensity_is_validated() {
        for bad in [-0.1, 2.1, f32::NAN, f32::INFINITY] {
            let profile = Profile {
                rumble: RumbleConfig {
                    enabled: true,
                    intensity: bad,
                },
                ..Profile::identity()
            };
            assert!(profile.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn old_profiles_without_a_rumble_section_still_load() {
        let profile = Profile::from_json(r#"{"schema_version":1,"name":"Antigo"}"#).unwrap();
        assert_eq!(profile.rumble, RumbleConfig::default());
    }
}
