//! Estado de uma sessão UDP: contador de envio, anti-replay e a combinação dos dois.

use crate::crypto::{KEY_LEN, SessionCipher, open_packet, seal_packet};
use crate::error::ProtoError;
use crate::packet::{Packet, Payload, Role, parse_header};
use crate::secret::Secret;

/// Anti-replay da recepção (PROTOCOLO §6.3): aceita só `seq` maior que o último aceito.
///
/// Não existe wrap-around: depois de `0xFFFFFFFF` nada mais é aceito, e a sessão
/// precisa ser renovada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReplayWindow {
    last: Option<u32>,
}

impl ReplayWindow {
    /// Janela vazia (nenhum pacote aceito ainda).
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// Janela que já aceitou até `last` (útil em testes e em diagnósticos).
    pub const fn with_last(last: Option<u32>) -> Self {
        Self { last }
    }

    /// Último `seq` aceito.
    pub const fn last(&self) -> Option<u32> {
        self.last
    }

    /// Passo 8 da §6.5: `seq` precisa ser maior que o último aceito.
    pub const fn check(&self, seq: u32) -> Result<(), ProtoError> {
        match self.last {
            Some(last) if seq <= last => Err(ProtoError::StaleSeq),
            _ => Ok(()),
        }
    }

    /// Registra `seq` como aceito. Chame só depois de o pacote passar por todas as checagens.
    pub fn commit(&mut self, seq: u32) {
        debug_assert!(self.check(seq).is_ok());
        self.last = Some(seq);
    }
}

/// Contador de `seq` de envio: começa em 0 e esgota depois de `0xFFFFFFFF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeqCounter {
    next: Option<u32>,
}

impl SeqCounter {
    /// Contador novo (o primeiro `seq` é 0).
    pub const fn new() -> Self {
        Self { next: Some(0) }
    }

    /// Consome e devolve o próximo `seq`, ou [`ProtoError::SeqExhausted`] se acabou.
    pub fn next_seq(&mut self) -> Result<u32, ProtoError> {
        let seq = self.next.ok_or(ProtoError::SeqExhausted)?;
        self.next = seq.checked_add(1);
        Ok(seq)
    }

    /// `true` depois que o último `seq` utilizável foi consumido.
    pub const fn is_exhausted(&self) -> bool {
        self.next.is_none()
    }
}

impl Default for SeqCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Uma sessão UDP completa de um lado só: cifra, contador de envio e anti-replay.
///
/// Serve para o client (e o simulador), que têm uma sessão por vez. O server, que
/// procura a sessão pelo `session_id`, usa [`parse_header`] e depois
/// [`Session::receive_parsed`].
#[derive(Debug, Clone)]
pub struct Session {
    role: Role,
    id: u32,
    cipher: SessionCipher,
    tx: SeqCounter,
    rx: ReplayWindow,
}

impl Session {
    /// Cria a sessão com os dados do `welcome`.
    pub fn new(role: Role, session_id: u32, key: &Secret<[u8; KEY_LEN]>) -> Self {
        Self {
            role,
            id: session_id,
            cipher: SessionCipher::new(key),
            tx: SeqCounter::new(),
            rx: ReplayWindow::new(),
        }
    }

    /// `session_id` da sessão.
    pub const fn session_id(&self) -> u32 {
        self.id
    }

    /// Papel deste lado.
    pub const fn role(&self) -> Role {
        self.role
    }

    /// `true` quando o `seq` de envio acabou e a sessão precisa ser renovada.
    pub const fn is_exhausted(&self) -> bool {
        self.tx.is_exhausted()
    }

    /// Sela `payload` com o próximo `seq` em `out` e devolve o tamanho do pacote.
    ///
    /// Recusa com [`ProtoError::WrongDirection`] um payload que este papel não envia.
    pub fn send(&mut self, payload: &Payload, out: &mut [u8]) -> Result<usize, ProtoError> {
        if payload.packet_type().direction() != self.role.sends() {
            return Err(ProtoError::WrongDirection);
        }
        let len = payload.packet_type().packet_len();
        if out.len() < len {
            return Err(ProtoError::BufferTooSmall);
        }
        let seq = self.tx.next_seq()?;
        seal_packet(&self.cipher, self.id, seq, payload, out)
    }

    /// Valida e abre um datagrama recebido (todos os passos da §6.5).
    pub fn receive(&mut self, packet: &[u8]) -> Result<Packet, ProtoError> {
        let header = parse_header(packet, self.role)?;
        self.receive_parsed(&header, packet)
    }

