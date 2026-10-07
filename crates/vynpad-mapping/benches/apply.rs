//! Hot path do mapeamento: a meta da T-206 é `apply` < 5 µs.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use vynpad_driver::xbuttons;
use vynpad_mapping::{
    Accel, Button, ButtonTarget, CompiledProfile, Curve, Gyro, MapperState, Profile, RawInput,
    Turbo, apply,
};

/// Perfil que liga todas as etapas: curvas, deadzones, gyro com filtro, remapeamento e turbo.
fn heavy_profile() -> Profile {
    let mut p = Profile::standard();
    p.sticks.left.curve = Curve::Exponential { exponent: 1.8 };
    p.sticks.right.curve = Curve::Custom {
        points: [(0.2, 0.1), (0.4, 0.3), (0.6, 0.6), (0.8, 0.85)],
    };
    p.sticks.right.anti_deadzone = 0.1;
    p.gyro.enabled = true;
    p.buttons
        .remap
        .insert(Button::A, ButtonTarget::Button(Button::B));
    p.buttons
        .remap
        .insert(Button::Y, ButtonTarget::RightTrigger);
    p.buttons.turbo.push(Turbo {
        button: Button::X,
        hz: 12.0,
    });
    p
}

fn bench(c: &mut Criterion) {
    let raw = RawInput {
        buttons: xbuttons::A | xbuttons::X | xbuttons::LEFT_SHOULDER,
        lt: 90,
        rt: 200,
        lx: 12_000,
        ly: -8_000,
        rx: -20_000,
        ry: 5_000,
        gyro: Some(Gyro {
            x: 0.3,
            y: -0.8,
            z: 0.1,
        }),
        accel: Some(Accel {
            x: 1.2,
            y: 9.6,
            z: 0.4,
        }),
    };

    for (name, profile) in [
        ("standard", Profile::standard()),
        ("heavy", heavy_profile()),
    ] {
        let compiled = CompiledProfile::compile(&profile).unwrap();
        let mut state = MapperState::new();
        c.bench_function(&format!("apply/{name}"), |b| {
            b.iter(|| apply(&compiled, black_box(&raw), &mut state, black_box(0.004)));
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
