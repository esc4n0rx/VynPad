//! Propriedades do pipeline, com entradas e perfis gerados aleatoriamente.

use proptest::prelude::*;
use vynpad_mapping::{
    Button, ButtonTarget, CompiledProfile, Curve, DeadzoneShape, Gyro, GyroAxis, GyroMode,
    MapperState, Profile, RawInput, StickConfig, StickSide, TriggerConfig, Turbo, apply,
};

fn button() -> impl Strategy<Value = Button> {
    (0..Button::ALL.len()).prop_map(|i| Button::ALL[i])
}

fn raw_input() -> impl Strategy<Value = RawInput> {
    (
        any::<u16>(),
        any::<u8>(),
        any::<u8>(),
        any::<i16>(),
        any::<i16>(),
        any::<i16>(),
        any::<i16>(),
        proptest::option::of((-40.0f32..40.0, -40.0f32..40.0, -40.0f32..40.0)),
    )
        .prop_map(|(buttons, lt, rt, lx, ly, rx, ry, gyro)| RawInput {
            buttons,
            lt,
            rt,
            lx,
            ly,
            rx,
            ry,
            gyro: gyro.map(|(x, y, z)| Gyro { x, y, z }),
        })
}

fn curve(monotone_only: bool) -> BoxedStrategy<Curve> {
    let linear = Just(Curve::Linear).boxed();
    let exp = (0.25f32..=4.0)
        .prop_map(|exponent| Curve::Exponential { exponent })
        .boxed();
    if monotone_only {
        return prop_oneof![linear, exp].boxed();
    }
    let custom = proptest::array::uniform4(0.0f32..=1.0)
        .prop_map(|y| Curve::Custom {
            points: [(0.2, y[0]), (0.4, y[1]), (0.6, y[2]), (0.8, y[3])],
        })
        .boxed();
    prop_oneof![linear, exp, custom].boxed()
}

fn stick(monotone_only: bool) -> impl Strategy<Value = StickConfig> {
    (
        (0.0f32..0.45, 0.0f32..0.45, 0.0f32..0.89),
        prop_oneof![Just(DeadzoneShape::Radial), Just(DeadzoneShape::Axial)],
        curve(monotone_only),
        0.1f32..=4.0,
        (any::<bool>(), any::<bool>()),
    )
        .prop_map(
            |((inner, outer, anti), shape, curve, sensitivity, (invert_x, invert_y))| StickConfig {
                deadzone_inner: inner,
                deadzone_outer: outer,
                deadzone_shape: shape,
                anti_deadzone: anti,
                curve,
                sensitivity,
                invert_x,
                invert_y,
            },
        )
}

fn trigger() -> impl Strategy<Value = TriggerConfig> {
    (0.0f32..0.5, 0.0f32..=1.0, any::<bool>()).prop_map(|(deadzone, t, digital)| TriggerConfig {
        deadzone,
        full_point: deadzone + (1.0 - deadzone) * (0.05 + 0.95 * t),
        digital,
    })
}

type GyroParts = (
    bool,
    GyroMode,
    StickSide,
    (f32, f32),
    (f32, bool),
    (f32, f32),
    Option<Button>,
    bool,
);

fn gyro_parts() -> impl Strategy<Value = GyroParts> {
    (
        any::<bool>(),
        prop_oneof![
            Just(GyroMode::Always),
            button().prop_map(|button| GyroMode::WhileHeld { button }),
            button().prop_map(|button| GyroMode::Toggle { button }),
        ],
        prop_oneof![Just(StickSide::Left), Just(StickSide::Right)],
        (0.0f32..=10.0, 0.0f32..=10.0),
        (0.0f32..=1.0, any::<bool>()),
        (0.01f32..=30.0, 0.0f32..=10.0),
        proptest::option::of(button()),
        any::<bool>(),
    )
}

