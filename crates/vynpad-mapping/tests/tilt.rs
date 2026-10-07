//! Gyro por ângulo de inclinação (volante e jogos de corrida): o analógico acompanha o quanto o
//! aparelho está inclinado e fica onde ele está, em vez de voltar ao centro ao parar de girar.

use vynpad_driver::xbuttons;
use vynpad_mapping::{
    Accel, CompiledProfile, Gyro, GyroAxis, GyroResponse, MapperState, Profile, RawInput,
    StickSide, apply,
};

const DT: f32 = 0.004;
const G: f32 = 9.806_65;

/// Gravidade medida com o aparelho girado `deg` graus **anti-horários** (volante virado à esquerda).
fn roll(deg: f32) -> Accel {
    let t = deg.to_radians();
    Accel {
        x: G * t.sin(),
        y: G * t.cos(),
        z: 0.0,
    }
}

fn frame(accel: Accel, gyro: Gyro) -> RawInput {
    RawInput {
        gyro: Some(gyro),
        accel: Some(accel),
        ..RawInput::default()
    }
}

fn still(accel: Accel) -> RawInput {
    frame(accel, Gyro::default())
}

/// Perfil de volante: inclinação em torno de Z no analógico esquerdo X, sem nada no Y.
fn wheel() -> Profile {
    let mut p = Profile::identity();
    p.gyro.enabled = true;
    p.gyro.response = GyroResponse::Tilt;
    p.gyro.target = StickSide::Left;
    p.gyro.axis_x = GyroAxis::Roll;
    p.gyro.sensitivity_y = 0.0;
    p.gyro.deadzone = 0.0;
    p.gyro.smoothing.enabled = false;
    p.gyro.tilt_range_deg = 45.0;
    p
}

/// Gira o aparelho de `from` a `to` graus (anti-horários) em `secs`, como o sensor real: o
/// giroscópio informa a velocidade e a gravidade acompanha a pose. Devolve o `lx` no fim.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // contagens pequenas de quadros
fn turn(compiled: &CompiledProfile, state: &mut MapperState, from: f32, to: f32, secs: f32) -> i16 {
    let steps = (secs / DT).round() as usize;
    let rate = (to - from).to_radians() / secs;
    let mut lx = 0;
    for i in 1..=steps {
        let deg = from + (to - from) * (i as f32 / steps as f32);
        lx = apply(
            compiled,
            &frame(
                roll(deg),
                Gyro {
                    x: 0.0,
                    y: 0.0,
                    z: rate,
                },
            ),
            state,
            DT,
        )
        .lx;
    }
    lx
}

fn lx_after(profile: &Profile, raw: &RawInput, frames: usize) -> i16 {
    let compiled = CompiledProfile::compile(profile).unwrap();
    let mut state = MapperState::new();
    let mut out = apply(&compiled, raw, &mut state, DT);
    for _ in 1..frames {
        out = apply(&compiled, raw, &mut state, DT);
    }
    out.lx
}

fn near(a: i16, b: i16, tol: i16) -> bool {
    (i32::from(a) - i32::from(b)).abs() <= i32::from(tol)
}

#[test]
fn volante_virado_a_direita_vai_para_a_direita_e_ao_fim_do_curso_em_45_graus() {
    // Horário (direita) = ângulo anti-horário negativo.
    assert_eq!(lx_after(&wheel(), &still(roll(-45.0)), 1), 32767);
    assert!(near(lx_after(&wheel(), &still(roll(-22.5)), 1), 16384, 300));
    assert!(near(lx_after(&wheel(), &still(roll(22.5)), 1), -16384, 300));
}

#[test]
fn o_analogico_fica_onde_o_aparelho_esta_sem_voltar_ao_centro() {
    // Era o defeito do modo por velocidade: parado, a saída caía a zero.
    let held = lx_after(&wheel(), &still(roll(-20.0)), 1000);
    assert!(near(held, 14_563, 400), "{held}");
}

#[test]
fn passar_do_fim_do_curso_trava_no_maximo() {
    assert_eq!(lx_after(&wheel(), &still(roll(-80.0)), 1), 32767);
    assert_eq!(lx_after(&wheel(), &still(roll(80.0)), 1), -32768);
    let mut p = wheel();
    p.gyro.sensitivity_x = 2.0;
    assert_eq!(lx_after(&p, &still(roll(-30.0)), 1), 32767);
}

#[test]
fn o_angulo_de_fim_de_curso_e_configuravel() {
    let mut p = wheel();
    p.gyro.tilt_range_deg = 90.0;
    assert!(near(lx_after(&p, &still(roll(-45.0)), 1), 16384, 300));
}

#[test]
fn inverter_troca_o_sentido() {
    let mut p = wheel();
    p.gyro.invert_x = true;
    assert!(lx_after(&p, &still(roll(-30.0)), 1) < 0);
}

