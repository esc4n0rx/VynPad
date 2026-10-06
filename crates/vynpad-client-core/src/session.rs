//! Sessão UDP do client.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vynpad_protocol::{
    KEY_LEN, MAX_PACKET_LEN, Payload, Ping, Role, Secret, Session, packet::InputState,
};

use crate::{CoreError, GamepadInput, MotionSample, ReceivedPacket};

/// Sessão criptografada com o server, criada com os dados do `welcome` do canal de controle.
///
/// É segura entre threads: o envio (laço de input) e a recepção (rumble, pong) podem rodar em
/// paralelo; cada chamada leva o lock só pelo tempo de selar ou abrir um pacote.
#[derive(uniffi::Object)]
pub struct ClientSession {
    inner: Mutex<Session>,
}

impl ClientSession {
    fn session(&self) -> MutexGuard<'_, Session> {
        // Um panic em outra thread não corrompe o estado da sessão: seguimos com ele.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn seal(&self, payload: &Payload) -> Result<Vec<u8>, CoreError> {
        let mut buf = [0u8; MAX_PACKET_LEN];
        let len = self.session().send(payload, &mut buf)?;
        Ok(buf[..len].to_vec())
    }
}

#[uniffi::export]
impl ClientSession {
    /// Cria a sessão. `key` são os 32 bytes da chave de sessão do `welcome`.
    #[uniffi::constructor]
    pub fn new(session_id: u32, key: Vec<u8>) -> Result<Arc<Self>, CoreError> {
        let key: [u8; KEY_LEN] =
            key.try_into()
                .map_err(|k: Vec<u8>| CoreError::InvalidArgument {
                    reason: format!("a chave deve ter {KEY_LEN} bytes, recebeu {}", k.len()),
                })?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Session::new(Role::Client, session_id, &Secret::new(key))),
        }))
    }

    /// `session_id` desta sessão.
    pub fn session_id(&self) -> u32 {
        self.session().session_id()
    }

    /// `true` quando o `seq` de envio acabou e a sessão precisa ser renovada.
    pub fn is_exhausted(&self) -> bool {
        self.session().is_exhausted()
    }

    /// Sela o estado do controle (`0x01`) pronto para o socket UDP.
    pub fn encode_input(&self, input: GamepadInput) -> Result<Vec<u8>, CoreError> {
        self.seal(&Payload::Input(input.into()))
    }

    /// Sela o estado do controle com os sensores de movimento (`0x02`).
    pub fn encode_input_motion(
        &self,
        input: GamepadInput,
        motion: MotionSample,
    ) -> Result<Vec<u8>, CoreError> {
        let state: InputState = input.into();
        self.seal(&Payload::InputMotion(state, motion.into()))
    }

    /// Sela um ping (`0x10`) com o relógio do client em µs.
    pub fn encode_ping(&self, t_client_us: i64) -> Result<Vec<u8>, CoreError> {
        self.seal(&Payload::Ping(Ping { t_client_us }))
    }

    /// Valida e abre um datagrama do server (todos os passos da §6.5).
    pub fn receive(&self, datagram: &[u8]) -> Result<ReceivedPacket, CoreError> {
        let packet = self.session().receive(datagram)?;
        packet.payload.try_into()
    }
}
