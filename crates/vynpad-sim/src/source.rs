//! De onde vem o input simulado: padrões prontos, scripts TOML ou um controle físico.

use std::f32::consts::PI;
use std::time::Duration;

use anyhow::{Context, bail};
use serde::Deserialize;
use vynpad_protocol::{Buttons, InputState, Motion};

use crate::units::{Rng, parse_duration};

/// Um quadro de input: o estado e, se houver, o movimento.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    /// Estado dos botões e analógicos.
    pub state: InputState,
    /// Giroscópio e acelerômetro (o quadro vira um pacote `0x02`).
    pub motion: Option<Motion>,
}

/// Gera um quadro por tick.
pub trait InputSource {
    /// Quadro para o instante `elapsed` desde o início; `None` encerra a simulação.
    fn frame(&mut self, elapsed: Duration) -> Option<Frame>;
}

/// -1..1 → i16.
fn axis(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// 0..1 → u8.
fn trigger(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0) as u8
}

// ---------------------------------------------------------------------- padrões

/// Padrões prontos (`--pattern`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    /// Os analógicos desenham círculos e os gatilhos vão e vêm.
    Circle,
    /// Aperta um botão de cada vez, em sequência.
    Buttons,
    /// Estado aleatório completo a cada quadro (carga máxima).
    Stress,
}

impl std::str::FromStr for PatternKind {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        match name {
            "circle" => Ok(Self::Circle),
            "buttons" => Ok(Self::Buttons),
            "stress" => Ok(Self::Stress),
            other => Err(format!(
                "padrão `{other}` desconhecido (circle, buttons ou stress)"
            )),
        }
    }
}

/// Fonte de padrões prontos.
pub struct Pattern {
    kind: PatternKind,
    rng: Rng,
}

/// Quanto cada botão fica pressionado no padrão `buttons`.
const BUTTON_STEP: Duration = Duration::from_millis(300);

impl Pattern {
    /// Cria o padrão; `seed` só afeta o `stress`.
    pub const fn new(kind: PatternKind, seed: u64) -> Self {
        Self {
            kind,
            rng: Rng::new(seed),
        }
    }
}

impl InputSource for Pattern {
    fn frame(&mut self, elapsed: Duration) -> Option<Frame> {
        let t = elapsed.as_secs_f32();
        let state = match self.kind {
            PatternKind::Circle => {
                let angle = t * PI; // meia volta por segundo: um círculo a cada 2 s
                let lt = angle.sin().mul_add(0.5, 0.5);
                InputState {
                    lx: axis(angle.cos() * 0.8),
                    ly: axis(angle.sin() * 0.8),
                    rx: axis((-angle).cos() * 0.5),
                    ry: axis((-angle).sin() * 0.5),
                    lt: trigger(lt),
                    rt: trigger(1.0 - lt),
                    ..InputState::default()
                }
            }
            PatternKind::Buttons => {
                let all: Vec<Buttons> = Buttons::all().iter().collect();
                let step = (elapsed.as_millis() / BUTTON_STEP.as_millis()) as usize;
                InputState {
                    buttons: all[step % all.len()],
                    ..InputState::default()
                }
            }
            PatternKind::Stress => InputState {
                buttons: Buttons::from_wire(self.rng.next_u64() as u16),
                lt: self.rng.next_u64() as u8,
                rt: self.rng.next_u64() as u8,
                lx: axis(self.rng.signed()),
                ly: axis(self.rng.signed()),
                rx: axis(self.rng.signed()),
                ry: axis(self.rng.signed()),
                ..InputState::default()
            },
        };
        Some(Frame {
            state,
            motion: None,
        })
    }
}

