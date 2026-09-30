//! Descoberta (PROTOCOLO §3, T-303): o celular encontra o PC e o PC mostra os celulares próximos,
//! sem digitar IP.
//!
//! - [`announce`]: o server se anuncia como `_padgaming._tcp` (mDNS), com o TXT do §3.1.
//! - [`broadcast`]: fallback para redes que filtram multicast: responde `PGDISC1` na 47802.
//! - [`nearby`]: os clients que anunciam `_padgaming-client._tcp` (lista de "dispositivos
//!   próximos", com debounce de 500 ms e expiração de 15 s).
//! - [`interfaces`]: só interfaces privadas, ativas e não virtuais (ou as que o usuário fixar).
//!
//! [`Discovery`] junta tudo. As interfaces são reavaliadas a cada 10 s (Wi-Fi trocado, cabo
//! plugado) e o campo `free` acompanha os slots.

pub mod announce;
pub mod broadcast;
pub mod info;
pub mod interfaces;
pub mod nearby;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use mdns_sd::ServiceDaemon;
use padgaming_core::SlotManager;

pub use announce::Announcer;
pub use broadcast::BroadcastResponder;
pub use info::ServerInfo;
pub use interfaces::{InterfaceChoice, InterfaceInfo, LanInterface};
pub use nearby::{NearbyBrowser, NearbyDevice, OnNearby, PairedLookup};

/// De quanto em quanto tempo as interfaces são reavaliadas.
const INTERFACE_REFRESH: Duration = Duration::from_secs(10);
/// Intervalo de espera da thread de manutenção (também é a latência para parar).
const MAINTENANCE_POLL: Duration = Duration::from_millis(200);

/// Falha da descoberta.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// Erro de socket.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Erro do `mdns-sd`.
    #[error("mDNS: {0}")]
    Mdns(#[from] mdns_sd::Error),
    /// Nenhuma interface privada, ativa e não virtual.
    #[error("nenhuma interface de rede utilizável para anunciar o server")]
    NoInterface,
}

/// Parâmetros da descoberta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryConfig {
    /// Que interfaces usar.
    pub interfaces: InterfaceChoice,
    /// Onde escutar o broadcast `PGDISC1`. O padrão é `0.0.0.0:47802`.
    pub broadcast_bind: SocketAddr,
    /// Liga o mDNS (anúncio e busca de clients). Desligado, só o broadcast funciona; útil em
    /// testes e em redes que bloqueiam multicast.
    pub mdns: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            interfaces: InterfaceChoice::Auto,
            broadcast_bind: BroadcastResponder::default_bind(),
            mdns: true,
        }
    }
}

struct MdnsParts {
    daemon: ServiceDaemon,
    announcer: Option<Announcer>,
    browser: NearbyBrowser,
}

struct State {
    info: Arc<ArcSwap<ServerInfo>>,
    interfaces: Mutex<Vec<LanInterface>>,
    choice: Mutex<InterfaceChoice>,
    mdns: Mutex<Option<MdnsParts>>,
}

impl State {
    /// Reavalia as interfaces e reajusta o mDNS se mudaram.
    fn refresh_interfaces(&self) {
        let choice = self
            .choice
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let Ok(now) = interfaces::current(&choice) else {
            return;
        };
        let mut current = self
            .interfaces
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if *current == now {
            // Sem mudança, mas o anúncio ainda pode não ter subido (não havia interface).
            drop(current);
            self.ensure_announced(&now);
            return;
        }
        tracing::info!(interfaces = ?now, "interfaces de rede mudaram");
        let before = std::mem::replace(&mut *current, now.clone());
        drop(current);
        if let Some(parts) = self
            .mdns
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            if let Err(error) = announce::switch_interfaces(&parts.daemon, &before, &now) {
                tracing::warn!(%error, "não consegui ajustar as interfaces do mDNS");
            }
            Self::announce(parts, &self.info.load(), &now);
        }
    }

    fn ensure_announced(&self, interfaces: &[LanInterface]) {
        let mut mdns = self.mdns.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(parts) = mdns.as_mut()
            && parts.announcer.is_none()
        {
            Self::announce(parts, &self.info.load(), interfaces);
        }
    }

    /// (Re)anuncia. Sem interface, retira o anúncio.
    fn announce(parts: &mut MdnsParts, info: &ServerInfo, interfaces: &[LanInterface]) {
        match (&parts.announcer, interfaces.is_empty()) {
            (Some(_), true) => {
                if let Some(old) = parts.announcer.take() {
                    old.shutdown();
                }
            }
            (Some(announcer), false) => {
                if let Err(error) = announcer.update(info, interfaces) {
                    tracing::warn!(%error, "não consegui atualizar o anúncio mDNS");
                }
            }
            (None, true) => {}
            (None, false) => match Announcer::start(&parts.daemon, info, interfaces) {
                Ok(announcer) => parts.announcer = Some(announcer),
                Err(error) => tracing::warn!(%error, "não consegui anunciar por mDNS"),
            },
        }
    }

    /// Publica os dados novos (broadcast na hora, mDNS reanunciado).
    fn publish(&self, next: &ServerInfo) {
        self.info.store(Arc::new(next.clone()));
        let interfaces = self
            .interfaces
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(parts) = self
            .mdns
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            Self::announce(parts, next, &interfaces);
        }
    }
}

/// A descoberta completa do server.
pub struct Discovery {
    state: Arc<State>,
    responder: Option<BroadcastResponder>,
    stop: Arc<AtomicBool>,
    maintenance: Option<JoinHandle<()>>,
}

