//! Comportamento do pipeline, etapa por etapa, pela API pública.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use padgaming_driver::{XReport, xbuttons};
use padgaming_mapping::{
    Button, ButtonTarget, CompiledProfile, Curve, DeadzoneShape, Gyro, GyroAxis, GyroMode, Mapper,
    MapperState, Profile, ProfileCell, RawInput, StickSide, Turbo, apply,
};

const DT: f32 = 0.004;

fn run(profile: &Profile, raw: &RawInput) -> XReport {
    let compiled = CompiledProfile::compile(profile).unwrap();
    apply(&compiled, raw, &mut MapperState::new(), DT)
}

fn left(x: i16, y: i16) -> RawInput {
    RawInput {
        lx: x,
        ly: y,
        ..RawInput::default()
    }
}

fn with_buttons(buttons: u16) -> RawInput {
    RawInput {
        buttons,
        ..RawInput::default()
    }
}

fn gyro(x: f32, y: f32, z: f32) -> RawInput {
    RawInput {
        gyro: Some(Gyro { x, y, z }),
        ..RawInput::default()
    }
}

fn gyro_profile() -> Profile {
    let mut p = Profile::identity();
    p.gyro.enabled = true;
    p.gyro.deadzone = 0.0;
    p.gyro.smoothing.enabled = false;
    p
}

/// `a` e `b` diferem em no máximo `tol` unidades.
fn close(a: i16, b: i16, tol: i16) -> bool {
    (i32::from(a) - i32::from(b)).abs() <= i32::from(tol)
}

// ---------------------------------------------------------------- identidade e padrão

#[test]
fn identity_profile_returns_the_input_untouched() {
    let raw = RawInput {
        buttons: xbuttons::A | xbuttons::DPAD_UP | xbuttons::GUIDE,
        lt: 17,
        rt: 255,
        lx: -32768,
        ly: 32767,
        rx: 1234,
        ry: -4321,
        gyro: None,
    };
    let out = run(&Profile::identity(), &raw);
    assert_eq!(out.buttons, raw.buttons);
    assert_eq!((out.lt, out.rt), (17, 255));
    assert_eq!(
        (out.lx, out.ly, out.rx, out.ry),
        (-32768, 32767, 1234, -4321)
    );
}

#[test]
fn reserved_button_bit_is_dropped() {
    let out = run(&Profile::identity(), &with_buttons(0x0800 | xbuttons::A));
    assert_eq!(out.buttons, xbuttons::A);
}

#[test]
fn standard_profile_has_an_8_percent_deadzone() {
    let p = Profile::standard();
    // 7% do curso: dentro da deadzone.
    assert_eq!(run(&p, &left(2293, 0)).lx, 0);
    // No fim do curso continua no fim do curso.
    assert_eq!(run(&p, &left(32767, 0)).lx, 32767);
    // Logo depois da deadzone a saída é pequena, mas não zero.
    let just_out = run(&p, &left(2700, 0)).lx;
    assert!(just_out > 0 && just_out < 400, "{just_out}");
}

// ---------------------------------------------------------------- analógicos

#[test]
fn radial_deadzone_zeroes_the_whole_vector_and_keeps_direction() {
    let mut p = Profile::identity();
    p.sticks.left.deadzone_inner = 0.2;
    // 0.14 e 0.14: cada eixo é menor que 0.2, mas a distância (0.198) ainda cabe na deadzone.
    assert_eq!(run(&p, &left(4587, 4587)), XReport::default());
    // Fora da deadzone a direção diagonal é preservada.
    let out = run(&p, &left(20000, 20000));
    assert!(close(out.lx, out.ly, 1));
    assert!(out.lx > 0);
}

#[test]
fn axial_deadzone_acts_on_each_axis_separately() {
    let mut p = Profile::identity();
    p.sticks.left.deadzone_inner = 0.2;
    p.sticks.left.deadzone_shape = DeadzoneShape::Axial;
    // X fora da deadzone, Y dentro: só o Y vai a zero.
    let out = run(&p, &left(20000, 4000));
    assert!(out.lx > 0);
    assert_eq!(out.ly, 0);
    // Sinais são preservados.
    let out = run(&p, &left(-20000, -20000));
    assert!(out.lx < 0 && out.ly < 0);
}

#[test]
fn anti_deadzone_starts_the_output_above_zero() {
    let mut p = Profile::identity();
    p.sticks.left.deadzone_inner = 0.1;
    p.sticks.left.anti_deadzone = 0.3;
    let out = run(&p, &left(3600, 0)); // logo acima de 10%
    assert!(
        out.lx > 9000,
        "saída {} deveria começar perto de 30%",
        out.lx
    );
    assert_eq!(run(&p, &left(32767, 0)).lx, 32767);
}

