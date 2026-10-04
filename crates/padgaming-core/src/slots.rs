//! Slot Manager: até 4 controles virtuais, cada um ligado a um dispositivo e a uma sessão.
//!
//! Ciclo de vida de um slot:
//!
//! ```text
//! Free ──assign──▶ Reserved ──1º input──▶ Active ──queda do TLS──▶ Suspended ──30 s──▶ Free
//!                     │                      │                        │
//!                     └──────── bye / desconectar pela UI ────────────┴──▶ Free
//! ```
//!
//! - `Reserved`: o slot é do dispositivo, mas o controle virtual ainda não existe. Ele só é
//!   plugado no primeiro input, para um handshake que falha não deixar controle fantasma.
//! - `Suspended`: a sessão caiu. O controle continua plugado (neutro) e o slot fica reservado
//!   ao dispositivo por [`SlotConfig::reconnect_window`]; se ele voltar dentro do prazo, retoma
//!   o mesmo slot sem replugar (o jogo não percebe). Depois do prazo, unplug e `Free`.
//!
//! Concorrência: as operações de controle (`assign`, `release`, UI...) serializam num mutex.
//! O hot path ([`SlotManager::submit`]) só lê uma tabela publicada via `ArcSwap`: sem lock e sem
//! alocação, exceto no primeiro input de uma sessão, que precisa plugar o controle.
//!
//! Limitação: o índice `XInput` é atribuído pelo Windows no plug. Trocar dois slots de lugar
//! exige unplug/replug e o jogo verá os controles desconectarem por um instante.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, Sender, unbounded};
use padgaming_driver::{DriverError, MAX_SLOTS, PadHandle, SlotId, VirtualPadDriver, XReport};
use padgaming_protocol::control::Uuid;

use crate::clock::{Clock, SystemClock};

const N: usize = MAX_SLOTS as usize;

/// Parâmetros do gerenciador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotConfig {
    /// Quanto tempo o slot fica reservado depois da queda da sessão (PROTOCOLO §9).
    pub reconnect_window: Duration,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            reconnect_window: Duration::from_secs(30),
        }
    }
}

/// Erro de uma operação de slot.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SlotError {
    /// Os 4 slots estão ocupados (o client recebe `reject{no_slot}`).
    #[error("todos os slots estão ocupados")]
    NoSlot,
    /// A sessão não tem slot ativo ou reservado.
    #[error("sessão desconhecida")]
    UnknownSession,
    /// O slot está livre.
    #[error("slot vazio")]
    EmptySlot,
    /// Falha do driver de emulação.
    #[error(transparent)]
    Driver(#[from] DriverError),
}

/// Controle virtual plugado, compartilhado com o hot path.
struct HotPad {
    handle: PadHandle,
    /// Sessão dona do controle; muda quando o dispositivo reconecta.
    session: AtomicU32,
    /// Entrada ignorada (a UI pausou o slot).
    paused: AtomicBool,
    /// Último estado enviado ao driver, empacotado em duas palavras (só para a UI ao vivo).
    last: [AtomicU64; 2],
}

impl HotPad {
    fn new(handle: PadHandle, session: u32) -> Self {
        Self {
            handle,
            session: AtomicU32::new(session),
            paused: AtomicBool::new(false),
            last: [AtomicU64::new(0), AtomicU64::new(0)],
        }
    }

    /// Envia o estado ao driver e guarda uma cópia para o monitor ao vivo. Sem lock e sem
    /// alocação: são duas escritas atômicas relaxadas.
    fn write(&self, driver: &dyn VirtualPadDriver, report: &XReport) -> Result<(), DriverError> {
        let [a, b] = pack(report);
        self.last[0].store(a, Ordering::Relaxed);
        self.last[1].store(b, Ordering::Relaxed);
        driver.update(&self.handle, report)
    }

    fn last_report(&self) -> XReport {
        unpack([
            self.last[0].load(Ordering::Relaxed),
            self.last[1].load(Ordering::Relaxed),
        ])
    }

