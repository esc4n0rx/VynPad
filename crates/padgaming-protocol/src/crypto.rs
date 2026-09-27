//! ChaCha20-Poly1305 dos pacotes UDP (PROTOCOLO §6.2), selando e abrindo sem alocação.

use core::fmt;

use chacha20poly1305::aead::array::Array;
use chacha20poly1305::{AeadInOut, ChaCha20Poly1305, Key, KeyInit, Nonce, Tag};

use crate::error::ProtoError;
use crate::packet::{
    Direction, HEADER_LEN, MAX_PAYLOAD_LEN, Packet, PacketHeader, Payload, TAG_LEN,
};
use crate::secret::Secret;
use crate::session::ReplayWindow;

/// Tamanho da `udp_key`.
pub const KEY_LEN: usize = 32;

/// Cifra de uma sessão, criada a partir da `udp_key` do `welcome`.
///
/// É barata de clonar: as threads de recepção e de envio podem ter cada uma a sua.
#[derive(Clone)]
pub struct SessionCipher(ChaCha20Poly1305);

impl SessionCipher {
    /// Cria a cifra a partir da chave da sessão.
    pub fn new(key: &Secret<[u8; KEY_LEN]>) -> Self {
        Self(ChaCha20Poly1305::new(&Key::from(*key.expose())))
    }
}

impl fmt::Debug for SessionCipher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionCipher(***)")
    }
}

/// Nonce de 12 bytes: `direction (u32 LE) ‖ seq (u32 LE) ‖ 00 00 00 00`.
fn nonce(direction: Direction, seq: u32) -> Nonce {
    let mut n = [0u8; 12];
    n[0..4].copy_from_slice(&(direction as u32).to_le_bytes());
    n[4..8].copy_from_slice(&seq.to_le_bytes());
    Array::from(n)
}

/// Monta e sela um pacote em `out` com um `seq` explícito; devolve o tamanho escrito.
///
/// Quem controla o `seq` é o chamador: normalmente [`crate::Session::send`], que usa um
/// [`crate::SeqCounter`]. **Nunca** repita um `seq` com a mesma chave.
pub fn seal_packet(
    cipher: &SessionCipher,
    session_id: u32,
    seq: u32,
    payload: &Payload,
    out: &mut [u8],
) -> Result<usize, ProtoError> {
    let packet_type = payload.packet_type();
    let len = packet_type.packet_len();
    let out = out.get_mut(..len).ok_or(ProtoError::BufferTooSmall)?;

    let header = PacketHeader {
        packet_type,
        session_id,
        seq,
    }
    .to_bytes();
    let (head, rest) = out.split_at_mut(HEADER_LEN);
    head.copy_from_slice(&header);
    let (body, tag_out) = rest.split_at_mut(packet_type.payload_len());
    payload.write(body);

    let tag = cipher
        .0
        .encrypt_inout_detached(&nonce(packet_type.direction(), seq), &header, body.into())
        .map_err(|_| ProtoError::BufferTooSmall)?;
    tag_out.copy_from_slice(&tag);
    Ok(len)
}

/// Passos 8 a 10 da PROTOCOLO §6.5 para um pacote cujo cabeçalho já passou por
/// [`crate::parse_header`] e cuja sessão já foi encontrada (passo 7).
///
/// Só avança `replay` se o pacote for aceito por inteiro. O payload é decifrado numa
/// cópia na pilha, então `packet` não é alterado.
pub fn open_packet(
    cipher: &SessionCipher,
    replay: &mut ReplayWindow,
    header: &PacketHeader,
    packet: &[u8],
) -> Result<Packet, ProtoError> {
    replay.check(header.seq)?;

    let packet_type = header.packet_type;
    let payload_len = packet_type.payload_len();
    // Protege contra um `header` que não veio deste `packet`.
    if packet.len() != packet_type.packet_len() {
        return Err(ProtoError::BadLength);
    }

    let mut plain = [0u8; MAX_PAYLOAD_LEN];
    let plain = &mut plain[..payload_len];
    plain.copy_from_slice(&packet[HEADER_LEN..HEADER_LEN + payload_len]);
    let tag =
        Tag::try_from(&packet[HEADER_LEN + payload_len..]).map_err(|_| ProtoError::BadLength)?;
    debug_assert_eq!(tag.len(), TAG_LEN);

    cipher
        .0
        .decrypt_inout_detached(
            &nonce(packet_type.direction(), header.seq),
            &packet[..HEADER_LEN],
            (&mut *plain).into(),
            &tag,
        )
        .map_err(|_| ProtoError::BadTag)?;

    let payload = Payload::read(packet_type, plain)?;
    replay.commit(header.seq);
    Ok(Packet {
        session_id: header.session_id,
        seq: header.seq,
        payload,
    })
}
