//! O canal de controle do app contra o server de controle TLS de verdade (loopback + driver de
//! mentira): pareamento por QR e por PIN, reconexão e as recusas.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{connect, phone, pinned, rig, rig_with, token_credential};
use vynpad_client_core::{
    ConfigInfo, ControlConnection, CoreError, Credential, SessionListener, TrustPolicy, WelcomeInfo,
};
use vynpad_protocol::control::ByeReason;

fn welcome_of(connection: &ControlConnection) -> WelcomeInfo {
    connection.welcome()
}

#[tokio::test(flavor = "multi_thread")]
async fn qr_pairing_pins_the_certificate_and_issues_the_device_token() {
    let rig = rig().await;
    let connection = connect(&rig, pinned(&rig), token_credential(&rig))
        .await
        .unwrap();
    let w = welcome_of(&connection);

    assert_eq!(w.server_name, "PC de teste");
    assert_eq!(w.slot, 0);
    assert_eq!(w.fingerprint, rig.fingerprint.to_vec());
    assert_eq!(w.udp_key.len(), 32);
    assert_eq!(w.device_token.as_ref().map(Vec::len), Some(32));
    assert!(w.session_id > 0);
    connection.disconnect();
}

#[tokio::test(flavor = "multi_thread")]
async fn pin_pairing_works_with_the_short_fingerprint_from_discovery() {
    let rig = rig().await;
    let short = hex::encode(&rig.fingerprint[..8]);
    let pin = rig.pairing.issue_pin().expose().clone();

    let connection = connect(
        &rig,
        TrustPolicy::ShortFingerprint { hex: short },
        Credential::Pin { pin },
    )
    .await
    .unwrap();
    let w = welcome_of(&connection);
    assert!(w.device_token.is_some());
    assert_eq!(w.fingerprint, rig.fingerprint.to_vec());
    connection.disconnect();
}

#[tokio::test(flavor = "multi_thread")]
async fn first_use_trust_reports_the_fingerprint_to_be_pinned() {
    let rig = rig().await;
    let pin = rig.pairing.issue_pin().expose().clone();
    let connection = connect(&rig, TrustPolicy::FirstUse, Credential::Pin { pin })
        .await
        .unwrap();
    assert_eq!(
        welcome_of(&connection).fingerprint,
        rig.fingerprint.to_vec()
    );
    connection.disconnect();
}

