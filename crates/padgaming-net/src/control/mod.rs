//! Servidor de controle TLS (PROTOCOLO §5, T-302).
//!
//! Aceita as conexões dos clients na porta TCP 47800, autentica (`hello` → [`PairingService`]),
//! aloca o slot, cria a sessão UDP (`session_id` + `udp_key`, registrada no
//! [`InputServer`](crate::input::InputServer)) e responde `welcome`. Depois mantém o keepalive
//! e distribui `config`/`bye`.
//!
//! O que fica de fora: o ciclo de vida completo da sessão (suspensão por silêncio do UDP e o
//! `tick` da janela de reconexão, T-304), a UI de pareamento (T-603) e o rumble (T-204).
//! Aqui, a queda do TLS suspende o slot ([`SlotManager::suspend`]) e o `bye` do client o libera.
//!
//! [`PairingService`]: crate::pairing::PairingService
//! [`SlotManager::suspend`]: padgaming_core::SlotManager::suspend

mod connection;
mod server;

pub use server::{ControlDeps, ControlServer};

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use padgaming_driver::SlotId;
use padgaming_mapping::{CompiledProfile, Profile, ProfileCell};
use padgaming_protocol::control::{Config, Uuid};
use tokio::sync::oneshot;

/// Porta TCP de controle (PROTOCOLO §2).
pub const DEFAULT_PORT: u16 = 47800;

/// Versões do protocolo aceitas por este server.
pub const SUPPORTED_VERSIONS: [u8; 1] = [1];

/// Parâmetros do servidor de controle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlConfig {
    /// Endereço de escuta. O padrão é `0.0.0.0:47800`.
    pub bind: SocketAddr,
    /// Conexões que ainda não completaram o handshake (TLS + `hello` + auth). Além disso,
    /// as novas são fechadas na hora.
    pub max_pending: usize,
    /// Prazo para o TLS e o `hello` chegarem.
    pub hello_timeout: Duration,
    /// Sem enviar nada por este tempo, manda `ping`.
    pub ping_after: Duration,
    /// Sem receber nada por este tempo, fecha a conexão.
    pub dead_after: Duration,
    /// Quanto o usuário tem para responder ao pedido de confirmação de um novo dispositivo.
    pub approval_timeout: Duration,
    /// Prazo de cada escrita no socket (um client que não lê não trava a sessão).
    pub write_timeout: Duration,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT)),
            max_pending: 16,
            hello_timeout: Duration::from_secs(10),
            ping_after: Duration::from_secs(5),
            dead_after: Duration::from_secs(15),
            approval_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(5),
        }
    }
}

/// O que o slot recebe ao ser ocupado por um dispositivo.
pub struct SlotSetup {
    /// Perfil de mapeamento do slot, compartilhado com a UI.
    pub profile: Arc<ProfileCell>,
    /// Configuração enviada ao client no `welcome`. `config.slot` é preenchido pelo server.
    pub config: Config,
}

/// Fornece o perfil e a configuração de cada slot (na T-403/T-604 vêm do SQLite).
pub trait SlotProfiles: Send + Sync {
    /// Chamado a cada sessão nova do `device` no `slot`.
    fn setup(&self, slot: SlotId, device: &Uuid) -> SlotSetup;
}

/// [`SlotProfiles`] com o perfil "Padrão" em cada slot e a configuração base.
pub struct DefaultProfiles {
    cells: [Arc<ProfileCell>; 4],
    base: Config,
}

impl DefaultProfiles {
    /// Cria os 4 perfis. `base.slot` é ignorado.
    ///
    /// # Panics
    /// Nunca com o perfil embutido; ele é válido por construção.
    pub fn new(base: Config) -> Self {
        let compiled = || {
            Arc::new(ProfileCell::new(
                CompiledProfile::compile(&Profile::standard()).expect("perfil padrão é válido"),
            ))
        };
        Self {
            cells: [compiled(), compiled(), compiled(), compiled()],
            base,
        }
    }

    /// O perfil do slot, para a UI trocá-lo.
    pub fn cell(&self, slot: SlotId) -> &Arc<ProfileCell> {
        &self.cells[usize::from(slot.index())]
    }
}

impl Default for DefaultProfiles {
    fn default() -> Self {
        Self::new(Config {
            slot: 0,
            send_rate_hz: 250,
            gyro_enabled: false,
            rumble_enabled: false,
            touch_haptics: true,
        })
    }
}

impl SlotProfiles for DefaultProfiles {
    fn setup(&self, slot: SlotId, _device: &Uuid) -> SlotSetup {
        SlotSetup {
            profile: Arc::clone(self.cell(slot)),
            config: self.base,
        }
    }
}

/// Pedido de confirmação de um novo dispositivo (opção "Pedir confirmação para novos
/// dispositivos"). Só chega depois de a credencial (QR/PIN) ter sido aceita.
#[derive(Debug)]
pub struct ApprovalRequest {
    /// Dispositivo que quer parear.
    pub device_id: Uuid,
    /// Nome exibido.
    pub name: String,
    /// Modelo.
    pub model: String,
    /// IP de origem.
    pub peer: IpAddr,
    /// A UI responde `true` para aceitar. Sem resposta em 30 s, ou se descartado, recusa.
    pub reply: oneshot::Sender<bool>,
}

/// Como uma sessão terminou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// O TLS caiu. O slot fica reservado para a reconexão.
    Dropped,
    /// 15 s sem nada no canal de controle. O slot fica reservado.
    Timeout,
    /// O client mandou `bye`: o slot é liberado.
    ClientBye,
    /// Desconectado pela UI, ou substituído por uma conexão nova do mesmo dispositivo.
    Kicked,
    /// Dispositivo revogado.
    Revoked,
    /// O server está encerrando.
    Shutdown,
}

/// Eventos para a UI (`devices:paired` e afins na T-506).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlEvent {
    /// Primeiro pareamento concluído; o dispositivo já foi persistido.
    Paired {
        /// Dispositivo.
        device_id: Uuid,
        /// Nome exibido.
        name: String,
        /// Modelo.
        model: String,
    },
    /// O `welcome` foi enviado.
    SessionStarted {
        /// Sessão UDP.
        session_id: u32,
        /// Slot ocupado.
        slot: SlotId,
        /// Dispositivo.
        device_id: Uuid,
    },
    /// A sessão terminou.
    SessionEnded {
        /// Sessão UDP.
        session_id: u32,
        /// Slot que ela ocupava.
        slot: SlotId,
        /// Dispositivo.
        device_id: Uuid,
        /// Motivo.
        reason: SessionEnd,
    },
}
