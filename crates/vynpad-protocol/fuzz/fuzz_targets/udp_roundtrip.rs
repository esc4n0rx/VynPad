//! Payloads com valores arbitrários: selar e abrir devolve exatamente o mesmo payload.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vynpad_protocol::{
    Buttons, InputState, KEY_LEN, MAX_PACKET_LEN, Motion, Payload, Ping, Pong, Role, Rumble,
    Secret, Session,
};

struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        let n = N.min(self.0.len());
        out[..n].copy_from_slice(&self.0[..n]);
        self.0 = &self.0[n..];
        out
    }
    fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
    }
    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take())
    }
    fn i16(&mut self) -> i16 {
        i16::from_le_bytes(self.take())
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }
    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.take())
    }
    fn input(&mut self) -> InputState {
        InputState {
            t_client_us: self.i64(),
            buttons: Buttons::from_wire(self.u16()),
            lt: self.u8(),
            rt: self.u8(),
            lx: self.i16(),
            ly: self.i16(),
            rx: self.i16(),
            ry: self.i16(),
            rumble_ack_id: self.u16(),
            battery: self.u8(),
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let mut b = Bytes(data);
    let selector = b.u8();
    let payload = match selector % 5 {
        0 => Payload::Input(b.input()),
        1 => Payload::InputMotion(
            b.input(),
            Motion {
                gx: b.i16(),
                gy: b.i16(),
                gz: b.i16(),
                ax: b.i16(),
                ay: b.i16(),
                az: b.i16(),
            },
        ),
        2 => Payload::Ping(Ping {
            t_client_us: b.i64(),
        }),
        3 => Payload::Pong(Pong {
            t_client_us: b.i64(),
            t_server_rx_us: b.i64(),
            server_proc_us: b.u32(),
        }),
        _ => Payload::Rumble(Rumble {
            rumble_id: b.u16(),
            large: b.u8(),
            small: b.u8(),
        }),
    };
    let session_id = b.u32();
    let key = Secret::new(b.take::<KEY_LEN>());

    let (tx_role, rx_role) = match payload {
        Payload::Pong(_) | Payload::Rumble(_) => (Role::Server, Role::Client),
        _ => (Role::Client, Role::Server),
    };
    let mut tx = Session::new(tx_role, session_id, &key);
    let mut rx = Session::new(rx_role, session_id, &key);

    let mut buf = [0u8; MAX_PACKET_LEN];
    for expected_seq in 0..2 {
        let n = tx.send(&payload, &mut buf).expect("envio");
        let got = rx.receive(&buf[..n]).expect("recepção");
        assert_eq!(got.payload, payload);
        assert_eq!(got.seq, expected_seq);
        assert!(rx.receive(&buf[..n]).is_err(), "replay aceito");
    }
});
