//! Hot path UDP: meta da T-102 é decode + open < 2 µs por pacote.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use padgaming_protocol::{
    Buttons, InputState, KEY_LEN, MAX_PACKET_LEN, Motion, Payload, ReplayWindow, Role, Secret,
    SessionCipher, open_packet, parse_header, seal_packet,
};

fn bench(c: &mut Criterion) {
    let cipher = SessionCipher::new(&Secret::new([0x42; KEY_LEN]));
    let state = InputState {
        t_client_us: 123_456_789,
        buttons: Buttons::A | Buttons::RIGHT_SHOULDER,
        rt: 200,
        lx: 12_000,
        ly: -8_000,
        battery: 80,
        ..InputState::default()
    };

    for (name, payload) in [
        ("input", Payload::Input(state)),
        (
            "input_motion",
            Payload::InputMotion(
                state,
                Motion {
                    ay: 981,
                    ..Motion::default()
                },
            ),
        ),
    ] {
        let mut packet = [0u8; MAX_PACKET_LEN];
        let n = seal_packet(&cipher, 1, 1_000, &payload, &mut packet).unwrap();
        let packet = &packet[..n];

        c.bench_function(&format!("decode+open/{name}"), |b| {
            b.iter(|| {
                let header = parse_header(black_box(packet), Role::Server).unwrap();
                let mut replay = ReplayWindow::with_last(Some(999));
                open_packet(&cipher, &mut replay, &header, packet).unwrap()
            });
        });

        let mut out = [0u8; MAX_PACKET_LEN];
        c.bench_function(&format!("encode+seal/{name}"), |b| {
            b.iter(|| {
                seal_packet(&cipher, 1, black_box(1_000), black_box(&payload), &mut out).unwrap()
            });
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