#[test]
fn outer_deadzone_reaches_full_scale_before_the_edge() {
    let mut p = Profile::identity();
    p.sticks.left.deadzone_outer = 0.2;
    // 80% do curso já é 100% de saída.
    assert!(run(&p, &left(26214, 0)).lx >= 32766);
    assert_eq!(run(&p, &left(32767, 0)).lx, 32767);
}

#[test]
fn exponential_curve_softens_the_center() {
    let mut p = Profile::identity();
    p.sticks.left.curve = Curve::Exponential { exponent: 2.0 };
    let half = run(&p, &left(16384, 0)).lx;
    // 0.5^2 = 0.25 do curso.
    assert!(close(half, 8192, 40), "{half}");
    assert_eq!(run(&p, &left(32767, 0)).lx, 32767);
}

#[test]
fn custom_curve_follows_its_points() {
    let mut p = Profile::identity();
    p.sticks.right.curve = Curve::Custom {
        points: [(0.25, 0.5), (0.5, 0.5), (0.75, 0.5), (0.9, 0.5)],
    };
    let raw = RawInput {
        rx: 16384,
        ..RawInput::default()
    };
    // Em x = 0.5 a curva vale 0,5.
    assert!(close(run(&p, &raw).rx, 16384, 40));
    let raw = RawInput {
        rx: 8192, // x = 0.25 -> 0.5
        ..RawInput::default()
    };
    assert!(close(run(&p, &raw).rx, 16384, 40));
}

#[test]
fn sensitivity_scales_and_the_output_is_clamped() {
    let mut p = Profile::identity();
    p.sticks.left.sensitivity = 2.0;
    assert!(close(run(&p, &left(8192, 0)).lx, 16384, 2));
    // Passou do fim do curso: satura em vez de estourar.
    assert_eq!(run(&p, &left(30000, 0)).lx, 32767);
    assert_eq!(run(&p, &left(-30000, 0)).lx, -32768);
    p.sticks.left.sensitivity = 0.5;
    assert!(close(run(&p, &left(16384, 0)).lx, 8192, 2));
}

#[test]
fn invert_flips_each_axis() {
    let mut p = Profile::identity();
    p.sticks.left.invert_x = true;
    let out = run(&p, &left(1000, 2000));
    assert_eq!((out.lx, out.ly), (-1000, 2000));
    p.sticks.left.invert_x = false;
    p.sticks.left.invert_y = true;
    let out = run(&p, &left(1000, 2000));
    assert_eq!((out.lx, out.ly), (1000, -2000));
}

#[test]
fn swap_exchanges_sticks_before_their_own_settings() {
    let mut p = Profile::identity();
    p.sticks.swap = true;
    p.sticks.left.invert_x = true; // configuração do analógico esquerdo de saída
    let raw = RawInput {
        lx: 1000,
        ly: 2000,
        rx: 3000,
        ry: 4000,
        ..RawInput::default()
    };
    let out = run(&p, &raw);
    // O direito bruto foi para a esquerda e sofreu a inversão do esquerdo.
    assert_eq!((out.lx, out.ly), (-3000, 4000));
    assert_eq!((out.rx, out.ry), (1000, 2000));
}

// ---------------------------------------------------------------- gatilhos

#[test]
fn trigger_deadzone_and_full_point() {
    let mut p = Profile::identity();
    p.triggers.left.deadzone = 0.2;
    p.triggers.left.full_point = 0.8;
    let lt = |v: u8| {
        run(
            &p,
            &RawInput {
                lt: v,
                ..RawInput::default()
            },
        )
        .lt
    };
    assert_eq!(lt(50), 0); // 19,6% < 20%
    assert_eq!(lt(255), 255);
    assert_eq!(lt(210), 255); // 82% >= ponto de acionamento total
    let mid = lt(128); // 50%: metade entre 20% e 80%
    assert!((i32::from(mid) - 127).abs() <= 2, "{mid}");
    // O gatilho direito não foi alterado.
    let out = run(
        &p,
        &RawInput {
            rt: 100,
            ..RawInput::default()
        },
    );
    assert_eq!(out.rt, 100);
}

#[test]
fn digital_trigger_is_all_or_nothing() {
    let mut p = Profile::identity();
    p.triggers.right.digital = true;
    p.triggers.right.full_point = 0.5;
    let rt = |v: u8| {
        run(
            &p,
            &RawInput {
                rt: v,
                ..RawInput::default()
            },
        )
        .rt
    };
    assert_eq!(rt(127), 0);
    assert_eq!(rt(128), 255);
    assert_eq!(rt(255), 255);
}

