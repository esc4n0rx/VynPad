//! Ambiente de teste compartilhado: o server de controle e de input de verdade, em loopback,
//! com um driver de mentira que guarda os relatórios recebidos.

#![allow(dead_code)] // Cada arquivo de teste usa só uma parte.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vynpad_client_core::{ControlConnection, CoreError, Credential, DeviceInfo, TrustPolicy};
use vynpad_core::{SlotConfig, SlotManager, SystemClock};
use vynpad_driver::MockDriver;
use vynpad_net::control::{ControlConfig, ControlDeps, ControlServer, DefaultProfiles};
use vynpad_net::identity::ServerIdentity;
use vynpad_net::input::{InputConfig, InputServer};
use vynpad_net::pairing::{DeviceRecord, DeviceStore, PairingConfig, PairingManager, StoreError};
use vynpad_protocol::control::Uuid;

#[derive(Clone, Default)]
pub struct MemStore(Arc<Mutex<HashMap<String, DeviceRecord>>>);

impl DeviceStore for MemStore {
    fn find(&self, id: &Uuid) -> Result<Option<DeviceRecord>, StoreError> {
        Ok(self.0.lock().unwrap().get(id.as_str()).cloned())
    }
    fn save(&self, record: DeviceRecord) -> Result<(), StoreError> {
        self.0
            .lock()
            .unwrap()
            .insert(record.device_id.as_str().to_owned(), record);
        Ok(())
    }
}

pub struct Rig {
    pub control: ControlServer,
    pub pairing: Arc<PairingManager<MemStore>>,
    pub driver: Arc<MockDriver>,
    pub fingerprint: [u8; 32],
    pub input: Arc<InputServer>,
}

pub async fn rig() -> Rig {
    rig_with(Duration::from_secs(15)).await
}

/// `dead_after`: quanto silêncio o server tolera antes de derrubar a conexão.
pub async fn rig_with(dead_after: Duration) -> Rig {
    let driver = Arc::new(MockDriver::default());
    let slots = Arc::new(SlotManager::with_clock(
        driver.clone(),
        SlotConfig::default(),
        Arc::new(SystemClock),
    ));
    let input = Arc::new(
        InputServer::start(
            InputConfig {
                bind: "127.0.0.1:0".parse().unwrap(),
                poll_interval: Duration::from_millis(5),
                ..InputConfig::default()
            },
            slots.clone(),
        )
        .unwrap(),
    );
    let identity = ServerIdentity::generate().unwrap();
    let fingerprint = identity.fingerprint();
    let pairing = Arc::new(PairingManager::new(
        MemStore::default(),
        PairingConfig::default(),
    ));
    let control = ControlServer::start(
        ControlConfig {
            bind: "127.0.0.1:0".parse().unwrap(),
            hello_timeout: Duration::from_secs(2),
            ping_after: Duration::from_millis(100),
            dead_after,
            ..ControlConfig::default()
        },
        ControlDeps {
            identity,
            server_name: "PC de teste".into(),
            pairing: pairing.clone(),
            slots,
            input: input.clone(),
            profiles: Arc::new(DefaultProfiles::default()),
        },
    )
    .await
    .unwrap();
    Rig {
        control,
        pairing,
        driver,
        fingerprint,
        input,
    }
}

pub fn phone() -> DeviceInfo {
    DeviceInfo {
        device_id: "00000000-0000-4000-8000-000000000001".to_owned(),
        name: "Celular de teste".to_owned(),
        model: "Teste 1".to_owned(),
        os_ver: "15".to_owned(),
        app_ver: "0.1.0".to_owned(),
        gyro: true,
        rumble: true,
    }
}

/// Roda o `connect` bloqueante fora das threads do runtime, como o app faz com `Dispatchers.IO`.
pub async fn connect(
    rig: &Rig,
    trust: TrustPolicy,
    credential: Credential,
) -> Result<Arc<ControlConnection>, CoreError> {
    let port = rig.control.local_addr().port();
    tokio::task::spawn_blocking(move || {
        ControlConnection::connect("127.0.0.1", port, trust, phone(), credential)
    })
    .await
    .unwrap()
}

pub fn pinned(rig: &Rig) -> TrustPolicy {
    TrustPolicy::Pinned {
        fingerprint: rig.fingerprint.to_vec(),
    }
}

pub fn token_credential(rig: &Rig) -> Credential {
    Credential::PairingToken {
        token: rig.pairing.issue_pairing_token().expose().to_vec(),
    }
}