fn profile(monotone_only: bool) -> impl Strategy<Value = Profile> {
    let buttons = (
        proptest::collection::btree_map(
            button(),
            prop_oneof![
                Just(ButtonTarget::None),
                Just(ButtonTarget::LeftTrigger),
                Just(ButtonTarget::RightTrigger),
                button().prop_map(ButtonTarget::Button),
            ],
            0..6,
        ),
        proptest::collection::vec((button(), 1.0f32..=30.0), 0..3),
    );
    (
        (stick(monotone_only), stick(monotone_only), any::<bool>()),
        (trigger(), trigger()),
        gyro_parts(),
        buttons,
    )
        .prop_map(|((left, right, swap), (tl, tr), gyro, (remap, turbo))| {
            let (enabled, mode, target, (sx, sy), (dz, inv), (cutoff, beta), recenter, mix) = gyro;
            let mut p = Profile::identity();
            p.sticks.left = left;
            p.sticks.right = right;
            p.sticks.swap = swap;
            p.triggers.left = tl;
            p.triggers.right = tr;
            p.gyro.enabled = enabled;
            p.gyro.mode = mode;
            p.gyro.target = target;
            p.gyro.axis_x = GyroAxis::Roll;
            p.gyro.sensitivity_x = sx;
            p.gyro.sensitivity_y = sy;
            p.gyro.deadzone = dz;
            p.gyro.invert_x = inv;
            p.gyro.smoothing.min_cutoff = cutoff;
            p.gyro.smoothing.beta = beta;
            p.gyro.recenter_button = recenter;
            p.gyro.mix_with_stick = mix;
            p.buttons.remap = remap;
            p.buttons.turbo = turbo
                .into_iter()
                .map(|(button, hz)| Turbo { button, hz })
                .collect();
            p
        })
}

proptest! {
    /// Perfil identidade = entrada inalterada (só o bit reservado dos botões é descartado).
    #[test]
    fn identity_profile_never_changes_the_input(raw in raw_input()) {
        let raw = RawInput { gyro: None, ..raw };
        let out = apply(&CompiledProfile::identity(), &raw, &mut MapperState::new(), 0.004);
        prop_assert_eq!(out.buttons, raw.buttons & Button::ALL_MASK);
        prop_assert_eq!((out.lt, out.rt), (raw.lt, raw.rt));
        prop_assert_eq!((out.lx, out.ly, out.rx, out.ry), (raw.lx, raw.ly, raw.rx, raw.ry));
    }

    /// Qualquer perfil válido, qualquer entrada e qualquer `dt`: nunca dá pânico, e a
    /// saída é determinística.
    #[test]
    fn any_valid_profile_is_total_and_deterministic(
        p in profile(false),
        frames in proptest::collection::vec(raw_input(), 1..12),
        dt in prop_oneof![Just(0.0f32), Just(f32::NAN), Just(-3.0f32), Just(1e9f32), 0.0f32..0.1],
    ) {
        prop_assert!(p.validate().is_ok());
        let compiled = CompiledProfile::compile(&p).unwrap();
        let (mut a, mut b) = (MapperState::new(), MapperState::new());
        for raw in &frames {
            prop_assert_eq!(apply(&compiled, raw, &mut a, dt), apply(&compiled, raw, &mut b, dt));
        }
    }

    /// Entrada neutra (sem gyro) resulta em saída neutra, com qualquer perfil.
    #[test]
    fn neutral_input_gives_neutral_output(p in profile(false)) {
        let compiled = CompiledProfile::compile(&p).unwrap();
        let out = apply(&compiled, &RawInput::default(), &mut MapperState::new(), 0.004);
        prop_assert_eq!(out, vynpad_driver::XReport::default());
    }

    /// Com curvas monotônicas, empurrar o analógico mais longe nunca diminui a saída.
    #[test]
    fn stick_output_is_monotonic_with_input(
        cfg in stick(true),
        a in 0i16..=32767,
        b in 0i16..=32767,
    ) {
        let mut p = Profile::identity();
        p.sticks.left = StickConfig { invert_x: false, ..cfg };
        let compiled = CompiledProfile::compile(&p).unwrap();
        let (lo, hi) = (a.min(b), a.max(b));
        let run = |x: i16| {
            let raw = RawInput { lx: x, ..RawInput::default() };
            apply(&compiled, &raw, &mut MapperState::new(), 0.004).lx
        };
        prop_assert!(run(lo) <= run(hi));
    }

    /// Perfis válidos sobrevivem ao JSON sem perder nada.
    #[test]
    fn json_roundtrip_is_lossless(p in profile(false)) {
        let again = Profile::from_json(&p.to_json()).unwrap();
        prop_assert_eq!(again, p);
    }

    /// Nunca sai um botão que não exista, mesmo com remapeamento e turbo.
    #[test]
    fn output_buttons_are_always_known_bits(p in profile(false), raw in raw_input()) {
        let compiled = CompiledProfile::compile(&p).unwrap();
        let out = apply(&compiled, &raw, &mut MapperState::new(), 0.004);
        prop_assert_eq!(out.buttons & !Button::ALL_MASK, 0);
    }
}