// ---------------------------------------------------------------- botões

#[test]
fn buttons_can_be_remapped_dropped_or_sent_to_a_trigger() {
    let mut p = Profile::identity();
    p.buttons
        .remap
        .insert(Button::A, ButtonTarget::Button(Button::B));
    p.buttons.remap.insert(Button::X, ButtonTarget::None);
    p.buttons
        .remap
        .insert(Button::Y, ButtonTarget::RightTrigger);
    p.buttons
        .remap
        .insert(Button::LeftShoulder, ButtonTarget::LeftTrigger);

    let out = run(&p, &with_buttons(xbuttons::A));
    assert_eq!(out.buttons, xbuttons::B);
    let out = run(&p, &with_buttons(xbuttons::X | xbuttons::START));
    assert_eq!(out.buttons, xbuttons::START);
    let out = run(&p, &with_buttons(xbuttons::Y));
    assert_eq!((out.buttons, out.rt), (0, 255));
    let out = run(&p, &with_buttons(xbuttons::LEFT_SHOULDER));
    assert_eq!((out.buttons, out.lt), (0, 255));
}

#[test]
fn two_buttons_to_the_same_target_are_merged() {
    let mut p = Profile::identity();
    p.buttons
        .remap
        .insert(Button::A, ButtonTarget::Button(Button::B));
    let out = run(&p, &with_buttons(xbuttons::A | xbuttons::B));
    assert_eq!(out.buttons, xbuttons::B);
}

#[test]
fn a_swap_of_two_buttons_works() {
    let mut p = Profile::identity();
    p.buttons
        .remap
        .insert(Button::A, ButtonTarget::Button(Button::B));
    p.buttons
        .remap
        .insert(Button::B, ButtonTarget::Button(Button::A));
    assert_eq!(run(&p, &with_buttons(xbuttons::A)).buttons, xbuttons::B);
    assert_eq!(run(&p, &with_buttons(xbuttons::B)).buttons, xbuttons::A);
}

#[test]
fn turbo_toggles_at_the_configured_rate_and_resets_on_release() {
    let mut p = Profile::identity();
    p.buttons.turbo.push(Turbo {
        button: Button::A,
        hz: 10.0,
    });
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    let held = with_buttons(xbuttons::A);

    // 10 Hz = período de 100 ms: 50 ms pressionado, 50 ms solto. Passo de 10 ms.
    let mut samples = Vec::new();
    for _ in 0..20 {
        samples.push(apply(&compiled, &held, &mut state, 0.010).buttons == xbuttons::A);
    }
    // Confere no meio de cada meio período (as bordas dependem do arredondamento do f32).
    assert_eq!(&samples[1..4], &[true; 3]);
    assert_eq!(&samples[6..9], &[false; 3]);
    assert_eq!(&samples[11..14], &[true; 3]);
    assert_eq!(&samples[16..19], &[false; 3]);
    assert!(samples[0], "começa pressionado");

    // Solta e aperta de novo: recomeça pressionado.
    apply(&compiled, &RawInput::default(), &mut state, 0.010);
    assert_eq!(
        apply(&compiled, &held, &mut state, 0.010).buttons,
        xbuttons::A
    );

    // Outros botões não são afetados pelo turbo.
    let out = apply(
        &compiled,
        &with_buttons(xbuttons::A | xbuttons::B),
        &mut MapperState::new(),
        0.0,
    );
    assert_eq!(out.buttons, xbuttons::A | xbuttons::B);
}

// ---------------------------------------------------------------- gyro

#[test]
fn gyro_is_off_by_default() {
    let out = run(&Profile::identity(), &gyro(0.0, 3.0, 0.0));
    assert_eq!(out, XReport::default());
}

#[test]
fn turning_left_moves_the_right_stick_left_and_tilting_up_moves_it_up() {
    let p = gyro_profile();
    // Rotação positiva em Y = virar para a esquerda (PROTOCOLO §6.4): stick X negativo.
    let out = run(&p, &gyro(0.0, 1.0, 0.0));
    assert!(out.rx < -10000 && out.ry == 0, "{out:?}");
    // Rotação negativa em X = topo do aparelho para longe do jogador = mirar para cima.
    let out = run(&p, &gyro(-1.0, 0.0, 0.0));
    assert!(out.ry > 10000 && out.rx == 0, "{out:?}");
    // O analógico esquerdo não é tocado.
    assert_eq!((out.lx, out.ly), (0, 0));
}

