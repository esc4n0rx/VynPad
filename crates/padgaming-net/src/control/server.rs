//! O listener e o estado compartilhado do servidor de controle.

use std::collections::{HashMap, HashSet};
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use padgaming_core::SlotManager;
use padgaming_driver::SlotId;
use padgaming_protocol::control::{ByeReason, Config, Uuid};
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use super::connection;
use super::{ApprovalRequest, ControlConfig, ControlEvent, SessionHealth, SlotProfiles, watchdog};
use crate::identity::{IdentityError, ServerIdentity};
use crate::input::{InputServer, SessionActivity};
use crate::pairing::PairingService;

/// Capacidade do canal de eventos; quem ficar para trás perde os mais antigos.
const EVENT_BACKLOG: usize = 64;
/// Quanto o `shutdown` espera as sessões se despedirem.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// Tudo de que o servidor de controle depende.
pub struct ControlDeps {
    /// Certificado e `server_id`.
    pub identity: ServerIdentity,
    /// Nome do server, mostrado no client (1–64 caracteres).
    pub server_name: String,
    /// Autenticação e pareamento.
    pub pairing: Arc<dyn PairingService>,
    /// Os 4 controles virtuais.
    pub slots: Arc<SlotManager>,
    /// O loop UDP, onde as sessões são registradas.
    pub input: Arc<InputServer>,
    /// Perfil e configuração de cada slot.
    pub profiles: Arc<dyn SlotProfiles>,
}

/// Comando para uma conexão em andamento.
pub(super) enum Command {
    /// Enviar a configuração completa.
    Config(Config),
    /// Despedir-se com `bye` e fechar.
    Bye(ByeReason),
}

/// Uma sessão viva, como o coletor de métricas a enxerga.
pub struct LiveSession {
    /// Id da sessão.
    pub session_id: u32,
    /// Slot em que está agora.
    pub slot: SlotId,
    /// Dispositivo dono da sessão.
    pub device: Uuid,
    /// Atividade e métricas da sessão.
    pub activity: Arc<SessionActivity>,
}

/// Uma sessão viva no registro.
pub(super) struct Live {
    pub device: Uuid,
    pub slot: SlotId,
    pub commands: mpsc::UnboundedSender<Command>,
    /// Quando chegou o último pacote UDP da sessão.
    pub activity: Arc<SessionActivity>,
    /// Último estado publicado pelo watchdog.
    pub health: SessionHealth,
}

/// Estado compartilhado entre o listener e as conexões.
pub(super) struct Shared {
    pub config: ControlConfig,
    pub server_id: Uuid,
    pub server_name: String,
    pub udp_port: u16,
    pub pairing: Arc<dyn PairingService>,
    pub slots: Arc<SlotManager>,
    pub input: Arc<InputServer>,
    pub profiles: Arc<dyn SlotProfiles>,
    pub events: broadcast::Sender<ControlEvent>,
    pub shutdown: watch::Sender<bool>,
    pub require_approval: AtomicBool,
    approver: Mutex<Option<mpsc::Sender<ApprovalRequest>>>,
    live: Mutex<HashMap<u32, Live>>,
    /// `session_id`s já sorteados por um handshake que ainda não entrou em `live` (o `setup` do
    /// perfil é assíncrono e o lock de `live` não pode atravessar um `await`). Ordem dos locks:
    /// `live`, depois `reserved`.
    reserved: Mutex<HashSet<u32>>,
}

impl Shared {
    pub fn live(&self) -> MutexGuard<'_, HashMap<u32, Live>> {
        self.live.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn reserved(&self) -> MutexGuard<'_, HashSet<u32>> {
        self.reserved.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn approver(&self) -> Option<mpsc::Sender<ApprovalRequest>> {
        self.approver
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn emit(&self, event: ControlEvent) {
        // Sem ouvintes não é erro.
        let _ = self.events.send(event);
    }
}

/// O servidor de controle em execução.
pub struct ControlServer {
    shared: Arc<Shared>,
    local_addr: SocketAddr,
    accept: Mutex<Option<JoinHandle<()>>>,
    watchdog: Mutex<Option<JoinHandle<()>>>,
}

impl ControlServer {
    /// Abre o listener e começa a aceitar conexões. Precisa de um runtime Tokio.
    pub async fn start(config: ControlConfig, deps: ControlDeps) -> Result<Self, StartError> {
        let acceptor = TlsAcceptor::from(deps.identity.tls_config()?);
        let listener = TcpListener::bind(config.bind).await?;
        let local_addr = listener.local_addr()?;
        let (events, _) = broadcast::channel(EVENT_BACKLOG);
        let (shutdown, _) = watch::channel(false);
        let shared = Arc::new(Shared {
            config,
            server_id: deps.identity.server_id().clone(),
            server_name: deps.server_name,
            udp_port: deps.input.local_addr().port(),
            pairing: deps.pairing,
            slots: deps.slots,
            input: deps.input,
            profiles: deps.profiles,
            events,
            shutdown,
            require_approval: AtomicBool::new(false),
            approver: Mutex::new(None),
            live: Mutex::new(HashMap::new()),
            reserved: Mutex::new(HashSet::new()),
        });
        let accept = tokio::spawn(accept_loop(
            listener,
            acceptor,
            Arc::clone(&shared),
            Arc::new(Semaphore::new(config.max_pending)),
        ));
        let watchdog = tokio::spawn(watchdog::run(Arc::clone(&shared)));
        tracing::info!(%local_addr, "servidor de controle TLS iniciado");
        Ok(Self {
            shared,
            local_addr,
            accept: Mutex::new(Some(accept)),
            watchdog: Mutex::new(Some(watchdog)),
        })
    }

