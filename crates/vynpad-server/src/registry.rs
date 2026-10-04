//! O perfil de cada slot: o elo entre o `ProfileService` (banco), o `SlotManager` e o servidor de
//! controle. Implementa [`SlotProfiles`] (o que um dispositivo recebe ao conectar) e
//! [`ProfileHost`] (a troca a quente quando um perfil é editado).

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

use vynpad_core::SlotManager;
use vynpad_driver::SlotId;
use vynpad_mapping::{CompiledProfile, ProfileCell};
use vynpad_net::control::{ControlServer, PreferredFuture, SetupFuture, SlotProfiles, SlotSetup};
use vynpad_net::input::InputServer;
use vynpad_protocol::control::{Config, Uuid};
use vynpad_storage::seed::BUILTIN_PROFILE_ID;
use vynpad_storage::{AppSettings, DeviceRepo, ProfileHost, ProfileService};

/// Os 4 `ProfileCell`s (um por slot) e qual perfil cada slot está usando.
pub struct SlotRegistry {
    cells: Mutex<[Arc<ProfileCell>; 4]>,
    assigned: Mutex<[Option<String>; 4]>,
    /// Taxa máxima de envio sugerida aos clients (Hz), das configurações.
    send_rate_hz: AtomicU16,
    /// Perfil dos dispositivos que não têm um padrão próprio (das configurações).
    default_profile: Mutex<Option<String>>,
    slots: Arc<SlotManager>,
    devices: DeviceRepo,
    profiles: OnceLock<Weak<ProfileService>>,
    control: OnceLock<Weak<ControlServer>>,
    input: OnceLock<Weak<InputServer>>,
}

impl SlotRegistry {
    pub(crate) fn new(slots: Arc<SlotManager>, devices: DeviceRepo) -> Self {
        let cell = || Arc::new(ProfileCell::new(CompiledProfile::standard()));
        Self {
            cells: Mutex::new([cell(), cell(), cell(), cell()]),
            assigned: Mutex::new(Default::default()),
            send_rate_hz: AtomicU16::new(AppSettings::default().send_rate_hz),
            default_profile: Mutex::new(None),
            slots,
            devices,
            profiles: OnceLock::new(),
            control: OnceLock::new(),
            input: OnceLock::new(),
        }
    }

    /// Liga o serviço de perfis (existe depois do registro, que ele mesmo recebe como host).
    pub(crate) fn connect_profiles(&self, profiles: &Arc<ProfileService>) {
        let _ = self.profiles.set(Arc::downgrade(profiles));
    }

    /// Liga o servidor de controle (para reenviar `config` quando o gyro liga ou desliga).
    pub(crate) fn connect_control(&self, control: &Arc<ControlServer>) {
        let _ = self.control.set(Arc::downgrade(control));
    }

    /// Liga o servidor de input (para parar a vibração quando um perfil a desliga).
    pub(crate) fn connect_input(&self, input: &Arc<InputServer>) {
        let _ = self.input.set(Arc::downgrade(input));
    }

    /// A vibração pedida pelo jogo no `slot`, depois da intensidade do perfil do slot.
    pub(crate) fn scale_rumble(&self, slot: SlotId, large: u8, small: u8) -> (u8, u8) {
        self.cells()[usize::from(slot.index())]
            .load()
            .scale_rumble(large, small)
    }

    fn cells(&self) -> std::sync::MutexGuard<'_, [Arc<ProfileCell>; 4]> {
        self.cells.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn assigned(&self) -> std::sync::MutexGuard<'_, [Option<String>; 4]> {
        self.assigned.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn config_for(&self, compiled: &CompiledProfile, slot: SlotId) -> Config {
        Config {
            slot: slot.index(),
            send_rate_hz: self.send_rate_hz.load(Ordering::Relaxed),
            gyro_enabled: compiled.gyro_enabled(),
            rumble_enabled: compiled.rumble_enabled(),
            touch_haptics: true,
        }
    }

    /// Slots ocupados por um dispositivo que estão com o perfil `id`.
    fn slots_with(&self, id: &str) -> Vec<SlotId> {
        let snapshot = self.slots.snapshot();
        let assigned = self.assigned();
        snapshot
            .slots
            .iter()
            .filter(|s| s.device.is_some())
            .filter(|s| assigned[usize::from(s.slot.index())].as_deref() == Some(id))
            .map(|s| s.slot)
            .collect()
    }
}

impl SlotRegistry {
    /// Aplica as configurações que mexem nos slots: o perfil dos dispositivos sem padrão próprio
    /// e a taxa de envio (que, ao mudar, é reenviada na hora aos controles conectados).
    pub(crate) fn apply_settings(&self, settings: &AppSettings) {
        self.default_profile
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone_from(&settings.default_profile_id);
        let previous = self
            .send_rate_hz
            .swap(settings.send_rate_hz, Ordering::Relaxed);
        if previous != settings.send_rate_hz
            && let Some(control) = self.control.get().and_then(Weak::upgrade)
        {
            for slot in self
                .slots
                .snapshot()
                .slots
                .iter()
                .filter(|s| s.device.is_some())
            {
                let compiled = self.cells()[usize::from(slot.slot.index())].load();
                control.send_config(slot.slot, self.config_for(&compiled, slot.slot));
            }
        }
    }