    /// Passos 7 a 10 para um cabeçalho que já passou por [`parse_header`].
    pub fn receive_parsed(
        &mut self,
        header: &crate::PacketHeader,
        packet: &[u8],
    ) -> Result<Packet, ProtoError> {
        if header.session_id != self.id {
            return Err(ProtoError::UnknownSession);
        }
        open_packet(&self.cipher, &mut self.rx, header, packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{InputState, MAX_PACKET_LEN, Ping, Rumble};

    fn pair() -> (Session, Session) {
        let key = Secret::new([7u8; KEY_LEN]);
        (
            Session::new(Role::Client, 99, &key),
            Session::new(Role::Server, 99, &key),
        )
    }

    #[test]
    fn ida_e_volta() {
        let (mut client, mut server) = pair();
        let mut buf = [0u8; MAX_PACKET_LEN];

        let input = Payload::Input(InputState {
            lx: -5,
            ..InputState::default()
        });
        let n = client.send(&input, &mut buf).unwrap();
        let got = server.receive(&buf[..n]).unwrap();
        assert_eq!((got.seq, got.payload), (0, input));

        let rumble = Payload::Rumble(Rumble {
            rumble_id: 1,
            large: 9,
            small: 0,
        });
        let n = server.send(&rumble, &mut buf).unwrap();
        assert_eq!(client.receive(&buf[..n]).unwrap().payload, rumble);
    }

    #[test]
    fn replay_e_recusado_e_nao_avanca_a_janela() {
        let (mut client, mut server) = pair();
        let mut buf = [0u8; MAX_PACKET_LEN];
        let n = client
            .send(&Payload::Ping(Ping::default()), &mut buf)
            .unwrap();
        server.receive(&buf[..n]).unwrap();
        assert_eq!(server.receive(&buf[..n]), Err(ProtoError::StaleSeq));

        // Pacote adulterado com seq novo não pode "queimar" o seq legítimo.
        let n = client
            .send(&Payload::Ping(Ping::default()), &mut buf)
            .unwrap();
        let mut forged = buf;
        forged[n - 1] ^= 1;
        assert_eq!(server.receive(&forged[..n]), Err(ProtoError::BadTag));
        assert!(server.receive(&buf[..n]).is_ok());
    }

    #[test]
    fn nao_envia_na_direcao_errada() {
        let (mut client, _) = pair();
        let mut buf = [0u8; MAX_PACKET_LEN];
        let rumble = Payload::Rumble(Rumble::default());
        assert_eq!(
            client.send(&rumble, &mut buf),
            Err(ProtoError::WrongDirection)
        );
    }

    #[test]
    fn buffer_pequeno() {
        let (mut client, _) = pair();
        let mut buf = [0u8; 51];
        let input = Payload::Input(InputState::default());
        assert_eq!(
            client.send(&input, &mut buf),
            Err(ProtoError::BufferTooSmall)
        );
        // O seq não foi consumido.
        let mut buf = [0u8; MAX_PACKET_LEN];
        client.send(&input, &mut buf).unwrap();
        assert_eq!(&buf[8..12], &0u32.to_le_bytes());
    }

    #[test]
    fn cabecalho_de_outro_datagrama_nao_causa_panico() {
        let (mut client, _) = pair();
        let mut buf = [0u8; MAX_PACKET_LEN];
        let n = client
            .send(&Payload::Input(InputState::default()), &mut buf)
            .unwrap();
        let header = parse_header(&buf[..n], Role::Server).unwrap();
        let cipher = SessionCipher::new(&Secret::new([7u8; KEY_LEN]));
        let mut replay = ReplayWindow::new();
        let short = &buf[..36];
        assert_eq!(
            open_packet(&cipher, &mut replay, &header, short),
            Err(ProtoError::BadLength)
        );
    }

    #[test]
    fn seq_esgota_sem_wrap_around() {
        let mut c = SeqCounter {
            next: Some(u32::MAX - 1),
        };
        assert_eq!(c.next_seq(), Ok(u32::MAX - 1));
        assert_eq!(c.next_seq(), Ok(u32::MAX));
        assert!(c.is_exhausted());
        assert_eq!(c.next_seq(), Err(ProtoError::SeqExhausted));

        let w = ReplayWindow::with_last(Some(u32::MAX));
        assert_eq!(w.check(0), Err(ProtoError::StaleSeq));
    }
}