#[test]
fn gyro_sensitivity_invert_and_full_scale() {
    let mut p = gyro_profile();
    // 1,0 rad/s com sensibilidade 1,0 = metade do curso (fundo de escala = 2 rad/s).
    let out = run(&p, &gyro(0.0, -1.0, 0.0));
    assert!(close(out.rx, 16384, 2), "{}", out.rx);
    p.gyro.sensitivity_x = 2.0;
    assert_eq!(run(&p, &gyro(0.0, -1.0, 0.0)).rx, 32767);
    p.gyro.sensitivity_x = 1.0;
    p.gyro.invert_x = true;
    assert!(run(&p, &gyro(0.0, -1.0, 0.0)).rx < 0);
    p.gyro.sensitivity_x = 0.0;
    assert_eq!(run(&p, &gyro(0.0, -1.0, 0.0)).rx, 0);
}

#[test]
fn gyro_axes_and_target_are_configurable() {
    let mut p = gyro_profile();
    p.gyro.target = StickSide::Left;
    p.gyro.axis_x = GyroAxis::Roll;
    p.gyro.axis_y = GyroAxis::Yaw;
    // Roll: girar como volante para a direita (z negativo) = stick X positivo.
    let out = run(&p, &gyro(0.0, 0.0, -1.0));
    assert!(out.lx > 10000, "{out:?}");
    assert_eq!((out.rx, out.ry), (0, 0));
    // Yaw agora no eixo Y: virar para a esquerda (y positivo) = Y negativo.
    let out = run(&p, &gyro(0.0, 1.0, 0.0));
    assert!(out.ly < -10000, "{out:?}");
}

#[test]
fn gyro_deadzone_ignores_small_rotations_and_is_soft_at_the_edge() {
    let mut p = gyro_profile();
    p.gyro.deadzone = 0.5;
    assert_eq!(run(&p, &gyro(0.0, 0.4, 0.0)).rx, 0);
    // Logo acima da deadzone a saída começa perto de zero (sem degrau).
    let out = run(&p, &gyro(0.0, -0.6, 0.0)).rx;
    assert!(out > 0 && out < 3000, "{out}");
}

#[test]
fn gyro_mixes_with_the_stick_and_clamps_or_replaces_it() {
    let mut p = gyro_profile();
    let mut raw = gyro(0.0, -1.0, 0.0); // +16384 no X
    raw.rx = 8192;
    let out = run(&p, &raw);
    assert!(close(out.rx, 24576, 3), "soma: {}", out.rx);
    raw.rx = 30000;
    assert_eq!(run(&p, &raw).rx, 32767, "clamp");
    p.gyro.mix_with_stick = false;
    raw.rx = 8192;
    assert!(close(run(&p, &raw).rx, 16384, 2), "substitui o stick");
}

#[test]
fn gyro_while_held_only_acts_with_the_button_down() {
    let mut p = gyro_profile();
    p.gyro.mode = GyroMode::WhileHeld {
        button: Button::LeftShoulder,
    };
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    let mut raw = gyro(0.0, -1.0, 0.0);
    assert_eq!(apply(&compiled, &raw, &mut state, DT).rx, 0);
    raw.buttons = xbuttons::LEFT_SHOULDER;
    assert!(apply(&compiled, &raw, &mut state, DT).rx > 10000);
    raw.buttons = 0;
    assert_eq!(apply(&compiled, &raw, &mut state, DT).rx, 0);
}

#[test]
fn gyro_toggle_flips_on_each_press_not_while_held() {
    let mut p = gyro_profile();
    p.gyro.mode = GyroMode::Toggle {
        button: Button::RightThumb,
    };
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    let mut step = |buttons: u16| {
        let mut raw = gyro(0.0, -1.0, 0.0);
        raw.buttons = buttons;
        apply(&compiled, &raw, &mut state, DT).rx != 0
    };
    assert!(!step(0), "começa desligado");
    assert!(step(xbuttons::RIGHT_THUMB), "1º toque liga");
    assert!(step(xbuttons::RIGHT_THUMB), "segurar não alterna");
    assert!(step(0), "soltar mantém ligado");
    assert!(!step(xbuttons::RIGHT_THUMB), "2º toque desliga");
}

#[test]
fn recenter_button_turns_off_the_toggle_and_clears_the_filter() {
    let mut p = gyro_profile();
    p.gyro.mode = GyroMode::Toggle {
        button: Button::RightThumb,
    };
    p.gyro.recenter_button = Some(Button::Back);
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    let mut step = |buttons: u16| {
        let mut raw = gyro(0.0, -1.0, 0.0);
        raw.buttons = buttons;
        apply(&compiled, &raw, &mut state, DT).rx != 0
    };
    assert!(step(xbuttons::RIGHT_THUMB));
    assert!(step(0));
    assert!(
        !step(xbuttons::BACK),
        "recentralizar desliga o modo alternado"
    );
}

