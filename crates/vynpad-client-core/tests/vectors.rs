//! O núcleo do client contra os `protocol/test-vectors/`: o Kotlin chama este mesmo código, então
//! passar aqui é o contrato de que o app e o server falam o mesmo protocolo.

use std::path::PathBuf;

use serde_json::{Value, json};
use vynpad_client_core::{
    ClientSession, CoreError, GamepadInput, MotionSample, ReceivedPacket, discovery_port,
    discovery_request, parse_discovery_response, parse_pairing_uri,
};
use vynpad_protocol::{KEY_LEN, Payload, Role, Secret, Session};

fn load(file: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol/test-vectors")
        .join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["format"], 1, "{file}: formato de vectors desconhecido");
    doc["vectors"].as_array().unwrap().clone()
}

fn vector(name: &str) -> Value {
    load("udp.json")
        .into_iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("vector {name} não existe"))
}

fn client_for(v: &Value) -> std::sync::Arc<ClientSession> {
    let r = &v["receiver"];
    let key = hex::decode(r["key_hex"].as_str().unwrap()).unwrap();
    let id = u32::try_from(r["session_id"].as_u64().unwrap()).unwrap();
    ClientSession::new(id, key).unwrap()
}

#[test]
fn pairing_uri_vectors() {
    for v in load("pairing-uri.json") {
        let name = v["name"].as_str().unwrap();
        let parsed = parse_pairing_uri(v["uri"].as_str().unwrap());
        let expect = &v["expect"];
        if expect["result"] == "ok" {
            let got = parsed.unwrap_or_else(|e| panic!("{name}: {e}"));
            let p = &expect["params"];
            assert_eq!(json!(got.hosts), p["hosts"], "{name}");
            assert_eq!(json!(got.port), p["port"], "{name}");
            assert_eq!(
                hex::encode(&got.fingerprint),
                p["fingerprint_hex"],
                "{name}"
            );
            assert_eq!(
                hex::encode(&got.pairing_token),
                p["pairing_token_hex"],
                "{name}"
            );
            assert_eq!(json!(got.name), p["name"], "{name}");
        } else {
            match parsed.expect_err(name) {
                CoreError::PairingUri { code } => {
                    assert_eq!(json!(code), expect["error"], "{name}");
                }
                other => panic!("{name}: erro inesperado {other:?}"),
            }
        }
    }
}

/// O primeiro input da sessão (seq 0) sai byte a byte igual ao do vector `input_neutral`.
#[test]
fn first_input_matches_the_neutral_vector() {
    let v = vector("input_neutral");
    let session = client_for(&v);
    let p = &v["expect"]["payload"];
    let input = GamepadInput {
        t_client_us: p["t_client_us"].as_i64().unwrap(),
        battery: u8::try_from(p["battery"].as_u64().unwrap()).unwrap(),
        ..GamepadInput::default()
    };
    let sealed = session.encode_input(input).unwrap();
    assert_eq!(hex::encode(sealed), v["packet_hex"].as_str().unwrap());
}

/// Tudo que o client sela é aceito pelo lado do server, com os mesmos valores.
#[test]
fn client_packets_are_accepted_by_a_server_session() {
    let key = [9u8; KEY_LEN];
    let client = ClientSession::new(77, key.to_vec()).unwrap();
    let mut server = Session::new(Role::Server, 77, &Secret::new(key));

    let input = GamepadInput {
        t_client_us: 5_000_000,
        buttons: 0x1000 | 0x0100,
        lt: 12,
        rt: 250,
        lx: -32768,
        ly: 32767,
        rx: 100,
        ry: -100,
        rumble_ack_id: 3,
        battery: 80,
    };
    let motion = MotionSample {
        gx: 1,
        gy: -2,
        gz: 3,
        ax: 100,
        ay: -100,
        az: 981,
    };

    let plain = server
        .receive(&client.encode_input(input).unwrap())
        .unwrap();
    let Payload::Input(state) = plain.payload else {
        panic!("esperava Input")
    };
    assert_eq!(state.buttons.bits(), input.buttons);
    assert_eq!(
        (state.lt, state.rt, state.lx, state.ly),
        (12, 250, -32768, 32767)
    );
    assert_eq!(plain.seq, 0);

    let with_motion = server
        .receive(&client.encode_input_motion(input, motion).unwrap())
        .unwrap();
    let Payload::InputMotion(_, m) = with_motion.payload else {
        panic!("esperava InputMotion")
    };
    assert_eq!((m.gx, m.gy, m.gz, m.az), (1, -2, 3, 981));
    assert_eq!(with_motion.seq, 1);

    let ping = server.receive(&client.encode_ping(42).unwrap()).unwrap();
    assert!(matches!(ping.payload, Payload::Ping(p) if p.t_client_us == 42));
}

