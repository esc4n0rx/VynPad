//! O `Core`: monta e opera o server (driver, slots, rede, descoberta, persistência, pareamento).

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle as ThreadHandle;
use std::time::Duration;

use padgaming_core::{SlotManager, SystemClock};
use padgaming_driver::{DriverStatus, SlotId, VirtualPadDriver, default_driver};
use padgaming_net::control::{
    ApprovalRequest, ControlDeps, ControlEvent, ControlServer, SessionEnd,
};
use padgaming_net::discovery::{
    Discovery, DiscoveryConfig, LanInterface, NearbyDevice, ServerInfo,
};
use padgaming_net::identity::ServerIdentity;
use padgaming_net::input::{InputConfig, InputServer};
use padgaming_net::pairing::PairingManager;
use padgaming_net::qr::pairing_qr_svg;
use padgaming_protocol::Secret;
use padgaming_protocol::control::{ByeReason, Uuid};
use padgaming_protocol::discovery::short_fingerprint;
use padgaming_protocol::pairing_uri::PairingUri;
use padgaming_storage::seed::now_ms;
use padgaming_storage::{
    ConnectionLog, Database, DeviceDefaults, DeviceRepo, LogEvent, Ports, ProfileService,
    SettingsRepo, SettingsService,
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::config::{CoreConfig, computer_name};
use crate::error::CoreError;
use crate::event::{ApprovalEvent, ApprovalPrompt, CoreEvent};
use crate::registry::SlotRegistry;
use crate::store::{DeviceStoreAdapter, PairedSet};

/// Capacidade do canal de eventos; quem ficar para trás perde os mais antigos.
const EVENT_BACKLOG: usize = 256;
/// Pedidos de confirmação que podem esperar na fila.
const APPROVAL_QUEUE: usize = 8;
/// Intervalo com que as threads de fundo conferem o pedido de parada.
const POLL: Duration = Duration::from_millis(200);

/// O que a UI mostra para parear um celular: QR code e PIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingOffer {
    /// URI do QR code (`padgaming://pair?...`).
    pub uri: String,
    /// O QR code em SVG.
    pub qr_svg: String,
    /// PIN de 6 dígitos (pareamento sem câmera).
    pub pin: String,
    /// Validade do QR code.
    pub token_ttl: Duration,
    /// Validade do PIN.
    pub pin_ttl: Duration,
    /// Os 8 primeiros bytes do fingerprint, em hex: o usuário confere com a tela do celular ao
    /// parear por PIN.
    pub fingerprint_short: String,
    /// Endereços IPv4 do PC anunciados no QR code.
    pub hosts: Vec<Ipv4Addr>,
    /// Porta TCP de controle.
    pub port: u16,
}

/// As portas em que o server está escutando de fato.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundPorts {
    /// Canal de controle (TCP).
    pub control: u16,
    /// Input (UDP).
    pub input: u16,
    /// Descoberta por broadcast (UDP).
    pub discovery: u16,
}

/// O estado que as tarefas de fundo compartilham.
struct Shared {
    events: broadcast::Sender<CoreEvent>,
    devices: DeviceRepo,
    log: ConnectionLog,
    paired: Arc<PairedSet>,
    approvals: Mutex<HashMap<u64, oneshot::Sender<bool>>>,
    next_approval: AtomicU64,
}

impl Shared {
    fn emit(&self, event: CoreEvent) {
        // Sem ouvintes não é erro.
        let _ = self.events.send(event);
    }
}

/// O server em execução.
pub struct Core {
    shared: Arc<Shared>,
    db: Database,
    settings: Arc<SettingsService>,
    profiles: Arc<ProfileService>,
    slots: Arc<SlotManager>,
    driver: Arc<dyn VirtualPadDriver>,
    input: Arc<InputServer>,
    control: Arc<ControlServer>,
    discovery: Option<Discovery>,
    pairing: Arc<PairingManager<DeviceStoreAdapter>>,
    identity: ServerIdentity,
    config_pairing: padgaming_net::pairing::PairingConfig,
    ports: BoundPorts,
    stop: Arc<AtomicBool>,
    tasks: Vec<JoinHandle<()>>,
    threads: Vec<ThreadHandle<()>>,
}

