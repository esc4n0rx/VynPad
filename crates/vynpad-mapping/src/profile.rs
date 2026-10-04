//! Perfil de mapeamento: o que o usuário edita e o que vai para o JSON.
//!
//! Todos os campos têm valor padrão, então um perfil parcial é válido. `schema_version`
//! permite migrar perfis antigos no futuro; hoje só a versão 1 existe.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::input::Button;

/// Versão atual do formato JSON dos perfis.
pub const SCHEMA_VERSION: u32 = 1;

/// Erro ao ler ou validar um perfil.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileError {
    /// JSON inválido.
    #[error("JSON de perfil inválido: {0}")]
    Json(String),
    /// `schema_version` que esta versão do app não conhece.
    #[error("versão de perfil {0} não suportada (esperada {SCHEMA_VERSION})")]
    UnsupportedVersion(u32),
    /// Um campo está fora da faixa permitida.
    #[error("campo `{field}` inválido: {reason}")]
    Invalid {
        /// Caminho do campo (`sticks.left.deadzone_inner`...).
        field: String,
        /// Por que foi recusado.
        reason: &'static str,
    },
}

// ---------------------------------------------------------------------- sticks

/// Forma da deadzone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum DeadzoneShape {
    /// Pela distância ao centro (o analógico vira um círculo).
    #[default]
    Radial,
    /// Cada eixo separado (um quadrado).
    Axial,
}

/// Curva de resposta, de 0 a 1 (a curva sempre passa por (0,0) e (1,1)).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Curve {
    /// Resposta 1:1.
    #[default]
    Linear,
    /// `y = x^exponent`; maior que 1 suaviza o centro, menor que 1 o acelera.
    Exponential {
        /// Expoente (0,25 a 4).
        exponent: f32,
    },
    /// Quatro pontos `(x, y)` entre (0,0) e (1,1), com `x` estritamente crescente.
    Custom {
        /// Os pontos de controle.
        points: [(f32, f32); 4],
    },
}

/// Ajustes de um analógico.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct StickConfig {
    /// Deadzone interna (fração 0–1 do curso): abaixo disso a saída é zero.
    pub deadzone_inner: f32,
    /// Deadzone externa (fração 0–1): o curso termina esta fração antes da borda.
    pub deadzone_outer: f32,
    /// Forma das deadzones.
    pub deadzone_shape: DeadzoneShape,
    /// Anti-deadzone (fração 0–1): a saída começa neste valor assim que sai da deadzone,
    /// para vencer a deadzone do próprio jogo.
    pub anti_deadzone: f32,
    /// Curva de resposta.
    pub curve: Curve,
    /// Sensibilidade (multiplicador da saída, 0,1 a 4).
    pub sensitivity: f32,
    /// Inverte o eixo X.
    pub invert_x: bool,
    /// Inverte o eixo Y.
    pub invert_y: bool,
}

impl Default for StickConfig {
    /// Identidade: nenhuma alteração.
    fn default() -> Self {
        Self {
            deadzone_inner: 0.0,
            deadzone_outer: 0.0,
            deadzone_shape: DeadzoneShape::Radial,
            anti_deadzone: 0.0,
            curve: Curve::Linear,
            sensitivity: 1.0,
            invert_x: false,
            invert_y: false,
        }
    }
}

/// Os dois analógicos.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct SticksConfig {
    /// Analógico esquerdo.
    pub left: StickConfig,
    /// Analógico direito.
    pub right: StickConfig,
    /// Troca o esquerdo com o direito (antes dos ajustes acima).
    pub swap: bool,
}

// ---------------------------------------------------------------------- gatilhos

/// Ajustes de um gatilho.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct TriggerConfig {
    /// Deadzone (fração 0–1): abaixo disso a saída é zero.
    pub deadzone: f32,
    /// Ponto de acionamento total (fração 0–1): a partir dele a saída é 255.
    pub full_point: f32,
    /// Modo digital: a saída é 0 ou 255, e liga em `full_point`.
    pub digital: bool,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.0,
            full_point: 1.0,
            digital: false,
        }
    }
}

/// Os dois gatilhos.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct TriggersConfig {
    /// Gatilho esquerdo.
    pub left: TriggerConfig,
    /// Gatilho direito.
    pub right: TriggerConfig,
}

// ---------------------------------------------------------------------- gyro

