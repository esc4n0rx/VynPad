//! Propriedades do protocolo com `proptest` (T-701): o que vale para **qualquer** entrada, em
//! complemento aos test vectors (casos escolhidos) e ao estresse por mutação (`stress.rs`).

use std::net::Ipv4Addr;

use proptest::prelude::*;
use vynpad_protocol::pairing_uri::PairingUri;
use vynpad_protocol::{
    Buttons, InputState, KEY_LEN, MAX_PACKET_LEN, Motion, Payload, Ping, Pong, ProtoError,
    ReplayWindow, Role, Rumble, Secret, Session,
};

fn buttons() -> impl Strategy<Value = Buttons> {
    // `from_wire` mascara os bits reservados, como o decode faz.
    any::<u16>().prop_map(Buttons::from_wire)
}

fn input_state() -> impl Strategy<Value = InputState> {
    (
        any::<i64>(),
        buttons(),
        (any::<u8>(), any::<u8>()),
        (any::<i16>(), any::<i16>(), any::<i16>(), any::<i16>()),
        any::<u16>(),
        any::<u8>(),
    )
        .prop_map(
            |(t_client_us, buttons, (lt, rt), (lx, ly, rx, ry), rumble_ack_id, battery)| {
                InputState {
                    t_client_us,
                    buttons,
                    lt,
                    rt,
                    lx,
                    ly,
                    rx,
                    ry,
                    rumble_ack_id,
                    battery,
                }
            },
        )
}

fn motion() -> impl Strategy<Value = Motion> {
    prop::array::uniform6(any::<i16>()).prop_map(|[gx, gy, gz, ax, ay, az]| Motion {
        gx,
        gy,
        gz,
        ax,
        ay,
        az,
    })
}

/// Payloads que o client envia.
fn client_payload() -> impl Strategy<Value = Payload> {
    prop_oneof![
        input_state().prop_map(Payload::Input),
        (input_state(), motion()).prop_map(|(s, m)| Payload::InputMotion(s, m)),
        any::<i64>().prop_map(|t_client_us| Payload::Ping(Ping { t_client_us })),
    ]
}

/// Payloads que o server envia.
fn server_payload() -> impl Strategy<Value = Payload> {
    prop_oneof![
        (any::<i64>(), any::<i64>(), any::<u32>()).prop_map(
            |(t_client_us, t_server_rx_us, server_proc_us)| Payload::Pong(Pong {
                t_client_us,
                t_server_rx_us,
                server_proc_us,
            })
        ),
        (any::<u16>(), any::<u8>(), any::<u8>()).prop_map(|(rumble_id, large, small)| {
            Payload::Rumble(Rumble {
                rumble_id,
                large,
                small,
            })
        }),
    ]
}

fn pair(session_id: u32, key: [u8; KEY_LEN]) -> (Session, Session) {
    let key = Secret::new(key);
    (
        Session::new(Role::Client, session_id, &key),
        Session::new(Role::Server, session_id, &key),
    )
}