impl Core {
    /// Monta e inicia tudo, na ordem certa: banco, configurações, identidade TLS, driver, slots,
    /// loop UDP, servidor de controle e descoberta.
    #[allow(clippy::too_many_lines)] // uma sequência de inicialização; dividir só espalharia
    pub async fn start(config: CoreConfig) -> Result<Self, CoreError> {
        let db = Database::open(&config.data_dir).await?;
        let pool = db.pool().clone();
        let identity = ServerIdentity::load_or_create(&config.data_dir)?;
        let settings = Arc::new(SettingsService::load(SettingsRepo::new(pool.clone())).await?);
        let current = settings.get();

        let driver: Arc<dyn VirtualPadDriver> = config
            .driver
            .clone()
            .unwrap_or_else(|| Arc::from(default_driver()));
        let slots = Arc::new(SlotManager::with_clock(
            Arc::clone(&driver),
            config.slots,
            Arc::new(SystemClock),
        ));

        let devices = DeviceRepo::new(pool.clone());
        let log = ConnectionLog::new(pool.clone());
        let paired = Arc::new(PairedSet::default());
        paired.replace(
            devices
                .list()
                .await?
                .into_iter()
                .filter(|d| !d.revoked)
                .map(|d| d.id),
        );

        let registry = Arc::new(SlotRegistry::new(Arc::clone(&slots), devices.clone()));
        let profiles = Arc::new(ProfileService::new(pool.clone(), registry.clone()));
        registry.connect_profiles(&profiles);

        let ports = config.ports_override.unwrap_or(current.ports);
        let input = Arc::new(InputServer::start(
            InputConfig {
                bind: SocketAddr::new(config.bind_ip, ports.input),
                ..InputConfig::default()
            },
            Arc::clone(&slots),
        )?);

        let adapter = DeviceStoreAdapter::new(
            devices.clone(),
            tokio::runtime::Handle::current(),
            Arc::clone(&paired),
        );
        let pairing = Arc::new(PairingManager::new(adapter, config.pairing));

        let server_name = config.server_name.clone().unwrap_or_else(computer_name);
        let control = Arc::new(
            ControlServer::start(
                padgaming_net::control::ControlConfig {
                    bind: SocketAddr::new(config.bind_ip, ports.control),
                    ..config.control
                },
                ControlDeps {
                    identity: identity.clone(),
                    server_name: server_name.clone(),
                    pairing: pairing.clone(),
                    slots: Arc::clone(&slots),
                    input: Arc::clone(&input),
                    profiles: registry.clone(),
                },
            )
            .await?,
        );
        registry.connect_control(&control);
        control.set_require_approval(current.require_approval);

        let (events, _) = broadcast::channel(EVENT_BACKLOG);
        let shared = Arc::new(Shared {
            events,
            devices: devices.clone(),
            log: log.clone(),
            paired: Arc::clone(&paired),
            approvals: Mutex::new(HashMap::new()),
            next_approval: AtomicU64::new(1),
        });
        let stop = Arc::new(AtomicBool::new(false));

        let (approver, approvals) = mpsc::channel(APPROVAL_QUEUE);
        control.set_approver(Some(approver));

        let info = ServerInfo {
            id: identity.server_id().clone(),
            name: server_name,
            tcp_port: control.local_addr().port(),
            udp_port: input.local_addr().port(),
            os: ServerInfo::current_os().to_owned(),
            free: u8::try_from(slots.snapshot().free_count()).unwrap_or(4),
            fingerprint: identity.fingerprint(),
        };
        let discovery = Discovery::start(
            &DiscoveryConfig {
                interfaces: config.interfaces.clone(),
                broadcast_bind: SocketAddr::new(config.bind_ip, ports.discovery),
                mdns: config.mdns,
            },
            &info,
            {
                let paired = Arc::clone(&paired);
                Arc::new(move |id: &Uuid| paired.contains(id.as_str()))
            },
            {
                let shared = Arc::clone(&shared);
                Arc::new(move |list: Vec<NearbyDevice>| shared.emit(CoreEvent::Nearby(list)))
            },
        )?;
        let follower = discovery.follow_slots(&slots)?;
        let bound = BoundPorts {
            control: info.tcp_port,
            input: info.udp_port,
            discovery: discovery
                .broadcast_addr()
                .map_or(ports.discovery, |a| a.port()),
        };

        let mut core = Self {
            shared: Arc::clone(&shared),
            db,
            settings: Arc::clone(&settings),
            profiles,
            slots: Arc::clone(&slots),
            driver,
            input,
            control: Arc::clone(&control),
            discovery: Some(discovery),
            pairing,
            identity,
            config_pairing: config.pairing,
            ports: bound,
            stop: Arc::clone(&stop),
            tasks: Vec::new(),
            threads: vec![follower],
        };
        core.spawn_pumps(approvals);
        tracing::info!(?bound, "server no ar");
        Ok(core)
    }