    /// Envia um estado neutro; erros são irrelevantes (o controle pode já ter sumido).
    fn neutralize(&self, driver: &dyn VirtualPadDriver) {
        let _ = self.write(driver, &XReport::default());
    }
}

/// `XReport` em duas palavras de 64 bits: botões, gatilhos, LX e LY na primeira; RX e RY na segunda.
#[allow(clippy::cast_sign_loss)]
fn pack(r: &XReport) -> [u64; 2] {
    [
        u64::from(r.buttons)
            | u64::from(r.lt) << 16
            | u64::from(r.rt) << 24
            | u64::from(r.lx as u16) << 32
            | u64::from(r.ly as u16) << 48,
        u64::from(r.rx as u16) | u64::from(r.ry as u16) << 16,
    ]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn unpack(words: [u64; 2]) -> XReport {
    let [a, b] = words;
    XReport {
        buttons: a as u16,
        lt: (a >> 16) as u8,
        rt: (a >> 24) as u8,
        lx: (a >> 32) as u16 as i16,
        ly: (a >> 48) as u16 as i16,
        rx: b as u16 as i16,
        ry: (b >> 16) as u16 as i16,
    }
}

enum Phase {
    Reserved {
        session: u32,
    },
    Active {
        pad: Arc<HotPad>,
    },
    Suspended {
        since: Instant,
        pad: Option<Arc<HotPad>>,
    },
}

struct Occupant {
    device: Uuid,
    paused: bool,
    phase: Phase,
}

impl Occupant {
    fn pad(&self) -> Option<&Arc<HotPad>> {
        match &self.phase {
            Phase::Active { pad } => Some(pad),
            Phase::Suspended { pad, .. } => pad.as_ref(),
            Phase::Reserved { .. } => None,
        }
    }

    fn session(&self) -> Option<u32> {
        match &self.phase {
            Phase::Reserved { session } => Some(*session),
            Phase::Active { pad } => Some(pad.session.load(Ordering::Relaxed)),
            Phase::Suspended { .. } => None,
        }
    }
}

/// Tabela lida pelo hot path: só os controles `Active`.
type HotTable = [Option<Arc<HotPad>>; N];

/// Estado de um slot, como a UI o vê.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotView {
    /// Livre.
    Free,
    /// Reservado, ainda sem controle plugado.
    Reserved,
    /// Controle plugado e recebendo input.
    Active,
    /// Sessão caída; o slot é liberado em `expires_at` se o dispositivo não voltar.
    Suspended {
        /// Fim da reserva.
        expires_at: Instant,
    },
}

/// Foto de um slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotSnapshot {
    /// Posição.
    pub slot: SlotId,
    /// Estado.
    pub state: SlotView,
    /// Dispositivo dono do slot.
    pub device: Option<Uuid>,
    /// Sessão em andamento (`Reserved` e `Active`).
    pub session: Option<u32>,
    /// A UI pausou o slot.
    pub paused: bool,
    /// Índice `XInput` real do controle, se plugado e conhecido.
    pub user_index: Option<u8>,
}

/// Foto dos 4 slots, publicada a cada mudança.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotsSnapshot {
    /// Os slots, em ordem.
    pub slots: [SlotSnapshot; N],
}

impl SlotsSnapshot {
    /// Quantidade de slots livres (o `free` do anúncio mDNS).
    pub fn free_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.state == SlotView::Free)
            .count()
    }
}

struct Inner {
    slots: [Option<Occupant>; N],
}

/// Gerencia os 4 slots sobre um [`VirtualPadDriver`].
pub struct SlotManager {
    driver: Arc<dyn VirtualPadDriver>,
    clock: Arc<dyn Clock>,
    /// Janela de reconexão em ms (muda em tempo de execução, pelas configurações).
    reconnect_ms: AtomicU64,
    inner: Mutex<Inner>,
    hot: ArcSwap<HotTable>,
    snapshot: ArcSwap<SlotsSnapshot>,
    subscribers: Mutex<Vec<Sender<Arc<SlotsSnapshot>>>>,
}

fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

fn slot_id(index: usize) -> SlotId {
    SlotId::new(u8::try_from(index).expect("índice de slot")).expect("índice de slot")
}

impl SlotManager {
    /// Gerenciador com o relógio do sistema e a configuração padrão.
    pub fn new(driver: Arc<dyn VirtualPadDriver>) -> Self {
        Self::with_clock(driver, SlotConfig::default(), Arc::new(SystemClock))
    }

    /// Gerenciador com relógio e configuração explícitos.
    pub fn with_clock(
        driver: Arc<dyn VirtualPadDriver>,
        config: SlotConfig,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let empty = Inner {
            slots: Default::default(),
        };
        let snapshot = build_snapshot(&empty, driver.as_ref(), config.reconnect_window);
        Self {
            driver,
            clock,
            reconnect_ms: AtomicU64::new(duration_ms(config.reconnect_window)),
            inner: Mutex::new(empty),
            hot: ArcSwap::from_pointee(Default::default()),
            snapshot: ArcSwap::from_pointee(snapshot),
            subscribers: Mutex::new(Vec::new()),
        }
    }

    /// Quanto tempo o slot fica reservado depois da queda da sessão.
    pub fn reconnect_window(&self) -> Duration {
        Duration::from_millis(self.reconnect_ms.load(Ordering::Relaxed))
    }