#[test]
fn smoothing_dampens_jitter_but_not_a_sustained_turn() {
    let mut p = gyro_profile();
    p.gyro.smoothing.enabled = true;
    p.gyro.smoothing.min_cutoff = 1.0;
    p.gyro.smoothing.beta = 0.0;
    let compiled = CompiledProfile::compile(&p).unwrap();

    let mut state = MapperState::new();
    let mut peak = 0i16;
    for i in 0..300 {
        let jitter = if i % 2 == 0 { 0.2 } else { -0.2 };
        let out = apply(&compiled, &gyro(0.0, jitter, 0.0), &mut state, DT);
        if i > 150 {
            peak = peak.max(out.rx.abs());
        }
    }
    assert!(
        peak < 1500,
        "o tremor deveria ficar amortecido, pico {peak}"
    );

    let mut state = MapperState::new();
    let mut last = 0;
    for _ in 0..1000 {
        last = apply(&compiled, &gyro(0.0, -1.0, 0.0), &mut state, DT).rx;
    }
    assert!(close(last, 16384, 200), "giro contínuo passa: {last}");
}

#[test]
fn packets_without_motion_release_the_gyro() {
    let p = gyro_profile();
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    assert!(apply(&compiled, &gyro(0.0, -1.0, 0.0), &mut state, DT).rx > 0);
    assert_eq!(apply(&compiled, &RawInput::default(), &mut state, DT).rx, 0);
}

// ---------------------------------------------------------------- dt

#[test]
fn hostile_dt_values_are_harmless() {
    let mut p = gyro_profile();
    p.gyro.smoothing.enabled = true;
    p.buttons.turbo.push(Turbo {
        button: Button::A,
        hz: 30.0,
    });
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    let mut raw = gyro(0.0, -1.0, 0.0);
    raw.buttons = xbuttons::A;
    for dt in [
        0.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e30,
        3600.0,
    ] {
        let out = apply(&compiled, &raw, &mut state, dt);
        assert!(out.rx >= 0);
    }
}

// ---------------------------------------------------------------- troca em tempo real

#[test]
fn profile_swap_takes_effect_on_the_next_frame() {
    let cell = Arc::new(ProfileCell::new(CompiledProfile::identity()));
    let mut mapper = Mapper::new(Arc::clone(&cell));
    let raw = left(1000, 0);
    assert_eq!(mapper.map(&raw, DT).lx, 1000);

    let mut inverted = Profile::identity();
    inverted.sticks.left.invert_x = true;
    cell.store(Arc::new(CompiledProfile::compile(&inverted).unwrap()));
    assert_eq!(mapper.map(&raw, DT).lx, -1000);
    assert_eq!(cell.load().name(), "Identidade");
}

#[test]
fn swapping_profiles_from_another_thread_never_blocks_or_corrupts_the_input_thread() {
    use std::sync::atomic::AtomicU64;

    let cell = Arc::new(ProfileCell::new(CompiledProfile::identity()));
    let stop = Arc::new(AtomicBool::new(false));
    let frames = Arc::new(AtomicU64::new(0));

    let input = {
        let (cell, stop, frames) = (Arc::clone(&cell), Arc::clone(&stop), Arc::clone(&frames));
        thread::spawn(move || {
            let mut mapper = Mapper::new(cell);
            let raw = left(1000, 0);
            while !stop.load(Ordering::Relaxed) {
                // Só existem dois perfis: o resultado é um dos dois valores, nunca outro.
                let lx = mapper.map(&raw, DT).lx;
                assert!(lx == 1000 || lx == -1000, "saída inesperada: {lx}");
                frames.fetch_add(1, Ordering::Relaxed);
            }
        })
    };

    let normal = Arc::new(CompiledProfile::identity());
    let mut inverted_profile = Profile::identity();
    inverted_profile.sticks.left.invert_x = true;
    let inverted = Arc::new(CompiledProfile::compile(&inverted_profile).unwrap());

    // Troca sem parar até a thread de input processar uma boa quantidade de quadros.
    let mut swaps = 0u32;
    while frames.load(Ordering::Relaxed) < 20_000 {
        cell.store(Arc::clone(if swaps.is_multiple_of(2) {
            &normal
        } else {
            &inverted
        }));
        swaps += 1;
        assert!(swaps < 50_000_000, "a thread de input travou");
    }
    stop.store(true, Ordering::Relaxed);
    input.join().expect("a thread de input não pode dar pânico");
    assert!(swaps > 0);
}