    fn spawn_pumps(&mut self, approvals: mpsc::Receiver<ApprovalRequest>) {
        // Eventos do servidor de controle → histórico, "visto por último" e UI.
        self.tasks.push(tokio::spawn(control_pump(
            Arc::clone(&self.shared),
            self.control.subscribe(),
        )));
        // Mudança de configuração → efeito imediato e UI.
        self.tasks.push(tokio::spawn(settings_pump(
            Arc::clone(&self.shared),
            Arc::clone(&self.control),
            self.settings.subscribe(),
        )));
        // Pedidos de confirmação de novos dispositivos.
        self.tasks.push(tokio::spawn(approval_pump(
            Arc::clone(&self.shared),
            approvals,
        )));
        // Mudança nos slots (thread: o `SlotManager` usa canais síncronos).
        let updates = self.slots.subscribe();
        let (shared, stop) = (Arc::clone(&self.shared), Arc::clone(&self.stop));
        if let Ok(thread) = std::thread::Builder::new()
            .name("core-slots".into())
            .spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match updates.recv_timeout(POLL) {
                        Ok(snapshot) => shared.emit(CoreEvent::Slots(snapshot)),
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
        {
            self.threads.push(thread);
        }
    }

    // ------------------------------------------------------------------ consulta

    /// Eventos para a UI. Quem não acompanha o ritmo perde os mais antigos.
    pub fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.shared.events.subscribe()
    }

    /// Configurações (com `settings:changed`).
    pub fn settings(&self) -> &Arc<SettingsService> {
        &self.settings
    }

    /// Perfis de mapeamento (com a troca a quente nos slots).
    pub fn profiles(&self) -> &Arc<ProfileService> {
        &self.profiles
    }

    /// Dispositivos pareados.
    pub fn devices(&self) -> &DeviceRepo {
        &self.shared.devices
    }

    /// Histórico de conexões.
    pub fn log(&self) -> &ConnectionLog {
        &self.shared.log
    }

    /// Os 4 slots.
    pub fn slots(&self) -> &Arc<SlotManager> {
        &self.slots
    }

    /// O loop UDP de input (estatísticas).
    pub fn input(&self) -> &Arc<InputServer> {
        &self.input
    }

    /// O servidor de controle.
    pub fn control(&self) -> &Arc<ControlServer> {
        &self.control
    }

    /// O pareamento (tokens, PIN, autenticação).
    pub fn pairing(&self) -> &Arc<PairingManager<DeviceStoreAdapter>> {
        &self.pairing
    }

    /// Situação do driver de controles virtuais.
    pub fn driver_status(&self) -> DriverStatus {
        self.driver.status()
    }

    /// As portas em que o server escuta de fato.
    pub const fn ports(&self) -> BoundPorts {
        self.ports
    }

    /// As portas configuradas, se forem diferentes das que estão em uso (a UI avisa que é preciso
    /// reiniciar).
    pub fn ports_pending_restart(&self) -> Option<Ports> {
        let configured = self.settings.get().ports;
        (configured.control != self.ports.control
            || configured.input != self.ports.input
            || configured.discovery != self.ports.discovery)
            .then_some(configured)
    }

    /// Endereços de LAN em uso, para a conexão manual.
    pub fn local_addresses(&self) -> Vec<LanInterface> {
        self.discovery
            .as_ref()
            .map(Discovery::local_addresses)
            .unwrap_or_default()
    }

    /// A descoberta (interfaces, nome, dispositivos próximos).
    pub const fn discovery(&self) -> Option<&Discovery> {
        self.discovery.as_ref()
    }

    /// O fingerprint do certificado do server.
    pub fn fingerprint(&self) -> [u8; 32] {
        self.identity.fingerprint()
    }

    // ------------------------------------------------------------------ pareamento

    /// Gera QR code e PIN para parear um celular. Cada chamada invalida o PIN anterior.
    pub fn start_pairing(&self) -> Result<PairingOffer, CoreError> {
        let hosts: Vec<Ipv4Addr> = self.local_addresses().iter().map(|i| i.ip).collect();
        if hosts.is_empty() {
            return Err(CoreError::NoNetwork);
        }
        let name = self
            .discovery
            .as_ref()
            .map_or_else(computer_name, |d| d.info().wire_name());
        let uri = PairingUri {
            hosts: hosts.clone(),
            port: self.ports.control,
            fingerprint: self.identity.fingerprint(),
            pairing_token: Secret::new(*self.pairing.issue_pairing_token().expose()),
            name,
        }
        .encode()
        .map_err(|e| CoreError::Pairing(e.to_string()))?;
        let qr_svg = pairing_qr_svg(&uri).map_err(|e| CoreError::Pairing(e.to_string()))?;
        let pin = self.pairing.issue_pin().expose().clone();
        Ok(PairingOffer {
            uri,
            qr_svg,
            pin,
            token_ttl: self.config_pairing.token_ttl,
            pin_ttl: self.config_pairing.pin_ttl,
            fingerprint_short: short_fingerprint(&self.identity.fingerprint()),
            hosts,
            port: self.ports.control,
        })
    }