    /// Muda a janela de reconexão. Vale para as próximas quedas e para as reservas em andamento
    /// (o prazo de cada uma é contado a partir da queda).
    pub fn set_reconnect_window(&self, window: Duration) {
        self.reconnect_ms
            .store(duration_ms(window), Ordering::Relaxed);
        // As contagens regressivas da UI dependem do prazo: publica uma foto nova.
        let inner = self.lock();
        self.publish(&inner);
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // Nenhuma seção crítica deixa o estado pela metade se entrar em pânico.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    // ------------------------------------------------------------------ consulta

    /// Foto atual dos slots (sem lock).
    pub fn snapshot(&self) -> Arc<SlotsSnapshot> {
        self.snapshot.load_full()
    }

    /// Recebe uma foto a cada mudança (evento `slots:changed` da UI).
    pub fn subscribe(&self) -> Receiver<Arc<SlotsSnapshot>> {
        let (tx, rx) = unbounded();
        self.subscribers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(tx);
        rx
    }

    // ------------------------------------------------------------------ rede

    /// Atribui um slot ao dispositivo depois da autenticação.
    ///
    /// 1. Se o dispositivo já tem slot (reconexão, ou conexão antiga que ainda não caiu), ele
    ///    volta para o mesmo slot com a nova sessão, sem replugar o controle.
    /// 2. Senão, o slot preferido do dispositivo, se estiver livre.
    /// 3. Senão, o menor slot livre.
    /// 4. Senão, [`SlotError::NoSlot`].
    pub fn assign(
        &self,
        device: &Uuid,
        session: u32,
        preferred: Option<SlotId>,
    ) -> Result<SlotId, SlotError> {
        let mut inner = self.lock();
        self.expire_locked(&mut inner);

        let index = if let Some(i) = inner.find_device(device) {
            let occupant = inner.slots[i].as_mut().expect("achado por find_device");
            occupant.phase =
                match std::mem::replace(&mut occupant.phase, Phase::Reserved { session }) {
                    Phase::Active { pad } | Phase::Suspended { pad: Some(pad), .. } => {
                        pad.session.store(session, Ordering::Relaxed);
                        Phase::Active { pad }
                    }
                    Phase::Reserved { .. } | Phase::Suspended { pad: None, .. } => {
                        Phase::Reserved { session }
                    }
                };
            i
        } else {
            let free = |i: usize| inner.slots[i].is_none();
            let i = preferred
                .map(|s| usize::from(s.index()))
                .filter(|i| free(*i))
                .or_else(|| (0..N).find(|i| free(*i)))
                .ok_or(SlotError::NoSlot)?;
            inner.slots[i] = Some(Occupant {
                device: device.clone(),
                paused: false,
                phase: Phase::Reserved { session },
            });
            i
        };
        self.publish(&inner);
        Ok(slot_id(index))
    }

    /// Entrega o estado do controle ao driver. É o hot path do input.
    ///
    /// Sem lock e sem alocação, exceto no primeiro input da sessão, que pluga o controle.
    pub fn submit(&self, session: u32, report: &XReport) -> Result<(), SlotError> {
        {
            let table = self.hot.load();
            let pad = table
                .iter()
                .flatten()
                .find(|p| p.session.load(Ordering::Relaxed) == session);
            if let Some(pad) = pad {
                if pad.paused.load(Ordering::Relaxed) {
                    return Ok(());
                }
                return Ok(pad.write(self.driver.as_ref(), report)?);
            }
        }
        self.first_input(session, report)
    }

    /// Caminho lento: a sessão ainda não tem controle plugado.
    fn first_input(&self, session: u32, report: &XReport) -> Result<(), SlotError> {
        let pad = {
            let mut inner = self.lock();
            let index = inner
                .find_session(session)
                .ok_or(SlotError::UnknownSession)?;
            let occupant = inner.slots[index]
                .as_mut()
                .expect("achado por find_session");
            match &occupant.phase {
                // Outra thread plugou entre o `load` e o lock.
                Phase::Active { pad } => Arc::clone(pad),
                Phase::Reserved { .. } if occupant.paused => return Ok(()),
                Phase::Reserved { .. } => {
                    let handle = self.driver.plug(slot_id(index))?;
                    let pad = Arc::new(HotPad::new(handle, session));
                    occupant.phase = Phase::Active {
                        pad: Arc::clone(&pad),
                    };
                    self.publish(&inner);
                    pad
                }
                Phase::Suspended { .. } => return Err(SlotError::UnknownSession),
            }
        };
        if pad.paused.load(Ordering::Relaxed) {
            return Ok(());
        }
        Ok(pad.write(self.driver.as_ref(), report)?)
    }

    /// Solta todos os botões do controle da sessão sem tirá-lo do lugar (o pad continua
    /// plugado). É o watchdog de "input mudo" (PROTOCOLO §9). Devolve `false` se a sessão não
    /// tem controle plugado (ainda não mandou input, ou não existe).
    pub fn neutralize_session(&self, session: u32) -> bool {
        let table = self.hot.load();
        let Some(pad) = table
            .iter()
            .flatten()
            .find(|p| p.session.load(Ordering::Relaxed) == session)
        else {
            return false;
        };
        pad.neutralize(self.driver.as_ref());
        true
    }

    /// O último estado enviado ao controle do `slot`, para o monitor ao vivo da UI. `None` se o
    /// slot não tem controle plugado. Não bloqueia e não aloca; é uma leitura aproximada (as duas
    /// palavras podem ser de pacotes vizinhos), boa para desenhar, não para decidir nada.
    pub fn live_report(&self, slot: SlotId) -> Option<XReport> {
        self.hot.load()[usize::from(slot.index())]
            .as_ref()
            .map(|pad| pad.last_report())
    }

    /// Disponibilidade atual do driver (para revalidar depois de o Windows retomar).
    pub fn driver_status(&self) -> padgaming_driver::DriverStatus {
        self.driver.status()
    }

    /// A sessão caiu (TLS fechou ou expirou): o slot fica reservado ao dispositivo.
    ///
    /// Devolve o slot afetado. O controle, se existir, vai a neutro e continua plugado.
    pub fn suspend(&self, session: u32) -> Option<SlotId> {
        let mut inner = self.lock();
        let index = inner.find_session(session)?;
        let now = self.clock.now();
        let occupant = inner.slots[index]
            .as_mut()
            .expect("achado por find_session");
        let pad = match std::mem::replace(&mut occupant.phase, Phase::Reserved { session }) {
            Phase::Active { pad } => {
                pad.neutralize(self.driver.as_ref());
                Some(pad)
            }
            _ => None,
        };
        occupant.phase = Phase::Suspended { since: now, pad };
        self.publish(&inner);
        Some(slot_id(index))
    }

    /// O dispositivo saiu de propósito (`bye`): libera o slot na hora.
    pub fn release(&self, session: u32) -> Option<SlotId> {
        let mut inner = self.lock();
        let index = inner.find_session(session)?;
        self.free_locked(&mut inner, index);
        self.publish(&inner);
        Some(slot_id(index))
    }

    /// Libera os slots cuja reserva expirou. Deve ser chamado periodicamente (1 s basta);
    /// `assign` também o faz. Devolve os slots liberados.
    pub fn tick(&self) -> Vec<SlotId> {
        let mut inner = self.lock();
        let freed = self.expire_locked(&mut inner);
        if !freed.is_empty() {
            self.publish(&inner);
        }
        freed
    }

    fn expire_locked(&self, inner: &mut Inner) -> Vec<SlotId> {
        let now = self.clock.now();
        let window = self.reconnect_window();
        let expired: Vec<usize> = (0..N)
            .filter(|i| {
                matches!(
                    inner.slots[*i].as_ref().map(|o| &o.phase),
                    Some(Phase::Suspended { since, .. }) if now.duration_since(*since) >= window
                )
            })
            .collect();
        for i in &expired {
            self.free_locked(inner, *i);
        }
        expired.into_iter().map(slot_id).collect()
    }

    // ------------------------------------------------------------------ ações da UI

    /// Desconecta o slot. Devolve a sessão em andamento, para o servidor de controle mandar
    /// `bye{kicked}` e fechar a conexão.
    pub fn disconnect_slot(&self, slot: SlotId) -> Result<Option<u32>, SlotError> {
        let mut inner = self.lock();
        let index = usize::from(slot.index());
        let session = inner.slots[index]
            .as_ref()
            .ok_or(SlotError::EmptySlot)?
            .session();
        self.free_locked(&mut inner, index);
        self.publish(&inner);
        Ok(session)
    }

    /// Pausa ou retoma o slot: pausado, o input é ignorado e o controle fica neutro.
    pub fn set_paused(&self, slot: SlotId, paused: bool) -> Result<(), SlotError> {
        let mut inner = self.lock();
        let occupant = inner.slots[usize::from(slot.index())]
            .as_mut()
            .ok_or(SlotError::EmptySlot)?;
        occupant.paused = paused;
        if let Some(pad) = occupant.pad() {
            pad.paused.store(paused, Ordering::Relaxed);
            if paused {
                pad.neutralize(self.driver.as_ref());
            }
        }
        self.publish(&inner);
        Ok(())
    }

    /// Troca dois slots de lugar (um deles pode estar livre).
    ///
    /// Os controles plugados são removidos e replugados na ordem dos slots, porque o índice
    /// `XInput` é atribuído pelo Windows no plug. Se o replug falhar, o slot volta a `Reserved` e
    /// o controle é plugado de novo no próximo input.
    pub fn swap(&self, a: SlotId, b: SlotId) -> Result<(), SlotError> {
        let (ia, ib) = (usize::from(a.index()), usize::from(b.index()));
        if ia == ib {
            return Ok(());
        }
        let mut inner = self.lock();
        if inner.slots[ia].is_none() && inner.slots[ib].is_none() {
            return Err(SlotError::EmptySlot);
        }

        // Quem estava plugado (ativo) precisa de controle novo na posição nova.
        let mut replug = [false; N];
        for i in [ia, ib] {
            if let Some(occupant) = inner.slots[i].as_mut()
                && let Some(pad) = occupant.pad()
            {
                let pad = Arc::clone(pad);
                pad.neutralize(self.driver.as_ref());
                self.unplug(&pad);
                replug[i] = matches!(occupant.phase, Phase::Active { .. });
                occupant.phase = match occupant.phase {
                    Phase::Active { .. } => Phase::Reserved {
                        session: pad.session.load(Ordering::Relaxed),
                    },
                    Phase::Suspended { since, .. } => Phase::Suspended { since, pad: None },
                    Phase::Reserved { .. } => unreachable!("Reserved não tem controle"),
                };
            }
        }

        inner.slots.swap(ia, ib);
        replug.swap(ia, ib);

        for i in [ia.min(ib), ia.max(ib)] {
            if !replug[i] {
                continue;
            }
            let session = inner.slots[i].as_ref().and_then(Occupant::session);
            let (Some(session), Ok(handle)) = (session, self.driver.plug(slot_id(i))) else {
                continue; // fica `Reserved`; pluga no próximo input
            };
            let occupant = inner.slots[i].as_mut().expect("slot ocupado");
            let pad = Arc::new(HotPad::new(handle, session));
            pad.paused.store(occupant.paused, Ordering::Relaxed);
            occupant.phase = Phase::Active { pad };
        }
        self.publish(&inner);
        Ok(())
    }

    // ------------------------------------------------------------------ internos

    /// Neutraliza, remove o controle e libera o slot.
    fn free_locked(&self, inner: &mut Inner, index: usize) {
        if let Some(occupant) = inner.slots[index].take()
            && let Some(pad) = occupant.pad()
        {
            pad.neutralize(self.driver.as_ref());
            self.unplug(pad);
        }
    }

    fn unplug(&self, pad: &HotPad) {
        // O handle não é `Clone` (quem o tem deve chamar `unplug`); aqui ele também está na
        // tabela do hot path, então reconstruímos um equivalente. Um `update` em voo com o
        // handle antigo só recebe `InvalidHandle`.
        let handle = PadHandle::new(pad.handle.slot(), pad.handle.token());
        let _ = self.driver.unplug(handle);
    }

    /// Atualiza a tabela do hot path e a foto da UI, e avisa os assinantes.
    fn publish(&self, inner: &Inner) {
        let mut table: HotTable = Default::default();
        for (i, slot) in inner.slots.iter().enumerate() {
            if let Some(Occupant {
                phase: Phase::Active { pad },
                ..
            }) = slot
            {
                table[i] = Some(Arc::clone(pad));
            }
        }
        self.hot.store(Arc::new(table));

        let snapshot = Arc::new(build_snapshot(
            inner,
            self.driver.as_ref(),
            self.reconnect_window(),
        ));
        self.snapshot.store(Arc::clone(&snapshot));
        self.subscribers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|tx| tx.send(Arc::clone(&snapshot)).is_ok());
    }
}