#[test]
fn deadzone_ignora_o_tremor_perto_do_centro() {
    let mut p = wheel();
    p.gyro.deadzone = 0.05;
    assert_eq!(lx_after(&p, &still(roll(-1.0)), 1), 0);
    assert!(lx_after(&p, &still(roll(-20.0)), 1) > 10_000);
}

#[test]
fn recentralizar_faz_da_posicao_atual_o_neutro() {
    let mut p = wheel();
    p.gyro.recenter_button = Some(vynpad_mapping::Button::Back);
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    // Aparelho inclinado 30° para a direita desde o início.
    let pose = roll(-30.0);
    assert!(apply(&compiled, &still(pose), &mut state, DT).lx > 20_000);
    // Aperta recentralizar nessa posição: vira o neutro.
    let mut pressed = still(pose);
    pressed.buttons = xbuttons::BACK;
    apply(&compiled, &pressed, &mut state, DT);
    let out = apply(&compiled, &still(pose), &mut state, DT);
    assert!(out.lx.abs() < 300, "{}", out.lx);
    // Mais 15° para a direita a partir do novo neutro = 1/3 do curso.
    let more = turn(&compiled, &mut state, -30.0, -45.0, 0.25);
    assert!(near(more, 10_922, 600), "{more}");
}

#[test]
fn solavanco_nao_arrasta_o_angulo_so_o_giroscopio_conduz() {
    let compiled = CompiledProfile::compile(&wheel()).unwrap();
    let mut state = MapperState::new();
    apply(&compiled, &still(roll(0.0)), &mut state, DT);
    // Aceleração absurda (carro sacudindo): o acelerômetro é ignorado e o gyro integra.
    let shake = Accel {
        x: 40.0,
        y: -35.0,
        z: 20.0,
    };
    let turning = Gyro {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    }; // 1 rad/s horário
    let mut lx = 0;
    for _ in 0..125 {
        // 0,5 s
        lx = apply(&compiled, &frame(shake, turning), &mut state, DT).lx;
    }
    // ≈ 0,5 rad de 0,785 rad (45°) = ~64 % do curso.
    assert!(near(lx, 20_900, 1500), "{lx}");
}

#[test]
fn deriva_do_giroscopio_e_corrigida_pela_gravidade() {
    let compiled = CompiledProfile::compile(&wheel()).unwrap();
    let mut state = MapperState::new();
    // Parado na vertical, mas o giroscópio acusa 0,05 rad/s de viés durante 10 s.
    let biased = frame(
        roll(0.0),
        Gyro {
            x: 0.0,
            y: 0.0,
            z: 0.05,
        },
    );
    let mut lx = 0;
    for _ in 0..2500 {
        lx = apply(&compiled, &biased, &mut state, DT).lx;
    }
    assert!(lx.abs() < 1500, "a deriva ficou em {lx}");
}

#[test]
fn aparelho_deitado_nao_tem_angulo_de_volante_e_nao_gera_ruido() {
    // Tela para cima, sem componente no plano X/Y: o ângulo de roll não existe.
    let flat = Accel {
        x: 0.05,
        y: -0.04,
        z: G,
    };
    assert_eq!(lx_after(&wheel(), &still(flat), 200), 0);
}

#[test]
fn sem_acelerometro_no_pacote_o_modo_fica_quieto() {
    let raw = RawInput {
        gyro: Some(Gyro::default()),
        accel: None,
        ..RawInput::default()
    };
    assert_eq!(lx_after(&wheel(), &raw, 5), 0);
}

#[test]
fn inclinar_para_os_lados_com_o_aparelho_deitado_tambem_funciona() {
    // Yaw: o aparelho deitado (tela para cima) com o lado direito para baixo vai para a direita.
    let mut p = wheel();
    p.gyro.axis_x = GyroAxis::Yaw;
    let t = 20.0_f32.to_radians();
    let right_down = Accel {
        x: -G * t.sin(),
        y: 0.0,
        z: G * t.cos(),
    };
    assert!(near(lx_after(&p, &still(right_down), 1), 14_563, 400));
    let left_down = Accel {
        x: G * t.sin(),
        y: 0.0,
        z: G * t.cos(),
    };
    assert!(near(lx_after(&p, &still(left_down), 1), -14_563, 400));
}

#[test]
fn o_modo_por_velocidade_continua_igual() {
    let mut p = wheel();
    p.gyro.response = GyroResponse::Rate;
    let raw = frame(
        roll(-40.0),
        Gyro {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
    );
    // 1 rad/s horário em `rate` = metade do curso; a inclinação não interfere.
    assert!(near(lx_after(&p, &raw, 1), 16384, 300));
    assert_eq!(lx_after(&p, &still(roll(-40.0)), 1), 0);
}

#[test]
fn faixa_de_angulo_invalida_e_recusada() {
    let mut p = wheel();
    p.gyro.tilt_range_deg = 5.0;
    assert!(p.validate().is_err());
    p.gyro.tilt_range_deg = f32::NAN;
    assert!(p.validate().is_err());
    p.gyro.tilt_range_deg = 45.0;
    assert!(p.validate().is_ok());
}