/// Quando o gyro age.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GyroMode {
    /// Sempre.
    #[default]
    Always,
    /// Enquanto o botão está pressionado.
    WhileHeld {
        /// O botão (lido da entrada bruta, antes do remapeamento).
        button: Button,
    },
    /// Cada toque no botão liga ou desliga.
    Toggle {
        /// O botão (lido da entrada bruta, antes do remapeamento).
        button: Button,
    },
}

/// Analógico que recebe o gyro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum StickSide {
    /// Esquerdo.
    Left,
    /// Direito.
    #[default]
    Right,
}

/// Movimento físico que alimenta um eixo do analógico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum GyroAxis {
    /// Virar o aparelho para os lados (em torno do eixo Y).
    Yaw,
    /// Inclinar o aparelho para cima e para baixo (em torno do eixo X).
    Pitch,
    /// Girar o aparelho como um volante (em torno do eixo Z).
    Roll,
}

/// Filtro 1-Euro (suaviza o tremor sem atrasar movimentos rápidos).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Smoothing {
    /// Liga o filtro.
    pub enabled: bool,
    /// Frequência de corte mínima, em Hz (menor = mais suave).
    pub min_cutoff: f32,
    /// Quanto a velocidade abre o filtro (maior = menos atraso em movimentos rápidos).
    pub beta: f32,
}

impl Default for Smoothing {
    fn default() -> Self {
        Self {
            enabled: true,
            min_cutoff: 1.0,
            beta: 0.05,
        }
    }
}

/// Gyro → analógico.
// Espelha o JSON do perfil, que tem várias chaves booleanas independentes.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct GyroConfig {
    /// Liga o gyro.
    pub enabled: bool,
    /// Quando age.
    pub mode: GyroMode,
    /// Analógico alvo.
    pub target: StickSide,
    /// Movimento que move o eixo X do analógico.
    pub axis_x: GyroAxis,
    /// Movimento que move o eixo Y do analógico.
    pub axis_y: GyroAxis,
    /// Sensibilidade em X (0–10). 1,0 = analógico no fim do curso a ~115°/s.
    pub sensitivity_x: f32,
    /// Sensibilidade em Y (0–10).
    pub sensitivity_y: f32,
    /// Inverte o eixo X.
    pub invert_x: bool,
    /// Inverte o eixo Y.
    pub invert_y: bool,
    /// Deadzone do gyro, em rad/s (0–1): velocidades menores são ignoradas.
    pub deadzone: f32,
    /// Suavização.
    pub smoothing: Smoothing,
    /// Botão que recentraliza (zera o filtro e desliga o modo alternado).
    pub recenter_button: Option<Button>,
    /// `true` soma o gyro ao analógico (com clamp); `false` o substitui enquanto o gyro age.
    pub mix_with_stick: bool,
}

impl Default for GyroConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: GyroMode::Always,
            target: StickSide::Right,
            axis_x: GyroAxis::Yaw,
            axis_y: GyroAxis::Pitch,
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            invert_x: false,
            invert_y: false,
            deadzone: 0.02,
            smoothing: Smoothing::default(),
            recenter_button: None,
            mix_with_stick: true,
        }
    }
}

// ---------------------------------------------------------------------- botões

/// Para onde vai um botão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(try_from = "String", into = "String")]
pub enum ButtonTarget {
    /// Nenhum (o botão é ignorado).
    None,
    /// Aciona o gatilho esquerdo por completo.
    LeftTrigger,
    /// Aciona o gatilho direito por completo.
    RightTrigger,
    /// Outro botão.
    Button(Button),
}

impl TryFrom<String> for ButtonTarget {
    type Error = String;

    fn try_from(name: String) -> Result<Self, String> {
        match name.as_str() {
            "none" => Ok(Self::None),
            "left_trigger" => Ok(Self::LeftTrigger),
            "right_trigger" => Ok(Self::RightTrigger),
            other => Button::from_name(other)
                .map(Self::Button)
                .ok_or_else(|| format!("destino de botão desconhecido: {other}")),
        }
    }
}

impl From<ButtonTarget> for String {
    fn from(target: ButtonTarget) -> Self {
        match target {
            ButtonTarget::None => "none".to_owned(),
            ButtonTarget::LeftTrigger => "left_trigger".to_owned(),
            ButtonTarget::RightTrigger => "right_trigger".to_owned(),
            ButtonTarget::Button(b) => b.name().to_owned(),
        }
    }
}