proptest! {
    /// Tudo que o client sela, o server abre igual, com o `seq` e a sessão certos.
    #[test]
    fn client_to_server_round_trip(
        id in any::<u32>(),
        key in any::<[u8; KEY_LEN]>(),
        payload in client_payload(),
    ) {
        let (mut client, mut server) = pair(id, key);
        let mut buf = [0u8; MAX_PACKET_LEN];
        let n = client.send(&payload, &mut buf).unwrap();
        let packet = server.receive(&buf[..n]).unwrap();
        prop_assert_eq!(packet.payload, payload);
        prop_assert_eq!(packet.session_id, id);
        prop_assert_eq!(packet.seq, 0);
    }

    /// E o mesmo no sentido contrário (Pong e Rumble).
    #[test]
    fn server_to_client_round_trip(
        id in any::<u32>(),
        key in any::<[u8; KEY_LEN]>(),
        payload in server_payload(),
    ) {
        let (mut client, mut server) = pair(id, key);
        let mut buf = [0u8; MAX_PACKET_LEN];
        let n = server.send(&payload, &mut buf).unwrap();
        let packet = client.receive(&buf[..n]).unwrap();
        prop_assert_eq!(packet.payload, payload);
    }

    /// Um papel não envia o payload do outro.
    #[test]
    fn wrong_direction_is_refused(payload in server_payload()) {
        let (mut client, _) = pair(1, [3; KEY_LEN]);
        let mut buf = [0u8; MAX_PACKET_LEN];
        prop_assert_eq!(client.send(&payload, &mut buf), Err(ProtoError::WrongDirection));
    }

    /// Adulterar **um bit** em qualquer posição do datagrama derruba o pacote: o cabeçalho é
    /// conferido e a tag AEAD cobre cabeçalho e conteúdo.
    #[test]
    fn any_bit_flip_is_rejected(
        key in any::<[u8; KEY_LEN]>(),
        payload in client_payload(),
        bit in any::<usize>(),
    ) {
        let (mut client, mut server) = pair(7, key);
        let mut buf = [0u8; MAX_PACKET_LEN];
        let n = client.send(&payload, &mut buf).unwrap();
        let bit = bit % (n * 8);
        buf[bit / 8] ^= 1 << (bit % 8);
        prop_assert!(server.receive(&buf[..n]).is_err());
    }

    /// Reenviar o mesmo datagrama não vale uma segunda vez (anti-replay).
    #[test]
    fn replayed_datagram_is_stale(key in any::<[u8; KEY_LEN]>(), payload in client_payload()) {
        let (mut client, mut server) = pair(7, key);
        let mut buf = [0u8; MAX_PACKET_LEN];
        let n = client.send(&payload, &mut buf).unwrap();
        prop_assert!(server.receive(&buf[..n]).is_ok());
        prop_assert_eq!(server.receive(&buf[..n]), Err(ProtoError::StaleSeq));
    }

    /// Bytes quaisquer nunca causam pânico e nunca são aceitos sem a chave.
    #[test]
    fn arbitrary_bytes_never_pass(bytes in prop::collection::vec(any::<u8>(), 0..=2 * MAX_PACKET_LEN)) {
        let (_, mut server) = pair(7, [9; KEY_LEN]);
        prop_assert!(server.receive(&bytes).is_err());
    }

    /// A janela aceita cada `seq` no máximo uma vez e só em ordem crescente.
    #[test]
    fn replay_window_accepts_only_increasing(seqs in prop::collection::vec(any::<u32>(), 0..200)) {
        let mut window = ReplayWindow::new();
        let mut last: Option<u32> = None;
        for seq in seqs {
            let accepted = window.check(seq).is_ok();
            prop_assert_eq!(accepted, last.is_none_or(|l| seq > l));
            if accepted {
                window.commit(seq);
                last = Some(seq);
            }
        }
        prop_assert_eq!(window.last(), last);
    }

    /// O `rumble_id` nunca é 0 (0 significa "nenhum") e sempre muda.
    #[test]
    fn rumble_id_never_zero(id in any::<u16>()) {
        let next = Rumble::next_id(id);
        prop_assert_ne!(next, 0);
        prop_assert_ne!(next, id);
    }

    /// A URI do QR sobrevive à ida e volta, com qualquer nome de 1 a 64 caracteres.
    #[test]
    fn pairing_uri_round_trip(
        hosts in prop::collection::vec(any::<[u8; 4]>().prop_map(Ipv4Addr::from), 1..=4),
        port in any::<u16>(),
        fingerprint in any::<[u8; 32]>(),
        token in any::<[u8; 16]>(),
        name in "\\PC{1,64}",
    ) {
        let uri = PairingUri {
            hosts,
            port,
            fingerprint,
            pairing_token: Secret::new(token),
            name,
        };
        let text = uri.encode().unwrap();
        prop_assert!(text.starts_with("vynpad://pair?"));
        prop_assert_eq!(PairingUri::parse(&text).unwrap(), uri);
    }
}