    /// Endereço em que o listener está (útil com a porta 0, nos testes).
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Eventos para a UI. Quem não acompanha o ritmo perde os mais antigos.
    pub fn subscribe(&self) -> broadcast::Receiver<ControlEvent> {
        self.shared.events.subscribe()
    }

    /// Liga ou desliga "Pedir confirmação para novos dispositivos".
    ///
    /// Ligado, todo primeiro pareamento vira um [`ApprovalRequest`] para o destino definido em
    /// [`Self::set_approver`]; sem destino, os novos dispositivos são recusados.
    pub fn set_require_approval(&self, on: bool) {
        self.shared.require_approval.store(on, Ordering::Relaxed);
    }

    /// Para onde vão os pedidos de confirmação (a ponte com a UI).
    pub fn set_approver(&self, approver: Option<mpsc::Sender<ApprovalRequest>>) {
        *self
            .shared
            .approver
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = approver;
    }

    /// Manda a configuração completa à sessão do `slot` (ex.: o usuário ligou o gyro).
    /// `config.slot` é ignorado. Devolve `false` se o slot não tem sessão viva.
    pub fn send_config(&self, slot: SlotId, mut config: Config) -> bool {
        config.slot = slot.index();
        self.shared
            .live()
            .values()
            .find(|l| l.slot == slot)
            .is_some_and(|l| l.commands.send(Command::Config(config)).is_ok())
    }

    /// As sessões vivas com o slot atual de cada uma, para o coletor de métricas (T-305).
    pub fn live_sessions(&self) -> Vec<LiveSession> {
        self.shared
            .live()
            .iter()
            .map(|(id, live)| LiveSession {
                session_id: *id,
                slot: live.slot,
                device: live.device.clone(),
                activity: Arc::clone(&live.activity),
            })
            .collect()
    }

    /// Troca o slot das sessões vivas de `a` e `b` (a UI trocou os controles de lugar). Quem
    /// chama manda o `config` novo com [`Self::send_config`].
    pub fn relocate(&self, a: SlotId, b: SlotId) {
        for live in self.shared.live().values_mut() {
            if live.slot == a {
                live.slot = b;
            } else if live.slot == b {
                live.slot = a;
            }
        }
    }

    /// Encerra a sessão com `bye` (`kicked`, `revoked`...). O slot é liberado.
    /// Devolve `false` se a sessão não existe.
    pub fn end_session(&self, session_id: u32, reason: ByeReason) -> bool {
        self.shared
            .live()
            .get(&session_id)
            .is_some_and(|l| l.commands.send(Command::Bye(reason)).is_ok())
    }

    /// O sistema retomou da suspensão (o app avisa, T-607): solta todos os controles, pois
    /// nada do que estava em andamento é confiável, e revalida o driver (evento
    /// [`ControlEvent::DriverRechecked`]). As sessões que voltarem a mandar input retomam sozinhas.
    pub fn system_resumed(&self) {
        watchdog::resumed(&self.shared);
    }

    /// Número de sessões vivas.
    pub fn session_count(&self) -> usize {
        self.shared.live().len()
    }

    /// Para de aceitar, manda `bye{shutdown}` às sessões e espera elas terminarem (até 2 s).
    pub async fn shutdown(self) {
        self.stop().await;
    }

    /// Como [`Self::shutdown`], sem consumir o servidor (quem o guarda num `Arc` compartilhado).
    /// Chamar de novo não faz nada.
    pub async fn stop(&self) {
        let _ = self.shared.shutdown.send(true);
        let tasks = [
            self.accept
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
            self.watchdog
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        ];
        for task in tasks.into_iter().flatten() {
            let _ = task.await;
        }
        let deadline = tokio::time::Instant::now() + SHUTDOWN_GRACE;
        while self.session_count() > 0 && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tracing::info!("servidor de controle encerrado");
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        // Sem `shutdown` explícito: as conexões se despedem sozinhas e o listener fecha.
        let _ = self.shared.shutdown.send(true);
    }
}

/// Falha ao iniciar o servidor.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// Não consegui abrir a porta.
    #[error("não consegui abrir a porta de controle: {0}")]
    Bind(#[from] io::Error),
    /// Configuração TLS.
    #[error(transparent)]
    Tls(#[from] IdentityError),
}

async fn accept_loop(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    shared: Arc<Shared>,
    pending: Arc<Semaphore>,
) {
    let mut stop = shared.shutdown.subscribe();
    loop {
        let (stream, peer) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(pair) => pair,
                Err(error) => {
                    // Falta de descritores e afins: espera um pouco para não girar em falso.
                    tracing::warn!(%error, "falha no accept do canal de controle");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            },
            _ = stop.changed() => break,
        };
        let Ok(permit) = Arc::clone(&pending).try_acquire_owned() else {
            tracing::warn!(%peer, "conexões pendentes demais: recusada");
            continue; // `stream` cai aqui e fecha o socket.
        };
        let _ = stream.set_nodelay(true);
        tokio::spawn(connection::run(
            Arc::clone(&shared),
            acceptor.clone(),
            stream,
            peer,
            permit,
        ));
    }
}