/// Turbo: enquanto o botão é segurado, ele liga e desliga `hz` vezes por segundo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Turbo {
    /// Botão (da entrada bruta).
    pub button: Button,
    /// Pressionamentos por segundo (1–30).
    pub hz: f32,
}

/// Remapeamento e turbo.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct ButtonsConfig {
    /// Botões que não vão para si mesmos. Os ausentes são identidade.
    pub remap: BTreeMap<Button, ButtonTarget>,
    /// Botões com turbo.
    pub turbo: Vec<Turbo>,
}

// ---------------------------------------------------------------------- rumble

/// Vibração: o que o jogo pede chega ao celular com esta intensidade.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct RumbleConfig {
    /// Repassa a vibração ao celular. Desligada, os motores ficam parados.
    pub enabled: bool,
    /// Multiplicador do que o jogo pede, de 0 a 2 (200%). Acima de 1,0 satura em 255.
    pub intensity: f32,
}

impl Default for RumbleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 1.0,
        }
    }
}

// ---------------------------------------------------------------------- perfil

/// Perfil de mapeamento completo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Profile {
    /// Versão do formato.
    pub schema_version: u32,
    /// Nome exibido.
    pub name: String,
    /// Analógicos.
    pub sticks: SticksConfig,
    /// Gatilhos.
    pub triggers: TriggersConfig,
    /// Gyro.
    pub gyro: GyroConfig,
    /// Botões.
    pub buttons: ButtonsConfig,
    /// Vibração.
    pub rumble: RumbleConfig,
}

impl Default for Profile {
    fn default() -> Self {
        Self::identity()
    }
}

impl Profile {
    /// Perfil que não altera nada: a saída é igual à entrada (sem gyro).
    pub fn identity() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: "Identidade".to_owned(),
            sticks: SticksConfig::default(),
            triggers: TriggersConfig::default(),
            gyro: GyroConfig::default(),
            buttons: ButtonsConfig::default(),
            rumble: RumbleConfig::default(),
        }
    }

    /// Perfil "Padrão" embutido: identidade com deadzone de 8% nos analógicos.
    pub fn standard() -> Self {
        let stick = StickConfig {
            deadzone_inner: 0.08,
            ..StickConfig::default()
        };
        Self {
            name: "Padrão".to_owned(),
            sticks: SticksConfig {
                left: stick,
                right: stick,
                swap: false,
            },
            ..Self::identity()
        }
    }

    /// Lê um perfil de JSON e o valida.
    pub fn from_json(json: &str) -> Result<Self, ProfileError> {
        let profile: Self =
            serde_json::from_str(json).map_err(|e| ProfileError::Json(e.to_string()))?;
        profile.validate()?;
        Ok(profile)
    }

    /// Serializa o perfil em JSON legível.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("perfil sempre serializa")
    }

    /// Confere versão e faixas de todos os campos.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ProfileError::UnsupportedVersion(self.schema_version));
        }
        validate_stick("sticks.left", &self.sticks.left)?;
        validate_stick("sticks.right", &self.sticks.right)?;
        validate_trigger("triggers.left", &self.triggers.left)?;
        validate_trigger("triggers.right", &self.triggers.right)?;
        validate_gyro(&self.gyro)?;
        check(
            "rumble.intensity",
            self.rumble.intensity.is_finite() && (0.0..=2.0).contains(&self.rumble.intensity),
            "deve estar entre 0 e 2",
        )?;
        for turbo in &self.buttons.turbo {
            check(
                "buttons.turbo.hz",
                turbo.hz.is_finite() && (1.0..=30.0).contains(&turbo.hz),
                "deve estar entre 1 e 30",
            )?;
        }
        Ok(())
    }
}

fn check(field: &str, ok: bool, reason: &'static str) -> Result<(), ProfileError> {
    if ok {
        Ok(())
    } else {
        Err(ProfileError::Invalid {
            field: field.to_owned(),
            reason,
        })
    }
}

fn unit(v: f32) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}

