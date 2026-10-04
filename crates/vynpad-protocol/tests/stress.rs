//! Estresse por mutação, sem libFuzzer: roda na toolchain estável e no Windows.
//!
//! Parte dos test vectors, aplica mutações aleatórias (bits, bytes, cortes, extensões)
//! e confere as mesmas propriedades dos alvos de `fuzz/`: nada entra em pânico, nenhum
//! datagrama adulterado é aceito e toda mensagem de controle aceita sobrevive à ida e volta.
//!
//! Com duração curta roda junto com `cargo test`; para uma rodada longa:
//! `VYNPAD_STRESS_SECS=600 cargo test -p vynpad-protocol --release --test stress -- --nocapture`

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::Value;
use vynpad_protocol::control::{Incoming, decode_frame, encode_frame};
use vynpad_protocol::{
    KEY_LEN, MAX_PACKET_LEN, Role, Secret, Session, SessionCipher, parse_header, seal_packet,
};

/// xorshift64*: determinístico a partir da semente, sem dependência extra.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).unwrap()
    }
    fn byte(&mut self) -> u8 {
        self.next().to_le_bytes()[0]
    }
}

fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut out = input.to_vec();
    for _ in 0..=rng.below(4) {
        match rng.below(6) {
            0 if !out.is_empty() => {
                let i = rng.below(out.len());
                out[i] ^= 1 << rng.below(8);
            }
            1 if !out.is_empty() => {
                let i = rng.below(out.len());
                out[i] = rng.byte();
            }
            2 if !out.is_empty() => out.truncate(rng.below(out.len())),
            3 => {
                let extra = rng.below(8) + 1;
                out.extend((0..extra).map(|_| rng.byte()));
            }
            4 if out.len() > 1 => {
                let i = rng.below(out.len() - 1);
                out.swap(i, i + 1);
            }
            _ => {
                // Bytes "interessantes" em posição aleatória.
                if !out.is_empty() {
                    let i = rng.below(out.len());
                    out[i] = [0x00, 0xFF, 0x7F, 0x80, b'"', b'{', b'}'][rng.below(7)];
                }
            }
        }
    }
    out
}

fn seeds(file: &str, field: &str) -> Vec<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol/test-vectors")
        .join(file);
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    doc["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| hex::decode(v[field].as_str().unwrap()).unwrap())
        .collect()
}

fn check_udp(data: &[u8], seed: &[u8], key: &Secret<[u8; KEY_LEN]>) {
    for role in [Role::Server, Role::Client] {
        let session_id = parse_header(data, role).map_or(1, |h| h.session_id);
        let mut session = Session::new(role, session_id, key);
        if let Ok(packet) = session.receive(data) {
            // Aceito só se for a semente intacta ou exatamente o que o encoder produz
            // para esse conteúdo: ou seja, uma codificação autêntica, nunca um pacote
            // adulterado. (A mutação pode desfazer o defeito de um vector inválido.)
            let mut resealed = [0u8; MAX_PACKET_LEN];
            let n = seal_packet(
                &SessionCipher::new(key),
                packet.session_id,
                packet.seq,
                &packet.payload,
                &mut resealed,
            )
            .unwrap();
            assert!(
                data == seed || data == &resealed[..n],
                "datagrama adulterado aceito: {packet:?}"
            );
        }
    }
}

fn check_control(data: &[u8]) {
    if let Ok(Incoming::Message(msg)) = decode_frame(data) {
        let frame = encode_frame(&msg).expect("mensagem aceita precisa ser codificável");
        assert_eq!(decode_frame(&frame), Ok(Incoming::Message(msg)));
    }
}

#[test]
fn mutacoes_nao_quebram_a_decodificacao() {
    let secs: u64 = std::env::var("VYNPAD_STRESS_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let seed: u64 = std::env::var("VYNPAD_STRESS_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5EED);
    let mut rng = Rng(seed | 1);

    // Chave de teste dos vectors (os que usam outra chave nunca abrem).
    let key: [u8; KEY_LEN] = std::array::from_fn(|i| u8::try_from(i).unwrap());
    let key = Secret::new(key);
    let udp = seeds("udp.json", "packet_hex");
    let control = seeds("control.json", "frame_hex");

    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut runs = 0u64;
    while Instant::now() < deadline {
        for _ in 0..1_000 {
            let seed = &udp[rng.below(udp.len())];
            check_udp(&mutate(&mut rng, seed), seed, &key);
            let frame = &control[rng.below(control.len())];
            check_control(&mutate(&mut rng, frame));
            runs += 1;
        }
    }
    println!("{runs} iterações (semente {seed:#x}, {secs} s)");
}
