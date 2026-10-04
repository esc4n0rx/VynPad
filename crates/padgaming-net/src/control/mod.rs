//! Servidor de controle TLS (PROTOCOLO §5, T-302).
//!
//! Aceita as conexões dos clients na porta TCP 47800, autentica (`hello` → [`PairingService`]),
//! aloca o slot, cria a sessão UDP (`session_id` + `udp_key`, registrada no
//! [`InputServer`](crate::input::InputServer)) e responde `welcome`. Depois mantém o keepalive
//! e distribui `config`/`bye`.
//!
//! Ciclo de vida (T-304): a queda do TLS suspende o slot ([`SlotManager::suspend`], reserva de
//! 30 s) e o `bye` do client o libera. Um watchdog de 50 ms cuida do silêncio do UDP (250 ms →
//! controle neutro, 5 s → "reconectando") e libera as reservas vencidas (`SlotManager::tick`).
//!
//! O que fica de fora: a UI de pareamento (T-603) e o rumble (T-204).
//!
//! [`PairingService`]: crate::pairing::PairingService
//! [`SlotManager::suspend`]: padgaming_core::SlotManager::suspend

mod connection;
mod server;
mod watchdog;

pub use server::{ControlDeps, ControlServer, LiveSession, StartError};

use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use padgaming_driver::SlotId;
use padgaming_mapping::{CompiledProfile, Profile, ProfileCell};
use padgaming_protocol::control::{Config, RejectReason, Uuid};
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
    /// Quantas dessas vagas um mesmo endereço pode ocupar (T-706): um aparelho na LAN abrindo
    /// conexões e ficando mudo não pode esgotar as vagas dos outros.
    pub max_pending_per_ip: usize,
    /// Prazo para o TLS e o `hello` chegarem.
    pub hello_timeout: Duration,
    /// Sem enviar nada por este tempo, manda `ping`.
    pub ping_after: Duration,
    /// Sem receber nada por este tempo, fecha a conexão.
    pub dead_after: Duration,
    /// De quanto em quanto tempo o server manda um `ping` só para medir o RTT (T-305).
    pub rtt_probe_every: Duration,
    /// Quanto o usuário tem para responder ao pedido de confirmação de um novo dispositivo.
    pub approval_timeout: Duration,
    /// Prazo de cada escrita no socket (um client que não lê não trava a sessão).
    pub write_timeout: Duration,
    /// Sem pacote UDP válido por este tempo, o controle vai para o estado neutro (PROTOCOLO §9).
    pub neutral_after: Duration,
    /// Sem pacote UDP válido por este tempo, a sessão aparece como "reconectando".
    pub stalled_after: Duration,
    /// Intervalo do watchdog.
    pub watchdog_interval: Duration,
    /// De quanto em quanto tempo o watchdog libera as reservas de slot vencidas.
    pub reservation_check: Duration,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT)),
            max_pending: 16,
            max_pending_per_ip: 4,
            hello_timeout: Duration::from_secs(10),
            ping_after: Duration::from_secs(5),
            dead_after: Duration::from_secs(15),
            rtt_probe_every: Duration::from_millis(250),
            approval_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(5),
            neutral_after: Duration::from_millis(250),
            stalled_after: Duration::from_secs(5),
            watchdog_interval: Duration::from_millis(50),
            reservation_check: Duration::from_secs(1),
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

/// O futuro devolvido por [`SlotProfiles::preferred_slot`].
pub type PreferredFuture<'a> = Pin<Box<dyn Future<Output = Option<SlotId>> + Send + 'a>>;

/// O futuro devolvido por [`SlotProfiles::setup`].
pub type SetupFuture<'a> = Pin<Box<dyn Future<Output = SlotSetup> + Send + 'a>>;

/// Fornece o perfil e a configuração de cada slot (vêm do SQLite, então a consulta é assíncrona).
pub trait SlotProfiles: Send + Sync {
    /// Slot em que o `device` prefere ficar (o `assign` o usa se estiver livre). O padrão é não ter
    /// preferência.
    fn preferred_slot<'a>(&'a self, _device: &'a Uuid) -> PreferredFuture<'a> {
        Box::pin(async { None })
    }

    /// Chamado a cada sessão nova do `device` no `slot`, antes do `welcome`.
    fn setup<'a>(&'a self, slot: SlotId, device: &'a Uuid) -> SetupFuture<'a>;
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
    fn setup<'a>(&'a self, slot: SlotId, _device: &'a Uuid) -> SetupFuture<'a> {
        Box::pin(async move {
            SlotSetup {
                profile: Arc::clone(self.cell(slot)),
                config: self.base,
            }
        })
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

/// Saúde do fluxo de input UDP de uma sessão (`session:state` na UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionHealth {
    /// Chegando pacotes.
    Live,
    /// 250 ms sem pacotes: o controle virtual foi solto (continua plugado).
    Neutral,
    /// 5 s sem pacotes: a UI mostra "Reconectando…". O slot continua da sessão.
    Stalled,
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
        /// IP de origem da conexão.
        peer: IpAddr,
    },
    /// Uma conexão foi recusada (`reject`).
    Rejected {
        /// IP de origem.
        peer: IpAddr,
        /// Dispositivo que se anunciou no `hello` (`None` se a conexão nem chegou a um `hello`
        /// válido). Pode ser um dispositivo desconhecido do server: quem consulta deve conferir.
        device_id: Option<Uuid>,
        /// Motivo enviado ao client.
        reason: RejectReason,
    },
    /// O fluxo de input da sessão mudou de estado.
    SessionState {
        /// Sessão UDP.
        session_id: u32,
        /// Slot da sessão.
        slot: SlotId,
        /// Novo estado.
        health: SessionHealth,
    },
    /// A reserva do slot venceu (o dispositivo não voltou em 30 s): controle desplugado.
    ReservationExpired {
        /// Slot liberado.
        slot: SlotId,
    },
    /// O driver foi reavaliado depois de o sistema retomar da suspensão.
    DriverRechecked(padgaming_driver::DriverStatus),
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