fn validate_stick(path: &str, s: &StickConfig) -> Result<(), ProfileError> {
    check(
        &format!("{path}.deadzone_inner"),
        unit(s.deadzone_inner),
        "deve estar entre 0 e 1",
    )?;
    check(
        &format!("{path}.deadzone_outer"),
        unit(s.deadzone_outer),
        "deve estar entre 0 e 1",
    )?;
    check(
        &format!("{path}.deadzone_inner"),
        s.deadzone_inner + s.deadzone_outer < 0.95,
        "inner + outer deve ficar abaixo de 0,95",
    )?;
    check(
        &format!("{path}.anti_deadzone"),
        unit(s.anti_deadzone) && s.anti_deadzone < 0.9,
        "deve estar entre 0 e 0,9",
    )?;
    check(
        &format!("{path}.sensitivity"),
        s.sensitivity.is_finite() && (0.1..=4.0).contains(&s.sensitivity),
        "deve estar entre 0,1 e 4",
    )?;
    match s.curve {
        Curve::Linear => Ok(()),
        Curve::Exponential { exponent } => check(
            &format!("{path}.curve.exponent"),
            exponent.is_finite() && (0.25..=4.0).contains(&exponent),
            "deve estar entre 0,25 e 4",
        ),
        Curve::Custom { points } => {
            let mut last_x = 0.0;
            for (x, y) in points {
                check(
                    &format!("{path}.curve.points"),
                    x.is_finite() && x > last_x && x < 1.0 && unit(y),
                    "x deve crescer estritamente dentro de (0,1) e y ficar entre 0 e 1",
                )?;
                last_x = x;
            }
            Ok(())
        }
    }
}

fn validate_trigger(path: &str, t: &TriggerConfig) -> Result<(), ProfileError> {
    check(
        &format!("{path}.deadzone"),
        unit(t.deadzone),
        "deve estar entre 0 e 1",
    )?;
    check(
        &format!("{path}.full_point"),
        unit(t.full_point) && t.full_point > t.deadzone,
        "deve estar entre 0 e 1 e acima da deadzone",
    )
}