#[tokio::test(flavor = "multi_thread")]
async fn reconnecting_with_the_device_token_keeps_the_slot_and_issues_no_new_token() {
    let rig = rig().await;
    let first = connect(&rig, pinned(&rig), token_credential(&rig))
        .await
        .unwrap();
    let token = welcome_of(&first).device_token.unwrap();
    first.disconnect();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let again = connect(&rig, pinned(&rig), Credential::DeviceToken { token })
        .await
        .unwrap();
    let w = welcome_of(&again);
    assert_eq!(w.slot, 0);
    assert!(
        w.device_token.is_none(),
        "o token só vem no primeiro pareamento"
    );
    again.disconnect();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_pin_is_rejected_as_bad_auth() {
    let rig = rig().await;
    let _real = rig.pairing.issue_pin();
    let err = connect(
        &rig,
        pinned(&rig),
        Credential::Pin {
            pin: "000000".to_owned(),
        },
    )
    .await
    .err()
    .unwrap();
    assert_eq!(
        err,
        CoreError::Rejected {
            reason: "bad_auth".to_owned(),
            retry_after_s: None
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn five_wrong_pins_lock_the_server_for_a_while() {
    let rig = rig().await;
    let _real = rig.pairing.issue_pin();
    let mut last = None;
    for _ in 0..6 {
        last = connect(
            &rig,
            pinned(&rig),
            Credential::Pin {
                pin: "000000".to_owned(),
            },
        )
        .await
        .err();
    }
    match last.unwrap() {
        CoreError::Rejected {
            reason,
            retry_after_s,
        } => {
            assert_eq!(reason, "locked");
            assert!(retry_after_s.is_some_and(|s| s > 0));
        }
        other => panic!("esperava locked, veio {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_different_certificate_is_refused_before_any_credential_is_sent() {
    let rig = rig().await;
    let wrong_full = TrustPolicy::Pinned {
        fingerprint: vec![0xAA; 32],
    };
    let err = connect(&rig, wrong_full, token_credential(&rig))
        .await
        .err()
        .unwrap();
    assert_eq!(err, CoreError::FingerprintMismatch);

    let wrong_short = TrustPolicy::ShortFingerprint {
        hex: "0011223344556677".to_owned(),
    };
    let err = connect(
        &rig,
        wrong_short,
        Credential::Pin {
            pin: "123456".to_owned(),
        },
    )
    .await
    .err()
    .unwrap();
    assert_eq!(err, CoreError::FingerprintMismatch);
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_arguments_and_unreachable_servers_are_reported_apart() {
    let rig = rig().await;
    let err = tokio::task::spawn_blocking(|| {
        ControlConnection::connect(
            "not-an-ip",
            47800,
            TrustPolicy::FirstUse,
            phone(),
            Credential::Pin {
                pin: "123456".to_owned(),
            },
        )
    })
    .await
    .unwrap()
    .err()
    .unwrap();
    assert!(matches!(err, CoreError::InvalidArgument { .. }));

    let err = connect(
        &rig,
        TrustPolicy::ShortFingerprint {
            hex: "zz".to_owned(),
        },
        Credential::Pin {
            pin: "123456".to_owned(),
        },
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(err, CoreError::InvalidArgument { .. }));

    // Porta onde ninguém escuta: recusa de conexão, não de credencial.
    let err = tokio::task::spawn_blocking(|| {
        ControlConnection::connect(
            "127.0.0.1",
            1,
            TrustPolicy::FirstUse,
            phone(),
            Credential::Pin {
                pin: "123456".to_owned(),
            },
        )
    })
    .await
    .unwrap()
    .err()
    .unwrap();
    assert!(matches!(err, CoreError::Network { .. }));
}

#[derive(Default)]
struct Spy {
    configs: Mutex<Vec<ConfigInfo>>,
    closed: Mutex<Vec<String>>,
}

impl SessionListener for Spy {
    fn on_config(&self, config: ConfigInfo) {
        self.configs.lock().unwrap().push(config);
    }
    fn on_closed(&self, reason: String) {
        self.closed.lock().unwrap().push(reason);
    }
}

async fn closed_reason(spy: &Spy) -> String {
    for _ in 0..200 {
        if let Some(reason) = spy.closed.lock().unwrap().first().cloned() {
            return reason;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("a sessão não terminou");
}

/// O server derruba quem fica 1 s em silêncio; o keepalive responde aos pings e mantém a sessão.
#[tokio::test(flavor = "multi_thread")]
async fn the_keepalive_keeps_the_session_alive_past_the_servers_silence_limit() {
    let rig = rig_with(Duration::from_secs(1)).await;
    let connection = connect(&rig, pinned(&rig), token_credential(&rig))
        .await
        .unwrap();
    let spy = Arc::new(Spy::default());
    connection.start(spy.clone());

    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(rig.control.session_count(), 1, "o server derrubou a sessão");
    assert!(spy.closed.lock().unwrap().is_empty());

    connection.disconnect();
    assert_eq!(closed_reason(&spy).await, "local");
}

/// Sem o keepalive o server derruba a conexão (prova de que o teste acima mede algo).
#[tokio::test(flavor = "multi_thread")]
async fn without_the_keepalive_the_server_drops_the_session() {
    let rig = rig_with(Duration::from_secs(1)).await;
    let _connection = connect(&rig, pinned(&rig), token_credential(&rig))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(rig.control.session_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kick_from_the_server_reaches_the_listener_as_bye_kicked() {
    let rig = rig().await;
    let connection = connect(&rig, pinned(&rig), token_credential(&rig))
        .await
        .unwrap();
    let spy = Arc::new(Spy::default());
    connection.start(spy.clone());
    let id = welcome_of(&connection).session_id;

    assert!(rig.control.end_session(id, ByeReason::Kicked));
    assert_eq!(closed_reason(&spy).await, "bye:kicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dropped_connection_reaches_the_listener_as_network_or_bye() {
    let rig = rig().await;
    let connection = connect(&rig, pinned(&rig), token_credential(&rig))
        .await
        .unwrap();
    let spy = Arc::new(Spy::default());
    connection.start(spy.clone());

    rig.control.shutdown().await;
    let reason = closed_reason(&spy).await;
    assert!(
        reason == "network" || reason.starts_with("bye:"),
        "motivo inesperado: {reason}"
    );
}