#[test]
fn receives_pong_and_rumble_from_the_vectors() {
    let pong = vector("pong");
    let got = client_for(&pong)
        .receive(&hex::decode(pong["packet_hex"].as_str().unwrap()).unwrap())
        .unwrap();
    assert_eq!(
        got,
        ReceivedPacket::Pong {
            t_client_us: 123_456_789_012,
            t_server_rx_us: 98_765_432_101,
            server_proc_us: 37,
        }
    );

    // O vector de rumble pressupõe `last_seq = 4`: abrir primeiro um pong não serve, então
    // validamos o seq 5 numa sessão que já recebeu os anteriores.
    let rumble = vector("rumble_on");
    let session = client_for(&rumble);
    let datagram = hex::decode(rumble["packet_hex"].as_str().unwrap()).unwrap();
    let got = session.receive(&datagram).unwrap();
    assert_eq!(
        got,
        ReceivedPacket::Rumble {
            rumble_id: 1,
            large: 255,
            small: 128
        }
    );

    // Reapresentar o mesmo datagrama é replay.
    assert_eq!(
        session.receive(&datagram).unwrap_err(),
        CoreError::Protocol {
            code: "stale_seq".to_owned()
        }
    );
}

#[test]
fn client_side_error_vectors_use_the_stable_codes() {
    for v in load("udp.json") {
        if v["receiver"]["role"] != "client" || v["receiver"]["last_seq"] != Value::Null {
            continue;
        }
        if v["expect"]["result"] != "error" {
            continue;
        }
        let name = v["name"].as_str().unwrap();
        let err = client_for(&v)
            .receive(&hex::decode(v["packet_hex"].as_str().unwrap()).unwrap())
            .expect_err(name);
        match err {
            CoreError::Protocol { code } => assert_eq!(json!(code), v["expect"]["error"], "{name}"),
            other => panic!("{name}: erro inesperado {other:?}"),
        }
    }
}

#[test]
fn rejects_a_key_with_the_wrong_length() {
    let err = ClientSession::new(1, vec![0; 31]).err().unwrap();
    assert!(matches!(err, CoreError::InvalidArgument { .. }));
}

#[test]
fn parses_a_discovery_response_and_exposes_the_request() {
    let json = br#"{"id":"0b8e6a52-7d1c-4c0e-9f3e-0a1b2c3d4e5f","name":"PC-Sala","ver":1,"tcp":47800,"udp":47801,"os":"windows","free":3,"fp":"af8beea5aef0bef6"}"#;
    let s = parse_discovery_response(json).unwrap();
    assert_eq!(
        (s.name.as_str(), s.tcp_port, s.udp_port, s.free_slots),
        ("PC-Sala", 47800, 47801, 3)
    );
    assert_eq!(s.short_fingerprint, "af8beea5aef0bef6");

    assert!(matches!(
        parse_discovery_response(b"{}"),
        Err(CoreError::Discovery { .. })
    ));
    assert_eq!(discovery_request(), b"VPDISC1");
    assert_eq!(discovery_port(), 47802);
}