fn validate_gyro(g: &GyroConfig) -> Result<(), ProfileError> {
    for (name, v) in [
        ("sensitivity_x", g.sensitivity_x),
        ("sensitivity_y", g.sensitivity_y),
    ] {
        check(
            &format!("gyro.{name}"),
            v.is_finite() && (0.0..=10.0).contains(&v),
            "deve estar entre 0 e 10",
        )?;
    }
    check(
        "gyro.deadzone",
        g.deadzone.is_finite() && (0.0..=1.0).contains(&g.deadzone),
        "deve estar entre 0 e 1 rad/s",
    )?;
    check(
        "gyro.smoothing.min_cutoff",
        g.smoothing.min_cutoff.is_finite() && (0.01..=30.0).contains(&g.smoothing.min_cutoff),
        "deve estar entre 0,01 e 30 Hz",
    )?;
    check(
        "gyro.smoothing.beta",
        g.smoothing.beta.is_finite() && (0.0..=10.0).contains(&g.smoothing.beta),
        "deve estar entre 0 e 10",
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // comparações exatas são intencionais nestes testes
mod tests {
    use super::*;

    #[test]
    fn built_in_profiles_are_valid() {
        Profile::identity().validate().unwrap();
        Profile::standard().validate().unwrap();
        assert_eq!(Profile::standard().sticks.left.deadzone_inner, 0.08);
        assert_eq!(Profile::standard().triggers, TriggersConfig::default());
    }

    #[test]
    fn json_roundtrip_keeps_everything() {
        let mut profile = Profile::standard();
        profile.gyro.enabled = true;
        profile.gyro.mode = GyroMode::Toggle {
            button: Button::LeftThumb,
        };
        profile.gyro.recenter_button = Some(Button::Back);
        profile.sticks.right.curve = Curve::Custom {
            points: [(0.2, 0.1), (0.4, 0.3), (0.6, 0.6), (0.8, 0.85)],
        };
        profile
            .buttons
            .remap
            .insert(Button::A, ButtonTarget::Button(Button::B));
        profile.buttons.remap.insert(Button::X, ButtonTarget::None);
        profile
            .buttons
            .remap
            .insert(Button::Y, ButtonTarget::RightTrigger);
        profile.buttons.turbo.push(Turbo {
            button: Button::A,
            hz: 12.0,
        });
        let again = Profile::from_json(&profile.to_json()).unwrap();
        assert_eq!(again, profile);
    }

    #[test]
    fn partial_json_fills_in_defaults() {
        let profile = Profile::from_json(
            r#"{ "name": "Só o gyro", "gyro": { "enabled": true }, "buttons": { "remap": { "a": "b" } } }"#,
        )
        .unwrap();
        assert!(profile.gyro.enabled);
        assert_eq!(profile.gyro.sensitivity_x, 1.0);
        assert_eq!(profile.sticks, SticksConfig::default());
        assert_eq!(
            profile.buttons.remap.get(&Button::A),
            Some(&ButtonTarget::Button(Button::B))
        );
    }

    #[test]
    fn rejects_bad_json_versions_and_targets() {
        assert!(matches!(
            Profile::from_json("{"),
            Err(ProfileError::Json(_))
        ));
        assert_eq!(
            Profile::from_json(r#"{ "schema_version": 2 }"#),
            Err(ProfileError::UnsupportedVersion(2))
        );
        assert!(matches!(
            Profile::from_json(r#"{ "buttons": { "remap": { "a": "banana" } } }"#),
            Err(ProfileError::Json(_))
        ));
        assert!(matches!(
            Profile::from_json(r#"{ "buttons": { "remap": { "banana": "a" } } }"#),
            Err(ProfileError::Json(_))
        ));
    }

    fn invalid(mutate: impl FnOnce(&mut Profile)) -> String {
        let mut profile = Profile::identity();
        mutate(&mut profile);
        match profile.validate() {
            Err(ProfileError::Invalid { field, .. }) => field,
            other => panic!("esperava Invalid, veio {other:?}"),
        }
    }

    #[test]
    fn out_of_range_fields_are_rejected_with_their_path() {
        assert_eq!(
            invalid(|p| p.sticks.left.deadzone_inner = 1.5),
            "sticks.left.deadzone_inner"
        );
        assert_eq!(
            invalid(|p| p.sticks.right.deadzone_outer = -0.1),
            "sticks.right.deadzone_outer"
        );
        assert_eq!(
            invalid(|p| {
                p.sticks.left.deadzone_inner = 0.5;
                p.sticks.left.deadzone_outer = 0.5;
            }),
            "sticks.left.deadzone_inner"
        );
        assert_eq!(
            invalid(|p| p.sticks.left.anti_deadzone = 0.95),
            "sticks.left.anti_deadzone"
        );
        assert_eq!(
            invalid(|p| p.sticks.left.sensitivity = 0.0),
            "sticks.left.sensitivity"
        );
        assert_eq!(
            invalid(|p| p.sticks.left.sensitivity = f32::NAN),
            "sticks.left.sensitivity"
        );
        assert_eq!(
            invalid(|p| p.sticks.left.curve = Curve::Exponential { exponent: 9.0 }),
            "sticks.left.curve.exponent"
        );
        assert_eq!(
            invalid(|p| p.sticks.left.curve = Curve::Custom {
                points: [(0.5, 0.5), (0.4, 0.6), (0.6, 0.7), (0.8, 0.9)],
            }),
            "sticks.left.curve.points"
        );
        assert_eq!(
            invalid(|p| p.sticks.left.curve = Curve::Custom {
                points: [(0.2, 2.0), (0.4, 0.6), (0.6, 0.7), (0.8, 0.9)],
            }),
            "sticks.left.curve.points"
        );
        assert_eq!(
            invalid(|p| p.triggers.left.deadzone = 2.0),
            "triggers.left.deadzone"
        );
        assert_eq!(
            invalid(|p| {
                p.triggers.right.deadzone = 0.5;
                p.triggers.right.full_point = 0.4;
            }),
            "triggers.right.full_point"
        );
        assert_eq!(
            invalid(|p| p.gyro.sensitivity_x = 11.0),
            "gyro.sensitivity_x"
        );
        assert_eq!(
            invalid(|p| p.gyro.sensitivity_y = -1.0),
            "gyro.sensitivity_y"
        );
        assert_eq!(invalid(|p| p.gyro.deadzone = 5.0), "gyro.deadzone");
        assert_eq!(
            invalid(|p| p.gyro.smoothing.min_cutoff = 0.0),
            "gyro.smoothing.min_cutoff"
        );
        assert_eq!(
            invalid(|p| p.gyro.smoothing.beta = 11.0),
            "gyro.smoothing.beta"
        );
        assert_eq!(
            invalid(|p| p.buttons.turbo.push(Turbo {
                button: Button::A,
                hz: 100.0
            })),
            "buttons.turbo.hz"
        );
    }
}