impl Inner {
    fn find_device(&self, device: &Uuid) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.as_ref().is_some_and(|o| &o.device == device))
    }

    fn find_session(&self, session: u32) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.as_ref().and_then(Occupant::session) == Some(session))
    }
}

fn build_snapshot(
    inner: &Inner,
    driver: &dyn VirtualPadDriver,
    reconnect_window: Duration,
) -> SlotsSnapshot {
    let slots = std::array::from_fn(|i| {
        let slot = slot_id(i);
        let Some(occupant) = inner.slots[i].as_ref() else {
            return SlotSnapshot {
                slot,
                state: SlotView::Free,
                device: None,
                session: None,
                paused: false,
                user_index: None,
            };
        };
        let state = match &occupant.phase {
            Phase::Reserved { .. } => SlotView::Reserved,
            Phase::Active { .. } => SlotView::Active,
            Phase::Suspended { since, .. } => SlotView::Suspended {
                expires_at: *since + reconnect_window,
            },
        };
        SlotSnapshot {
            slot,
            state,
            device: Some(occupant.device.clone()),
            session: occupant.session(),
            paused: occupant.paused,
            user_index: occupant.pad().and_then(|p| driver.user_index(&p.handle)),
        }
    });
    SlotsSnapshot { slots }
}

#[cfg(test)]
mod tests;
