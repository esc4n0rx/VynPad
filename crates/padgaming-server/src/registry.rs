//! O perfil de cada slot: o elo entre o `ProfileService` (banco), o `SlotManager` e o servidor de
//! controle. Implementa [`SlotProfiles`] (o que um dispositivo recebe ao conectar) e
//! [`ProfileHost`] (a troca a quente quando um perfil é editado).

use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

use padgaming_core::SlotManager;
use padgaming_driver::SlotId;
use padgaming_mapping::{CompiledProfile, ProfileCell};
use padgaming_net::control::{
    ControlServer, PreferredFuture, SetupFuture, SlotProfiles, SlotSetup,
};
use padgaming_protocol::control::{Config, Uuid};
use padgaming_storage::seed::BUILTIN_PROFILE_ID;
use padgaming_storage::{DeviceRepo, ProfileHost, ProfileService};

/// Taxa máxima de envio sugerida ao client (Hz).
const SEND_RATE_HZ: u16 = 250;

/// Os 4 `ProfileCell`s (um por slot) e qual perfil cada slot está usando.
pub struct SlotRegistry {
    cells: Mutex<[Arc<ProfileCell>; 4]>,
    assigned: Mutex<[Option<String>; 4]>,
    slots: Arc<SlotManager>,
    devices: DeviceRepo,
    profiles: OnceLock<Weak<ProfileService>>,
    control: OnceLock<Weak<ControlServer>>,
}

impl SlotRegistry {
    pub(crate) fn new(slots: Arc<SlotManager>, devices: DeviceRepo) -> Self {
        let cell = || Arc::new(ProfileCell::new(CompiledProfile::standard()));
        Self {
            cells: Mutex::new([cell(), cell(), cell(), cell()]),
            assigned: Mutex::new(Default::default()),
            slots,
            devices,
            profiles: OnceLock::new(),
            control: OnceLock::new(),
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

    fn cells(&self) -> std::sync::MutexGuard<'_, [Arc<ProfileCell>; 4]> {
        self.cells.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn assigned(&self) -> std::sync::MutexGuard<'_, [Option<String>; 4]> {
        self.assigned.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn config_for(compiled: &CompiledProfile, slot: SlotId) -> Config {
        Config {
            slot: slot.index(),
            send_rate_hz: SEND_RATE_HZ,
            gyro_enabled: compiled.gyro_enabled(),
            // O rumble chega com a T-204.
            rumble_enabled: false,
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
        self.assigned()[index] = Some(resolved.id.clone());
        if let Some(control) = self.control.get().and_then(Weak::upgrade) {
            control.send_config(slot, Self::config_for(&resolved.compiled, slot));
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
                control.send_config(slot, Self::config_for(&compiled, slot));
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
                .and_then(|d| d.default_profile_id);
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
                config: Self::config_for(&compiled, slot),
                profile: cell,
            }
        })
    }
}

impl ProfileHost for SlotRegistry {
    fn apply(&self, id: &str, compiled: Arc<CompiledProfile>) {
        for slot in self.slots_with(id) {
            self.cells()[usize::from(slot.index())].store(Arc::clone(&compiled));
            // O gyro pode ter ligado ou desligado: o client passa a mandar (ou parar de mandar)
            // pacotes com motion.
            if let Some(control) = self.control.get().and_then(Weak::upgrade) {
                control.send_config(slot, Self::config_for(&compiled, slot));
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
                control.send_config(slot, Self::config_for(&fallback, slot));
            }
        }
    }
}
