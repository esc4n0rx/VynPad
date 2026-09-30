//! O `DeviceStore` do pareamento (T-103) sobre o SQLite.

use std::collections::HashSet;
use std::sync::{Arc, PoisonError, RwLock};

use padgaming_net::pairing::{DeviceRecord, DeviceStore, StoreError};
use padgaming_protocol::control::Uuid;
use padgaming_storage::{DeviceRepo, NewPairing};
use tokio::runtime::Handle;

use padgaming_storage::seed::now_ms;

/// Os `device_id`s pareados e não revogados, em memória: a lista de "dispositivos próximos" pergunta
/// se cada aparelho já é pareado, de forma síncrona e várias vezes por segundo.
#[derive(Debug, Default)]
pub struct PairedSet(RwLock<HashSet<String>>);

impl PairedSet {
    pub(crate) fn replace(&self, ids: impl IntoIterator<Item = String>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = ids.into_iter().collect();
    }

    pub(crate) fn insert(&self, id: &str) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id.to_owned());
    }

    pub(crate) fn remove(&self, id: &str) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id);
    }

    /// `id` é de um dispositivo pareado e não revogado?
    pub fn contains(&self, id: &str) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(id)
    }
}

/// [`DeviceStore`] sobre o [`DeviceRepo`].
///
/// A trait é síncrona: o servidor de controle a chama de dentro de um `spawn_blocking`, então
/// `Handle::block_on` é seguro aqui (nunca chame isto de uma thread do runtime).
pub struct DeviceStoreAdapter {
    repo: DeviceRepo,
    runtime: Handle,
    paired: Arc<PairedSet>,
}

impl DeviceStoreAdapter {
    /// Adaptador que usa `runtime` para as consultas.
    pub const fn new(repo: DeviceRepo, runtime: Handle, paired: Arc<PairedSet>) -> Self {
        Self {
            repo,
            runtime,
            paired,
        }
    }
}

fn store_error(error: impl std::fmt::Display) -> StoreError {
    StoreError(error.to_string())
}

impl DeviceStore for DeviceStoreAdapter {
    fn find(&self, device_id: &Uuid) -> Result<Option<DeviceRecord>, StoreError> {
        let Some(device) = self
            .runtime
            .block_on(self.repo.find(device_id.as_str()))
            .map_err(store_error)?
        else {
            return Ok(None);
        };
        Ok(Some(DeviceRecord {
            device_id: device_id.clone(),
            name: device.name,
            model: device.model.unwrap_or_default(),
            os_version: device.os_version.unwrap_or_default(),
            token_hash: device.token_hash,
            revoked: device.revoked,
        }))
    }

    fn save(&self, record: DeviceRecord) -> Result<(), StoreError> {
        let non_empty = |s: String| (!s.is_empty()).then_some(s);
        self.runtime
            .block_on(self.repo.upsert_pairing(&NewPairing {
                id: record.device_id.as_str().to_owned(),
                name: record.name,
                model: non_empty(record.model),
                os_version: non_empty(record.os_version),
                token_hash: record.token_hash,
                now: now_ms(),
            }))
            .map_err(store_error)?;
        // Parear de novo remove a revogação.
        self.paired.insert(record.device_id.as_str());
        Ok(())
    }
}