impl Discovery {
    /// Inicia o respondedor de broadcast e, se `config.mdns`, o anúncio e a busca de clients.
    ///
    /// `paired` diz se um `device_id` já é pareado (para a lista de próximos) e `on_nearby`
    /// recebe a lista sempre que ela muda (debounce de 500 ms).
    pub fn start(
        config: &DiscoveryConfig,
        info: &ServerInfo,
        paired: PairedLookup,
        on_nearby: OnNearby,
    ) -> Result<Self, DiscoveryError> {
        let chosen = interfaces::current(&config.interfaces)?;
        let shared_info = Arc::new(ArcSwap::from_pointee(info.clone()));
        let responder = BroadcastResponder::start(config.broadcast_bind, Arc::clone(&shared_info))?;

        let mdns = if config.mdns {
            let daemon = announce::new_daemon(&chosen)?;
            let announcer = if chosen.is_empty() {
                tracing::warn!("sem interface utilizável: o anúncio mDNS começa quando houver uma");
                None
            } else {
                Some(Announcer::start(&daemon, info, &chosen)?)
            };
            let browser = NearbyBrowser::start(&daemon, paired, on_nearby)?;
            Some(MdnsParts {
                daemon,
                announcer,
                browser,
            })
        } else {
            None
        };

        let state = Arc::new(State {
            info: shared_info,
            interfaces: Mutex::new(chosen),
            choice: Mutex::new(config.interfaces.clone()),
            mdns: Mutex::new(mdns),
        });
        let stop = Arc::new(AtomicBool::new(false));
        let maintenance = std::thread::Builder::new()
            .name("discovery-maint".into())
            .spawn({
                let (state, stop) = (Arc::clone(&state), Arc::clone(&stop));
                move || {
                    let mut last = Instant::now();
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(MAINTENANCE_POLL);
                        if last.elapsed() >= INTERFACE_REFRESH {
                            last = Instant::now();
                            state.refresh_interfaces();
                        }
                    }
                }
            })?;
        Ok(Self {
            state,
            responder: Some(responder),
            stop,
            maintenance: Some(maintenance),
        })
    }

    /// Onde o respondedor de broadcast escuta.
    pub fn broadcast_addr(&self) -> Option<SocketAddr> {
        self.responder.as_ref().map(BroadcastResponder::local_addr)
    }

    /// O que está sendo anunciado agora.
    pub fn info(&self) -> Arc<ServerInfo> {
        self.state.info.load_full()
    }

    /// Atualiza os slots livres (broadcast na hora, mDNS reanunciado). Não faz nada se o valor
    /// não mudou.
    pub fn set_free(&self, free: u8) {
        let current = self.state.info.load();
        if current.free == free {
            return;
        }
        let mut next = (**current).clone();
        next.free = free.min(4);
        self.state.publish(&next);
    }

    /// Troca o nome exibido (Configurações).
    pub fn set_name(&self, name: &str) {
        let current = self.state.info.load();
        if current.name == name {
            return;
        }
        let mut next = (**current).clone();
        name.clone_into(&mut next.name);
        self.state.publish(&next);
    }

    /// Muda a escolha de interfaces (Configurações) e reaplica na hora.
    pub fn set_interface_choice(&self, choice: InterfaceChoice) {
        *self
            .state
            .choice
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = choice;
        self.state.refresh_interfaces();
    }

    /// Endereços locais em uso, para a UI mostrar na conexão manual.
    pub fn local_addresses(&self) -> Vec<LanInterface> {
        self.state
            .interfaces
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Os dispositivos próximos neste instante (vazio sem mDNS).
    pub fn nearby(&self) -> Vec<NearbyDevice> {
        self.state
            .mdns
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|parts| parts.browser.snapshot())
            .unwrap_or_default()
    }

    /// Faz o `free` acompanhar os slots: uma thread escuta as mudanças do [`SlotManager`].
    /// Termina sozinha com a descoberta ou quando o `SlotManager` some.
    pub fn follow_slots(&self, slots: &SlotManager) -> std::io::Result<JoinHandle<()>> {
        let updates = slots.subscribe();
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop);
        std::thread::Builder::new()
            .name("discovery-slots".into())
            .spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match updates.recv_timeout(MAINTENANCE_POLL) {
                        Ok(snapshot) => {
                            let free = u8::try_from(snapshot.free_count()).unwrap_or(4);
                            let current = state.info.load();
                            if current.free != free {
                                let mut next = (**current).clone();
                                next.free = free.min(4);
                                state.publish(&next);
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
    }

    /// Retira o anúncio da rede (com "goodbye") sem parar o resto; para quem guarda a descoberta
    /// compartilhada e não pode consumi-la (o app encerrando). Chamar de novo não faz nada.
    pub fn retire(&self) {
        if let Some(parts) = self
            .state
            .mdns
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            && let Some(announcer) = parts.announcer.take()
        {
            announcer.shutdown();
        }
    }

    /// Para tudo: retira o anúncio (com "goodbye"), para a busca e o broadcast.
    pub fn shutdown(mut self) {
        self.stop_all();
    }

    fn stop_all(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.maintenance.take() {
            let _ = thread.join();
        }
        if let Some(parts) = self
            .state
            .mdns
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            if let Some(announcer) = parts.announcer {
                announcer.shutdown();
            }
            parts.browser.shutdown();
            let _ = parts.daemon.shutdown();
        }
        if let Some(responder) = self.responder.take() {
            responder.shutdown();
        }
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.stop_all();
    }
}