    /// Cancela o pareamento em andamento (o QR code e o PIN deixam de valer).
    pub fn cancel_pairing(&self) {
        self.pairing.cancel_pairing();
    }

    /// Responde ao pedido de confirmação de um novo dispositivo. Devolve `false` se o pedido já
    /// expirou ou foi respondido.
    pub fn answer_approval(&self, request_id: u64, accept: bool) -> bool {
        let sender = self
            .shared
            .approvals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&request_id);
        let delivered = sender.is_some_and(|reply| reply.send(accept).is_ok());
        if delivered {
            self.shared
                .emit(CoreEvent::Approval(ApprovalEvent::Resolved {
                    request_id,
                    accepted: accept,
                }));
        }
        delivered
    }

    // ------------------------------------------------------------------ dispositivos

    /// Renomeia o dispositivo. Devolve o nome gravado.
    pub async fn rename_device(&self, id: &str, name: &str) -> Result<String, CoreError> {
        let name = self.shared.devices.rename(id, name).await?;
        self.shared.emit(CoreEvent::DevicesChanged);
        Ok(name)
    }

    /// Slot preferido, perfil padrão e cor. Valem na próxima conexão do dispositivo.
    pub async fn set_device_defaults(
        &self,
        id: &str,
        defaults: &DeviceDefaults,
    ) -> Result<(), CoreError> {
        self.shared.devices.set_defaults(id, defaults).await?;
        self.shared.emit(CoreEvent::DevicesChanged);
        Ok(())
    }

    /// Revoga o dispositivo: o token deixa de valer e a sessão em andamento é encerrada.
    pub async fn revoke_device(&self, id: &str) -> Result<(), CoreError> {
        self.shared.devices.revoke(id, now_ms()).await?;
        self.shared.paired.remove(id);
        self.drop_device_sessions(id, ByeReason::Revoked);
        self.shared.emit(CoreEvent::DevicesChanged);
        Ok(())
    }

    /// Esquece o dispositivo (apaga o registro e o histórico): ele precisa parear de novo.
    pub async fn forget_device(&self, id: &str) -> Result<(), CoreError> {
        self.shared.devices.forget(id).await?;
        self.shared.paired.remove(id);
        self.drop_device_sessions(id, ByeReason::Kicked);
        self.shared.emit(CoreEvent::DevicesChanged);
        Ok(())
    }

    /// Desconecta quem está no slot (botão "Desconectar" do dashboard).
    pub fn disconnect_slot(&self, slot: SlotId) -> Result<(), CoreError> {
        let session = self
            .slots
            .disconnect_slot(slot)
            .map_err(|_| CoreError::Data(padgaming_storage::DataError::NotFound("slot")))?;
        if let Some(session) = session {
            self.control.end_session(session, ByeReason::Kicked);
        }
        Ok(())
    }

    /// Libera o slot de `id` e encerra a sessão dele, se houver.
    fn drop_device_sessions(&self, id: &str, reason: ByeReason) {
        for slot in &self.slots.snapshot().slots {
            if slot.device.as_ref().map(Uuid::as_str) == Some(id)
                && let Ok(Some(session)) = self.slots.disconnect_slot(slot.slot)
            {
                self.control.end_session(session, reason);
            }
        }
    }

    // ------------------------------------------------------------------ encerramento

    /// Começa o encerramento sem consumir o `Core` (o app o guarda num `Arc`): avisa os clients
    /// (`bye{shutdown}`), tira o anúncio da rede, para as tarefas de fundo e fecha o banco.
    /// Chamar de novo não faz nada de mau.
    pub async fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        for task in &self.tasks {
            task.abort();
        }
        if let Some(discovery) = &self.discovery {
            discovery.retire();
        }
        self.control.stop().await;
        self.db.clone().close().await;
        tracing::info!("server encerrado");
    }

    /// Encerra tudo e libera os recursos (threads, sockets). Para o app, que compartilha o
    /// `Core`, use [`Self::stop`].
    pub async fn shutdown(mut self) {
        self.stop().await;
        if let Some(discovery) = self.discovery.take() {
            discovery.shutdown();
        }
        drop(self.control);
        drop(self.input);
        for thread in self.threads {
            let _ = thread.join();
        }
    }
}