// ---------------------------------------------------------------------- script TOML

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptFile {
    /// Recomeça do primeiro passo quando o último termina.
    #[serde(default, rename = "loop")]
    repeat: bool,
    #[serde(default)]
    step: Vec<StepFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepFile {
    duration: String,
    #[serde(default)]
    buttons: Vec<String>,
    #[serde(default)]
    lt: f32,
    #[serde(default)]
    rt: f32,
    #[serde(default)]
    lx: f32,
    #[serde(default)]
    ly: f32,
    #[serde(default)]
    rx: f32,
    #[serde(default)]
    ry: f32,
    /// Giroscópio em rad/s (`[x, y, z]`); liga o pacote com motion.
    gyro: Option<[f32; 3]>,
}

struct Step {
    end: Duration,
    frame: Frame,
}

/// Sequência de estados com tempos, lida de um arquivo TOML.
///
/// ```toml
/// loop = true
///
/// [[step]]
/// duration = "500ms"
/// buttons = ["a", "start"]
/// lx = 0.5          # analógicos de -1 a 1, gatilhos de 0 a 1
/// gyro = [0.0, -1.0, 0.0]   # rad/s
/// ```
pub struct Script {
    steps: Vec<Step>,
    total: Duration,
    repeat: bool,
}

fn button_by_name(name: &str) -> Option<Buttons> {
    Buttons::all()
        .iter_names()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, b)| b)
}

impl Script {
    /// Lê e valida o script.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let file: ScriptFile = toml::from_str(text).context("script TOML inválido")?;
        if file.step.is_empty() {
            bail!("o script não tem nenhum `[[step]]`");
        }
        let mut steps = Vec::with_capacity(file.step.len());
        let mut total = Duration::ZERO;
        for (i, step) in file.step.iter().enumerate() {
            let n = i + 1;
            let duration = parse_duration(&step.duration)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("passo {n}: `duration`"))?;
            if duration.is_zero() {
                bail!("passo {n}: `duration` deve ser maior que zero");
            }
            let mut buttons = Buttons::empty();
            for name in &step.buttons {
                buttons |= button_by_name(name)
                    .with_context(|| format!("passo {n}: botão `{name}` desconhecido"))?;
            }
            for (field, v, lo) in [
                ("lt", step.lt, 0.0),
                ("rt", step.rt, 0.0),
                ("lx", step.lx, -1.0),
                ("ly", step.ly, -1.0),
                ("rx", step.rx, -1.0),
                ("ry", step.ry, -1.0),
            ] {
                if !(lo..=1.0).contains(&v) {
                    bail!("passo {n}: `{field}` deve estar entre {lo} e 1 (veio {v})");
                }
            }
            let motion = step.gyro.map(|[gx, gy, gz]| Motion {
                gx: (gx * 1000.0) as i16,
                gy: (gy * 1000.0) as i16,
                gz: (gz * 1000.0) as i16,
                ..Motion::default()
            });
            total += duration;
            steps.push(Step {
                end: total,
                frame: Frame {
                    state: InputState {
                        buttons,
                        lt: trigger(step.lt),
                        rt: trigger(step.rt),
                        lx: axis(step.lx),
                        ly: axis(step.ly),
                        rx: axis(step.rx),
                        ry: axis(step.ry),
                        ..InputState::default()
                    },
                    motion,
                },
            });
        }
        Ok(Self {
            steps,
            total,
            repeat: file.repeat,
        })
    }

    /// Duração de uma volta do script.
    pub const fn duration(&self) -> Duration {
        self.total
    }
}

impl InputSource for Script {
    fn frame(&mut self, elapsed: Duration) -> Option<Frame> {
        let at = if self.repeat {
            Duration::from_nanos((elapsed.as_nanos() % self.total.as_nanos()) as u64)
        } else if elapsed >= self.total {
            return None;
        } else {
            elapsed
        };
        self.steps.iter().find(|s| at < s.end).map(|s| s.frame)
    }
}

// ---------------------------------------------------------------------- controle físico

/// Repassa um controle físico local (`--gamepad`), via `gilrs`.
#[cfg(feature = "gamepad")]
pub struct Gamepad {
    gilrs: gilrs::Gilrs,
}

