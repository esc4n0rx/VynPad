//! Confere a crate contra `protocol/test-vectors/` (PROTOCOLO §11).

use std::path::PathBuf;

use padgaming_protocol::control::{ControlMessage, Incoming, decode_frame, encode_frame};
use padgaming_protocol::{
    Buttons, InputState, KEY_LEN, MAGIC, MAX_PACKET_LEN, Motion, Packet, PacketType, Payload, Ping,
    Pong, ProtoError, ReplayWindow, Role, Rumble, Secret, SessionCipher, VERSION, open_packet,
    parse_header, seal_packet,
};
use serde_json::{Value, json};

fn load(file: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol/test-vectors")
        .join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["format"], 1, "{file}: formato de vectors desconhecido");
    doc["vectors"].as_array().unwrap().clone()
}

fn num<T: TryFrom<i64>>(v: &Value, field: &str) -> T {
    let n = v[field]
        .as_i64()
        .unwrap_or_else(|| panic!("campo {field} ausente"));
    T::try_from(n).unwrap_or_else(|_| panic!("campo {field} fora da faixa"))
}

// ---------------------------------------------------------------- UDP

/// Decodificação completa como o server/client fazem: passos 1–10 da §6.5.
fn decode(packet: &[u8], receiver: &Value) -> Result<Packet, ProtoError> {
    let role = match receiver["role"].as_str().unwrap() {
        "server" => Role::Server,
        "client" => Role::Client,
        other => panic!("role desconhecido: {other}"),
    };
    let key: [u8; KEY_LEN] = hex::decode(receiver["key_hex"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let last = receiver["last_seq"]
        .as_u64()
        .map(|n| u32::try_from(n).unwrap());

    let header = parse_header(packet, role)?;
    if u64::from(header.session_id) != receiver["session_id"].as_u64().unwrap() {
        return Err(ProtoError::UnknownSession);
    }
    let mut replay = ReplayWindow::with_last(last);
    let result = open_packet(
        &SessionCipher::new(&Secret::new(key)),
        &mut replay,
        &header,
        packet,
    );
    // A janela só avança quando o pacote é aceito.
    let expected_last = if result.is_ok() {
        Some(header.seq)
    } else {
        last
    };
    assert_eq!(replay.last(), expected_last);
    result
}

fn input_to_json(s: &InputState, motion: bool, out: &mut serde_json::Map<String, Value>) {
    out.extend([
        ("t_client_us".into(), json!(s.t_client_us)),
        ("buttons".into(), json!(s.buttons.bits())),
        ("lt".into(), json!(s.lt)),
        ("rt".into(), json!(s.rt)),
        ("lx".into(), json!(s.lx)),
        ("ly".into(), json!(s.ly)),
        ("rx".into(), json!(s.rx)),
        ("ry".into(), json!(s.ry)),
        ("rumble_ack_id".into(), json!(s.rumble_ack_id)),
        ("flags".into(), json!(u8::from(motion))),
        ("battery".into(), json!(s.battery)),
    ]);
}

fn payload_to_json(p: &Payload) -> Value {
    let mut m = serde_json::Map::new();
    match p {
        Payload::Input(s) => input_to_json(s, false, &mut m),
        Payload::InputMotion(s, mo) => {
            input_to_json(s, true, &mut m);
            m.extend([
                ("gx".into(), json!(mo.gx)),
                ("gy".into(), json!(mo.gy)),
                ("gz".into(), json!(mo.gz)),
                ("ax".into(), json!(mo.ax)),
                ("ay".into(), json!(mo.ay)),
                ("az".into(), json!(mo.az)),
            ]);
        }
        Payload::Ping(p) => {
            m.insert("t_client_us".into(), json!(p.t_client_us));
        }
        Payload::Pong(p) => m.extend([
            ("t_client_us".into(), json!(p.t_client_us)),
            ("t_server_rx_us".into(), json!(p.t_server_rx_us)),
            ("server_proc_us".into(), json!(p.server_proc_us)),
        ]),
        Payload::Rumble(r) => m.extend([
            ("rumble_id".into(), json!(r.rumble_id)),
            ("large".into(), json!(r.large)),
            ("small".into(), json!(r.small)),
        ]),
    }
    Value::Object(m)
}

fn input_from_json(v: &Value) -> InputState {
    InputState {
        t_client_us: num(v, "t_client_us"),
        buttons: Buttons::from_bits(num(v, "buttons"))
            .expect("bits reservados num vector de encode"),
        lt: num(v, "lt"),
        rt: num(v, "rt"),
        lx: num(v, "lx"),
        ly: num(v, "ly"),
        rx: num(v, "rx"),
        ry: num(v, "ry"),
        rumble_ack_id: num(v, "rumble_ack_id"),
        battery: num(v, "battery"),
    }
}

fn payload_from_json(ty: PacketType, v: &Value) -> Payload {
    match ty {
        PacketType::Input => Payload::Input(input_from_json(v)),
        PacketType::InputMotion => Payload::InputMotion(
            input_from_json(v),
            Motion {
                gx: num(v, "gx"),
                gy: num(v, "gy"),
                gz: num(v, "gz"),
                ax: num(v, "ax"),
                ay: num(v, "ay"),
                az: num(v, "az"),
            },
        ),
        PacketType::Ping => Payload::Ping(Ping {
            t_client_us: num(v, "t_client_us"),
        }),
        PacketType::Pong => Payload::Pong(Pong {
            t_client_us: num(v, "t_client_us"),
            t_server_rx_us: num(v, "t_server_rx_us"),
            server_proc_us: num(v, "server_proc_us"),
        }),
        PacketType::Rumble => Payload::Rumble(Rumble {
            rumble_id: num(v, "rumble_id"),
            large: num(v, "large"),
            small: num(v, "small"),
        }),
    }
}

#[test]
fn udp() {
    let vectors = load("udp.json");
    assert!(!vectors.is_empty());
    for v in &vectors {
        let name = v["name"].as_str().unwrap();
        let packet = hex::decode(v["packet_hex"].as_str().unwrap()).unwrap();
        let expect = &v["expect"];
        let got = decode(&packet, &v["receiver"]);

        match expect["result"].as_str().unwrap() {
            "error" => {
                let err = got.expect_err(name);
                assert_eq!(err.code(), expect["error"].as_str().unwrap(), "{name}");
            }
            "ok" => {
                let packet_ok = got.unwrap_or_else(|e| panic!("{name}: {e}"));
                let h = &expect["header"];
                let ty = PacketType::from_u8(num(h, "type")).unwrap();
                assert_eq!(num::<u16>(h, "magic"), MAGIC, "{name}");
                assert_eq!(num::<u8>(h, "ver"), VERSION, "{name}");
                assert_eq!(packet_ok.payload.packet_type(), ty, "{name}");
                assert_eq!(packet_ok.session_id, num::<u32>(h, "session_id"), "{name}");
                assert_eq!(packet_ok.seq, num::<u32>(h, "seq"), "{name}");
                assert_eq!(
                    payload_to_json(&packet_ok.payload),
                    expect["payload"],
                    "{name}"
                );

                if v["decode_only"].as_bool() != Some(true) {
                    // Encoder a partir do `expect` (não do valor decodificado).
                    let key: [u8; KEY_LEN] =
                        hex::decode(v["receiver"]["key_hex"].as_str().unwrap())
                            .unwrap()
                            .try_into()
                            .unwrap();
                    let payload = payload_from_json(ty, &expect["payload"]);
                    let mut buf = [0u8; MAX_PACKET_LEN];
                    let n = seal_packet(
                        &SessionCipher::new(&Secret::new(key)),
                        packet_ok.session_id,
                        packet_ok.seq,
                        &payload,
                        &mut buf,
                    )
                    .unwrap();
                    assert_eq!(
                        hex::encode(&buf[..n]),
                        v["packet_hex"].as_str().unwrap(),
                        "{name}"
                    );
                }
            }
            other => panic!("{name}: result desconhecido {other}"),
        }
    }
    println!("{} vectors UDP", vectors.len());
}

// ---------------------------------------------------------------- Controle

#[test]
fn control() {
    let vectors = load("control.json");
    assert!(!vectors.is_empty());
    for v in &vectors {
        let name = v["name"].as_str().unwrap();
        let frame = hex::decode(v["frame_hex"].as_str().unwrap()).unwrap();
        let expect = &v["expect"];
        let got = decode_frame(&frame);

        let result = match &got {
            Ok(Incoming::Message(_)) => "ok",
            Ok(Incoming::Ignored) => "ignore",
            Err(e) => e.code(),
        };
        assert_eq!(
            result,
            expect["result"].as_str().unwrap(),
            "{name}: {got:?}"
        );

        if let Ok(Incoming::Message(msg)) = got {
            let expected: ControlMessage =
                serde_json::from_value(expect["message"].clone()).unwrap();
            assert_eq!(msg, expected, "{name}");
            // Ida e volta pelo nosso encoder.
            let again = decode_frame(&encode_frame(&msg).unwrap()).unwrap();
            assert_eq!(again, Incoming::Message(msg), "{name}");
        }
    }
    println!("{} vectors de controle", vectors.len());
}