/// O que fazer com cada evento do servidor de controle.
async fn control_pump(shared: Arc<Shared>, mut events: broadcast::Receiver<ControlEvent>) {
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                tracing::warn!(missed, "eventos de controle perdidos");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        record(&shared, &event).await;
        shared.emit(CoreEvent::Control(event));
    }
}

/// Histórico e "visto por último". Falhas de gravação só vão para o log: não derrubam o server.
async fn record(shared: &Shared, event: &ControlEvent) {
    let now = now_ms();
    let result = match event {
        ControlEvent::Paired { device_id, .. } => {
            shared.paired.insert(device_id.as_str());
            shared.emit(CoreEvent::DevicesChanged);
            Ok(())
        }
        ControlEvent::SessionStarted {
            device_id,
            slot,
            peer,
            ..
        } => {
            let ip = peer.to_string();
            let seen = shared
                .devices
                .touch_seen(device_id.as_str(), Some(&ip), now)
                .await;
            let logged = shared
                .log
                .append(
                    Some(device_id.as_str()),
                    LogEvent::Connected,
                    Some(slot.index()),
                    Some(&ip),
                    now,
                )
                .await;
            shared.emit(CoreEvent::DevicesChanged);
            seen.and(logged)
        }
        ControlEvent::SessionEnded {
            device_id,
            slot,
            reason,
            ..
        } => {
            let detail = end_reason(*reason);
            let seen = shared
                .devices
                .touch_seen(device_id.as_str(), None, now)
                .await;
            let logged = shared
                .log
                .append(
                    Some(device_id.as_str()),
                    LogEvent::Disconnected,
                    Some(slot.index()),
                    Some(detail),
                    now,
                )
                .await;
            shared.emit(CoreEvent::DevicesChanged);
            seen.and(logged)
        }
        ControlEvent::Rejected {
            peer,
            device_id,
            reason,
        } => {
            // A chave estrangeira só aceita dispositivos que existem.
            let known = match device_id {
                Some(id) => shared
                    .devices
                    .find(id.as_str())
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.id),
                None => None,
            };
            let detail = format!("{reason:?} de {peer}");
            shared
                .log
                .append(
                    known.as_deref(),
                    LogEvent::Rejected,
                    None,
                    Some(&detail),
                    now,
                )
                .await
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        tracing::warn!(%error, "não consegui gravar no histórico");
    }
}

const fn end_reason(reason: SessionEnd) -> &'static str {
    match reason {
        SessionEnd::Dropped => "conexão caiu",
        SessionEnd::Timeout => "sem resposta",
        SessionEnd::ClientBye => "o aparelho saiu",
        SessionEnd::Kicked => "desconectado",
        SessionEnd::Revoked => "revogado",
        SessionEnd::Shutdown => "server encerrado",
    }
}

async fn settings_pump(
    shared: Arc<Shared>,
    control: Arc<ControlServer>,
    mut changes: broadcast::Receiver<Arc<padgaming_storage::AppSettings>>,
) {
    loop {
        match changes.recv().await {
            Ok(settings) => {
                control.set_require_approval(settings.require_approval);
                shared.emit(CoreEvent::Settings(settings));
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

async fn approval_pump(shared: Arc<Shared>, mut requests: mpsc::Receiver<ApprovalRequest>) {
    while let Some(request) = requests.recv().await {
        let request_id = shared.next_approval.fetch_add(1, Ordering::Relaxed);
        let prompt = ApprovalPrompt {
            request_id,
            device_id: request.device_id.as_str().to_owned(),
            name: request.name,
            model: request.model,
            peer: request.peer,
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        shared
            .approvals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(request_id, reply_tx);
        shared.emit(CoreEvent::Approval(ApprovalEvent::Requested(prompt)));

        // A resposta da UI vai para o servidor de controle; se ele desistir (30 s), o popup fecha.
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            let mut reply = request.reply;
            tokio::select! {
                answer = reply_rx => {
                    if let Ok(accept) = answer {
                        let _ = reply.send(accept);
                    }
                }
                () = reply.closed() => {
                    let pending = shared
                        .approvals
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .remove(&request_id);
                    if pending.is_some() {
                        shared.emit(CoreEvent::Approval(ApprovalEvent::Resolved {
                            request_id,
                            accepted: false,
                        }));
                    }
                }
            }
        });
    }
}