#[cfg(feature = "gamepad")]
impl Gamepad {
    /// Abre a biblioteca de controles.
    pub fn new() -> anyhow::Result<Self> {
        let gilrs = gilrs::Gilrs::new().map_err(|e| anyhow::anyhow!("gilrs: {e}"))?;
        Ok(Self { gilrs })
    }

    /// Nomes dos controles conectados.
    pub fn connected(&self) -> Vec<String> {
        self.gilrs
            .gamepads()
            .map(|(_, g)| g.name().to_owned())
            .collect()
    }
}

#[cfg(feature = "gamepad")]
impl InputSource for Gamepad {
    fn frame(&mut self, _elapsed: Duration) -> Option<Frame> {
        use gilrs::{Axis, Button};

        // Processa os eventos pendentes para o estado ficar em dia.
        while self.gilrs.next_event().is_some() {}
        let Some((_, pad)) = self.gilrs.gamepads().next() else {
            return Some(Frame::default()); // nenhum controle: fica neutro
        };
        let pressed = |b| pad.is_pressed(b);
        let trigger_value = |b: Button, fallback: Axis| {
            pad.button_data(b)
                .map(gilrs::ev::state::ButtonData::value)
                .or_else(|| Some(pad.value(fallback).max(0.0)))
                .unwrap_or(0.0)
        };
        let mut buttons = Buttons::empty();
        for (source, target) in [
            (Button::DPadUp, Buttons::DPAD_UP),
            (Button::DPadDown, Buttons::DPAD_DOWN),
            (Button::DPadLeft, Buttons::DPAD_LEFT),
            (Button::DPadRight, Buttons::DPAD_RIGHT),
            (Button::Start, Buttons::START),
            (Button::Select, Buttons::BACK),
            (Button::LeftThumb, Buttons::LEFT_THUMB),
            (Button::RightThumb, Buttons::RIGHT_THUMB),
            (Button::LeftTrigger, Buttons::LEFT_SHOULDER),
            (Button::RightTrigger, Buttons::RIGHT_SHOULDER),
            (Button::Mode, Buttons::GUIDE),
            (Button::South, Buttons::A),
            (Button::East, Buttons::B),
            (Button::West, Buttons::X),
            (Button::North, Buttons::Y),
        ] {
            if pressed(source) {
                buttons |= target;
            }
        }
        Some(Frame {
            state: InputState {
                buttons,
                lt: trigger(trigger_value(Button::LeftTrigger2, Axis::LeftZ)),
                rt: trigger(trigger_value(Button::RightTrigger2, Axis::RightZ)),
                lx: axis(pad.value(Axis::LeftStickX)),
                ly: axis(pad.value(Axis::LeftStickY)),
                rx: axis(pad.value(Axis::RightStickX)),
                ry: axis(pad.value(Axis::RightStickY)),
                ..InputState::default()
            },
            motion: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(source: &mut impl InputSource, ms: u64) -> Frame {
        source.frame(Duration::from_millis(ms)).expect("quadro")
    }

    #[test]
    fn circle_moves_the_sticks_around_and_stays_in_range() {
        let mut p = Pattern::new(PatternKind::Circle, 1);
        let start = at(&mut p, 0).state;
        assert_eq!((start.lx, start.ly), (axis(0.8), 0));
        let quarter = at(&mut p, 500).state; // 90 graus
        assert!(quarter.lx.abs() < 40 && quarter.ly > 25_000, "{quarter:?}");
        let half = at(&mut p, 1000).state;
        assert!(half.lx < -25_000);
        // Os gatilhos são complementares.
        let s = at(&mut p, 300).state;
        assert!((i32::from(s.lt) + i32::from(s.rt) - 255).abs() <= 1);
    }

    #[test]
    fn buttons_pattern_presses_one_at_a_time_and_wraps() {
        let mut p = Pattern::new(PatternKind::Buttons, 1);
        let all: Vec<Buttons> = Buttons::all().iter().collect();
        assert_eq!(all.len(), 15);
        for (i, expected) in all.iter().enumerate() {
            let ms = i as u64 * 300 + 100;
            assert_eq!(at(&mut p, ms).state.buttons, *expected);
        }
        assert_eq!(at(&mut p, 15 * 300 + 10).state.buttons, all[0]);
    }

    #[test]
    fn stress_is_random_but_reproducible_and_valid() {
        let frames = |seed| {
            let mut p = Pattern::new(PatternKind::Stress, seed);
            (0..50).map(|i| at(&mut p, i * 4).state).collect::<Vec<_>>()
        };
        assert_eq!(frames(9), frames(9));
        assert_ne!(frames(9), frames(10));
        // O bit reservado nunca é ligado.
        assert!(frames(9).iter().all(|s| s.buttons.bits() & 0x0800 == 0));
    }

    #[test]
    fn pattern_names_parse() {
        assert_eq!("circle".parse(), Ok(PatternKind::Circle));
        assert_eq!("buttons".parse(), Ok(PatternKind::Buttons));
        assert_eq!("stress".parse(), Ok(PatternKind::Stress));
        assert!("zigzag".parse::<PatternKind>().is_err());
    }

    const SCRIPT: &str = r#"
        [[step]]
        duration = "500ms"
        buttons = ["a", "start"]
        lx = 0.5

        [[step]]
        duration = "1s"
        rt = 1.0
        gyro = [0.0, -1.0, 0.0]
    "#;

    #[test]
    fn script_steps_follow_their_durations() {
        let mut s = Script::parse(SCRIPT).unwrap();
        assert_eq!(s.duration(), Duration::from_millis(1500));
        let first = at(&mut s, 100);
        assert_eq!(first.state.buttons, Buttons::A | Buttons::START);
        assert_eq!(first.state.lx, axis(0.5));
        assert_eq!(first.motion, None);
        let second = at(&mut s, 500);
        assert_eq!(second.state.rt, 255);
        assert_eq!(second.state.buttons, Buttons::empty());
        assert_eq!(second.motion.unwrap().gy, -1000);
        // Sem `loop`, o fim do script encerra a simulação.
        assert_eq!(s.frame(Duration::from_millis(1500)), None);
    }

    #[test]
    fn looping_scripts_wrap_around() {
        let mut s = Script::parse(&format!("loop = true\n{SCRIPT}")).unwrap();
        assert_eq!(at(&mut s, 1600).state.buttons, Buttons::A | Buttons::START);
        assert_eq!(at(&mut s, 3000 + 700).state.rt, 255);
    }

    #[test]
    fn invalid_scripts_explain_what_is_wrong() {
        let err = |text: &str| format!("{:#}", Script::parse(text).err().expect("deveria falhar"));
        assert!(err("").contains("nenhum"));
        assert!(
            err("[[step]]\nduration = \"1s\"\nbuttons = [\"turbo\"]")
                .contains("passo 1: botão `turbo`")
        );
        assert!(err("[[step]]\nduration = \"0ms\"").contains("maior que zero"));
        assert!(err("[[step]]\nduration = \"1\"").contains("passo 1"));
        assert!(err("[[step]]\nduration = \"1s\"\nlx = 2.0").contains("`lx`"));
        assert!(err("[[step]]\nduration = \"1s\"\nlt = -0.1").contains("`lt`"));
        assert!(err("[[step]]\nduration = \"1s\"\nnope = 1").contains("inválido"));
        assert!(err("isto não é toml").contains("TOML"));
    }

    #[test]
    fn button_names_match_the_protocol_names() {
        assert_eq!(button_by_name("dpad_up"), Some(Buttons::DPAD_UP));
        assert_eq!(button_by_name("A"), Some(Buttons::A));
        assert_eq!(
            button_by_name("left_shoulder"),
            Some(Buttons::LEFT_SHOULDER)
        );
        assert_eq!(button_by_name("nope"), None);
    }
}