    /// Um perfil que desliga a vibração para o que estiver vibrando agora.
    fn silence_if_off(&self, slot: SlotId, compiled: &CompiledProfile) {
        if compiled.rumble_enabled() {
            return;
        }
        let session = self.slots.snapshot().slots[usize::from(slot.index())].session;
        if let (Some(session), Some(input)) = (session, self.input.get().and_then(Weak::upgrade)) {
            input.set_rumble(session, 0, 0);
        }
    }

    /// O perfil que cada slot está usando (`None` onde nunca houve um dispositivo).
    pub(crate) fn assigned_profiles(&self) -> [Option<String>; 4] {
        self.assigned().clone()
    }

    /// Troca o perfil do `slot` na hora, sem mexer no que o dispositivo tem como padrão.
    /// Devolve o `id` realmente aplicado (o padrão, se o pedido estava inválido).
    pub(crate) async fn set_slot_profile(&self, slot: SlotId, id: &str) -> Option<String> {
        let profiles = self.profiles.get().and_then(Weak::upgrade)?;
        let resolved = profiles.resolve_or_default(Some(id)).await;
        let index = usize::from(slot.index());
        self.cells()[index].store(Arc::clone(&resolved.compiled));
        self.silence_if_off(slot, &resolved.compiled);
        self.assigned()[index] = Some(resolved.id.clone());
        if let Some(control) = self.control.get().and_then(Weak::upgrade) {
            control.send_config(slot, self.config_for(&resolved.compiled, slot));
        }
        Some(resolved.id)
    }

    /// Acompanha a troca de dois slots: o perfil (e a célula que a sessão segura) vai junto.
    pub(crate) fn swap(&self, a: SlotId, b: SlotId) {
        let (ia, ib) = (usize::from(a.index()), usize::from(b.index()));
        self.cells().swap(ia, ib);
        self.assigned().swap(ia, ib);
        if let Some(control) = self.control.get().and_then(Weak::upgrade) {
            control.relocate(a, b);
            let cells = self.cells();
            for slot in [a, b] {
                let compiled = cells[usize::from(slot.index())].load();
                control.send_config(slot, self.config_for(&compiled, slot));
            }
        }
    }
}

impl SlotProfiles for SlotRegistry {
    fn preferred_slot<'a>(&'a self, device: &'a Uuid) -> PreferredFuture<'a> {
        Box::pin(async move {
            let device = self.devices.find(device.as_str()).await.ok().flatten()?;
            device.preferred_slot.and_then(SlotId::new)
        })
    }

    fn setup<'a>(&'a self, slot: SlotId, device: &'a Uuid) -> SetupFuture<'a> {
        Box::pin(async move {
            let default_id = self
                .devices
                .find(device.as_str())
                .await
                .ok()
                .flatten()
                .and_then(|d| d.default_profile_id)
                // Sem perfil próprio, vale o das configurações (Controles → perfil padrão).
                .or_else(|| {
                    self.default_profile
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone()
                });
            let (id, compiled) = match self.profiles.get().and_then(Weak::upgrade) {
                Some(profiles) => {
                    let resolved = profiles.resolve_or_default(default_id.as_deref()).await;
                    (resolved.id, resolved.compiled)
                }
                None => (
                    BUILTIN_PROFILE_ID.to_owned(),
                    Arc::new(CompiledProfile::standard()),
                ),
            };
            let index = usize::from(slot.index());
            let cell = Arc::clone(&self.cells()[index]);
            cell.store(Arc::clone(&compiled));
            self.assigned()[index] = Some(id);
            SlotSetup {
                config: self.config_for(&compiled, slot),
                profile: cell,
            }
        })
    }
}

impl ProfileHost for SlotRegistry {
    fn apply(&self, id: &str, compiled: Arc<CompiledProfile>) {
        for slot in self.slots_with(id) {
            self.cells()[usize::from(slot.index())].store(Arc::clone(&compiled));
            self.silence_if_off(slot, &compiled);
            // O gyro pode ter ligado ou desligado: o client passa a mandar (ou parar de mandar)
            // pacotes com motion.
            if let Some(control) = self.control.get().and_then(Weak::upgrade) {
                control.send_config(slot, self.config_for(&compiled, slot));
            }
        }
    }

    fn slots_using(&self, id: &str) -> u32 {
        u32::try_from(self.slots_with(id).len()).unwrap_or(u32::MAX)
    }

    fn removed(&self, id: &str, fallback: Arc<CompiledProfile>) {
        for slot in self.slots_with(id) {
            let index = usize::from(slot.index());
            self.cells()[index].store(Arc::clone(&fallback));
            self.assigned()[index] = Some(BUILTIN_PROFILE_ID.to_owned());
            if let Some(control) = self.control.get().and_then(Weak::upgrade) {
                control.send_config(slot, self.config_for(&fallback, slot));
            }
        }
    }
}
