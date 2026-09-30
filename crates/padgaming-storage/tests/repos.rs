//! Repositórios e serviços de dados sobre o banco de verdade (em memória ou em arquivo).
#![allow(clippy::float_cmp)] // os floats dos perfis são gravados e lidos sem conta no meio

use std::sync::{Arc, Mutex};

use padgaming_mapping::{CompiledProfile, Profile};
use padgaming_storage::seed::{BUILTIN_PROFILE_ID, BUILTIN_PROFILE_NAME};
use padgaming_storage::{
    AppSettings, ConnectionLog, DataError, Database, DeviceDefaults, DeviceRepo, Language,
    LogEvent, LogQuery, NewPairing, Ports, ProfileHost, ProfileInput, ProfileService, SettingsRepo,
    SettingsService, Theme,
};

const NOW: i64 = 1_800_000_000_000;

type Case = (&'static str, fn(&mut AppSettings));

fn pairing(id: &str, hash: u8) -> NewPairing {
    NewPairing {
        id: id.to_owned(),
        name: "Pixel 8".into(),
        model: Some("Pixel 8".into()),
        os_version: Some("Android 16".into()),
        token_hash: [hash; 32],
        now: NOW,
    }
}

fn input(name: &str) -> ProfileInput {
    ProfileInput {
        name: name.into(),
        description: Some("descrição".into()),
        icon: Some("bolt".into()),
        profile: Profile::identity(),
    }
}

/// Anota o que o serviço de perfis mandou aplicar nos controles.
#[derive(Default)]
struct Recorder {
    applied: Mutex<Vec<(String, String)>>,
    removed: Mutex<Vec<String>>,
    in_use: Mutex<u32>,
}

impl ProfileHost for Recorder {
    fn apply(&self, id: &str, compiled: Arc<CompiledProfile>) {
        self.applied
            .lock()
            .unwrap()
            .push((id.to_owned(), compiled.name().to_owned()));
    }
    fn slots_using(&self, _id: &str) -> u32 {
        *self.in_use.lock().unwrap()
    }
    fn removed(&self, id: &str, fallback: Arc<CompiledProfile>) {
        assert_eq!(fallback.name(), BUILTIN_PROFILE_NAME);
        self.removed.lock().unwrap().push(id.to_owned());
    }
}

async fn profiles() -> (Database, ProfileService, Arc<Recorder>) {
    let db = Database::in_memory().await.unwrap();
    let host = Arc::new(Recorder::default());
    let service = ProfileService::new(db.pool().clone(), host.clone());
    (db, service, host)
}

// ------------------------------------------------------------------ configurações

#[tokio::test]
async fn settings_start_as_the_documented_defaults() {
    let db = Database::in_memory().await.unwrap();
    let settings = SettingsRepo::new(db.pool().clone()).load().await.unwrap();
    assert_eq!(settings, AppSettings::default());
    assert_eq!(settings.language, Language::PtBr);
    assert_eq!(settings.theme, Theme::Dark);
    assert_eq!(settings.ports.control, 47800);
}

#[tokio::test]
async fn settings_updates_persist_across_restarts_and_notify_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).await.unwrap();
    let service = SettingsService::load(SettingsRepo::new(db.pool().clone()))
        .await
        .unwrap();
    let mut events = service.subscribe();

    let updated = service
        .modify(|s| {
            s.language = Language::En;
            s.theme = Theme::System;
            s.autostart = true;
            s.ports = Ports {
                control: 50000,
                input: 50001,
                discovery: 50002,
            };
        })
        .await
        .unwrap();
    assert_eq!(updated.language, Language::En);
    assert_eq!(*events.recv().await.unwrap(), *updated);

    // Repetir o mesmo valor não grava nem avisa.
    service.update((*service.get()).clone()).await.unwrap();
    assert!(events.try_recv().is_err());
    db.close().await;

    let db = Database::open(dir.path()).await.unwrap();
    let again = SettingsService::load(SettingsRepo::new(db.pool().clone()))
        .await
        .unwrap();
    assert_eq!(*again.get(), *updated);
    assert!(again.get().autostart);
}

#[tokio::test]
async fn invalid_settings_are_refused_with_a_reason_and_change_nothing() {
    let db = Database::in_memory().await.unwrap();
    let service = SettingsService::load(SettingsRepo::new(db.pool().clone()))
        .await
        .unwrap();
    let original = service.get();
    let cases: [Case; 5] = [
        ("cor", |s| s.accent = "azul".into()),
        ("cor", |s| s.accent = "#12345".into()),
        ("1024", |s| s.ports.control = 80),
        ("diferentes", |s| s.ports.input = s.ports.control),
        ("mDNS", |s| s.ports.discovery = 5353),
    ];
    for (needle, change) in cases {
        let error = service.modify(change).await.unwrap_err();
        assert!(
            matches!(&error, DataError::Invalid(why) if why.contains(needle)),
            "{error}"
        );
    }
    assert_eq!(*service.get(), *original);
    let stored = SettingsRepo::new(db.pool().clone()).load().await.unwrap();
    assert_eq!(stored, *original);
}

#[tokio::test]
async fn a_corrupt_or_unknown_setting_never_stops_the_app() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();
    sqlx::query("UPDATE settings SET value = 'não é json' WHERE key = 'theme'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE settings SET value = '\"klingon\"' WHERE key = 'language'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE settings SET value = 'true' WHERE key = 'autostart'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('futuro', '1', 1)")
        .execute(pool)
        .await
        .unwrap();

    let settings = SettingsRepo::new(pool.clone()).load().await.unwrap();
    // Os dois campos estragados voltam ao padrão; o resto vale o que está no banco.
    assert_eq!(settings.theme, Theme::Dark);
    assert_eq!(settings.language, Language::PtBr);
    assert!(settings.autostart);
}

// ------------------------------------------------------------------ perfis

#[tokio::test]
async fn profiles_are_created_listed_and_read_back_identically() {
    let (_db, service, _) = profiles().await;
    let created = service.create(input("  Meu perfil  ")).await.unwrap();
    assert_eq!(created.summary.name, "Meu perfil");
    assert!(!created.summary.is_builtin);
    assert_eq!(created.profile.name, "Meu perfil");

    let read = service.get(&created.summary.id).await.unwrap();
    assert_eq!(read, created);

    let list = service.list().await.unwrap();
    // O embutido vem primeiro.
    assert_eq!(list[0].id, BUILTIN_PROFILE_ID);
    assert_eq!(list[1].id, created.summary.id);
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn profile_text_and_mapping_are_validated_before_anything_is_stored() {
    let (_db, service, _) = profiles().await;
    for bad in ["", "   ", &"x".repeat(65)] {
        assert!(matches!(
            service.create(input(bad)).await,
            Err(DataError::Invalid(_))
        ));
    }
    let mut long_description = input("ok");
    long_description.description = Some("d".repeat(501));
    assert!(matches!(
        service.create(long_description).await,
        Err(DataError::Invalid(_))
    ));

    let mut broken = input("quebrado");
    broken.profile.sticks.left.sensitivity = 99.0;
    let error = service.create(broken).await.unwrap_err();
    assert!(
        matches!(&error, DataError::Invalid(why) if why.contains("sensitivity")),
        "{error}"
    );
    assert_eq!(service.list().await.unwrap().len(), 1, "nada foi gravado");
}

#[tokio::test]
async fn saving_a_profile_hot_swaps_it_on_the_active_slots() {
    let (_db, service, host) = profiles().await;
    let created = service.create(input("Original")).await.unwrap();
    let id = created.summary.id.clone();

    let mut edit = input("Editado");
    edit.profile.sticks.left.sensitivity = 2.0;
    let saved = service.save(&id, edit).await.unwrap();
    assert_eq!(saved.summary.name, "Editado");
    assert_eq!(saved.summary.created_at, created.summary.created_at);
    assert!(saved.summary.updated_at >= created.summary.updated_at);
    assert_eq!(
        service
            .get(&id)
            .await
            .unwrap()
            .profile
            .sticks
            .left
            .sensitivity,
        2.0
    );

    // O host (o SlotManager) recebeu o perfil compilado na hora, sem reconectar.
    assert_eq!(
        *host.applied.lock().unwrap(),
        [(id.clone(), "Editado".to_owned())]
    );

    // Um perfil inválido não é gravado nem aplicado.
    let mut bad = input("Ruim");
    bad.profile.sticks.left.sensitivity = 0.0;
    assert!(service.save(&id, bad).await.is_err());
    assert_eq!(service.get(&id).await.unwrap().summary.name, "Editado");
    assert_eq!(host.applied.lock().unwrap().len(), 1);

    assert!(matches!(
        service.save("nao-existe", input("x")).await,
        Err(DataError::NotFound(_))
    ));
}

#[tokio::test]
async fn the_builtin_profile_cannot_be_edited_or_deleted_but_can_be_duplicated() {
    let (_db, service, host) = profiles().await;
    assert!(matches!(
        service.save(BUILTIN_PROFILE_ID, input("x")).await,
        Err(DataError::Builtin)
    ));
    // Nem com `confirm`.
    assert!(matches!(
        service.delete(BUILTIN_PROFILE_ID, true).await,
        Err(DataError::Builtin)
    ));
    assert!(host.applied.lock().unwrap().is_empty());

    let copy = service.duplicate(BUILTIN_PROFILE_ID, None).await.unwrap();
    assert_eq!(copy.summary.name, "Padrão (cópia)");
    assert!(!copy.summary.is_builtin);
    assert_ne!(copy.summary.id, BUILTIN_PROFILE_ID);
    assert_eq!(copy.profile.sticks, Profile::standard().sticks);
    let named = service
        .duplicate(&copy.summary.id, Some("Outro nome"))
        .await
        .unwrap();
    assert_eq!(named.summary.name, "Outro nome");
}

#[tokio::test]
async fn deleting_a_profile_in_use_needs_confirmation() {
    let (db, service, host) = profiles().await;
    let devices = DeviceRepo::new(db.pool().clone());
    let profile = service.create(input("Em uso")).await.unwrap();
    let id = profile.summary.id.clone();
    devices.upsert_pairing(&pairing("d1", 1)).await.unwrap();
    devices
        .set_defaults(
            "d1",
            &DeviceDefaults {
                default_profile_id: Some(id.clone()),
                ..DeviceDefaults::default()
            },
        )
        .await
        .unwrap();

    // Em uso por um dispositivo: recusa sem confirmar e nada muda.
    let error = service.delete(&id, false).await.unwrap_err();
    assert!(
        matches!(
            error,
            DataError::InUse {
                devices: 1,
                slots: 0
            }
        ),
        "{error}"
    );
    assert_eq!(service.list().await.unwrap().len(), 2);

    // Em uso por um controle ativo também.
    *host.in_use.lock().unwrap() = 2;
    let error = service.delete(&id, false).await.unwrap_err();
    assert!(matches!(
        error,
        DataError::InUse {
            devices: 1,
            slots: 2
        }
    ));

    // Confirmado: apaga, o dispositivo fica sem perfil padrão e os controles ativos vão para o
    // perfil Padrão.
    service.delete(&id, true).await.unwrap();
    assert_eq!(service.list().await.unwrap().len(), 1);
    assert_eq!(
        devices
            .find("d1")
            .await
            .unwrap()
            .unwrap()
            .default_profile_id,
        None
    );
    assert_eq!(*host.removed.lock().unwrap(), [id]);
}

#[tokio::test]
async fn deleting_an_unused_profile_needs_no_confirmation() {
    let (_db, service, host) = profiles().await;
    let profile = service.create(input("Solto")).await.unwrap();
    service.delete(&profile.summary.id, false).await.unwrap();
    assert!(host.removed.lock().unwrap().is_empty());
    assert!(matches!(
        service.delete(&profile.summary.id, false).await,
        Err(DataError::NotFound(_))
    ));
}

#[tokio::test]
async fn padprofile_export_and_import_round_trip() {
    let (_db, service, _) = profiles().await;
    let mut original = input("Para compartilhar");
    original.profile.sticks.right.invert_y = true;
    let created = service.create(original).await.unwrap();

    let text = service.export(&created.summary.id).await.unwrap();
    let file: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(file["format"], "padprofile");
    assert_eq!(file["version"], 1);
    assert_eq!(file["name"], "Para compartilhar");

    let imported = service.import(&text).await.unwrap();
    assert_ne!(imported.summary.id, created.summary.id, "id sempre novo");
    assert!(!imported.summary.is_builtin);
    assert_eq!(imported.summary.name, "Para compartilhar");
    assert_eq!(imported.summary.icon.as_deref(), Some("bolt"));
    assert!(imported.profile.sticks.right.invert_y);
    assert_eq!(service.list().await.unwrap().len(), 3);

    // O embutido também exporta, e importa como perfil comum.
    let builtin = service.export(BUILTIN_PROFILE_ID).await.unwrap();
    assert!(!service.import(&builtin).await.unwrap().summary.is_builtin);
}

#[tokio::test]
async fn importing_garbage_gives_clear_errors_and_stores_nothing() {
    let (_db, service, _) = profiles().await;
    let valid = service.export(BUILTIN_PROFILE_ID).await.unwrap();
    let with = |edit: fn(&mut serde_json::Value)| {
        let mut value: serde_json::Value = serde_json::from_str(&valid).unwrap();
        edit(&mut value);
        value.to_string()
    };
    let cases = [
        ("não é um perfil", "isto não é json".to_owned()),
        ("não é um perfil", "{}".to_owned()),
        (".padprofile", with(|v| v["format"] = "outra-coisa".into())),
        ("versão 2", with(|v| v["version"] = 2.into())),
        ("nome do perfil", with(|v| v["name"] = "".into())),
        (
            "sensitivity",
            with(|v| v["profile"]["sticks"]["left"]["sensitivity"] = 50.into()),
        ),
        ("KiB", "x".repeat(300 * 1024)),
    ];
    for (needle, text) in cases {
        let error = service.import(&text).await.unwrap_err();
        assert!(
            matches!(&error, DataError::Import(why) if why.contains(needle)),
            "{needle}: {error}"
        );
    }
    assert_eq!(service.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn restoring_the_default_profile_puts_it_back_and_applies_it() {
    let (db, service, host) = profiles().await;
    sqlx::query("UPDATE profiles SET config = 'lixo', name = 'Hackeado' WHERE id = ?")
        .bind(BUILTIN_PROFILE_ID)
        .execute(db.pool())
        .await
        .unwrap();
    assert!(service.get(BUILTIN_PROFILE_ID).await.is_err(), "corrompido");

    service.restore_builtin().await.unwrap();
    let restored = service.get(BUILTIN_PROFILE_ID).await.unwrap();
    assert_eq!(restored.summary.name, BUILTIN_PROFILE_NAME);
    assert_eq!(restored.profile, Profile::standard());
    assert_eq!(host.applied.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_missing_or_corrupt_profile_falls_back_to_the_default_one() {
    let (db, service, _) = profiles().await;
    let fallback = service.compiled_or_default(None).await;
    assert_eq!(fallback.name(), BUILTIN_PROFILE_NAME);
    assert_eq!(
        service.compiled_or_default(Some("nao-existe")).await.name(),
        BUILTIN_PROFILE_NAME
    );

    let created = service.create(input("Bom")).await.unwrap();
    assert_eq!(
        service
            .compiled_or_default(Some(&created.summary.id))
            .await
            .name(),
        "Bom"
    );
    sqlx::query("UPDATE profiles SET config = 'lixo' WHERE id = ?")
        .bind(&created.summary.id)
        .execute(db.pool())
        .await
        .unwrap();
    assert_eq!(
        service
            .compiled_or_default(Some(&created.summary.id))
            .await
            .name(),
        BUILTIN_PROFILE_NAME
    );
    // Regravar um perfil corrompido é como o usuário o conserta.
    service
        .save(&created.summary.id, input("Consertado"))
        .await
        .unwrap();
    assert_eq!(
        service.get(&created.summary.id).await.unwrap().summary.name,
        "Consertado"
    );
}

// ------------------------------------------------------------------ dispositivos

#[tokio::test]
async fn a_pairing_is_stored_without_the_token_and_logged() {
    let db = Database::in_memory().await.unwrap();
    let devices = DeviceRepo::new(db.pool().clone());
    assert!(devices.find("d1").await.unwrap().is_none());

    devices.upsert_pairing(&pairing("d1", 7)).await.unwrap();
    let device = devices.find("d1").await.unwrap().unwrap();
    assert_eq!(device.name, "Pixel 8");
    assert_eq!(device.token_hash, [7; 32]);
    assert_eq!(device.paired_at, NOW);
    assert!(!device.revoked);
    assert_eq!(device.preferred_slot, None);

    let log = ConnectionLog::new(db.pool().clone())
        .list(&LogQuery::default())
        .await
        .unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(
        (log[0].event.as_str(), log[0].device_id.as_deref()),
        ("paired", Some("d1"))
    );
}

#[tokio::test]
async fn pairing_again_replaces_the_token_and_clears_the_revocation_but_keeps_preferences() {
    let db = Database::in_memory().await.unwrap();
    let devices = DeviceRepo::new(db.pool().clone());
    devices.upsert_pairing(&pairing("d1", 1)).await.unwrap();
    devices
        .set_defaults(
            "d1",
            &DeviceDefaults {
                preferred_slot: Some(2),
                default_profile_id: Some(BUILTIN_PROFILE_ID.into()),
                color: Some("#FF0000".into()),
            },
        )
        .await
        .unwrap();
    devices.revoke("d1", NOW + 1).await.unwrap();
    assert!(devices.find("d1").await.unwrap().unwrap().revoked);

    let mut again = pairing("d1", 9);
    again.now = NOW + 5;
    again.name = "Pixel novo".into();
    devices.upsert_pairing(&again).await.unwrap();
    let device = devices.find("d1").await.unwrap().unwrap();
    assert_eq!(device.token_hash, [9; 32]);
    assert!(!device.revoked);
    assert_eq!(device.name, "Pixel novo");
    assert_eq!(device.paired_at, NOW + 5);
    assert_eq!(device.preferred_slot, Some(2));
    assert_eq!(
        device.default_profile_id.as_deref(),
        Some(BUILTIN_PROFILE_ID)
    );
    assert_eq!(device.color.as_deref(), Some("#FF0000"));
    assert_eq!(devices.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn rename_revoke_forget_and_defaults_validate_their_input() {
    let db = Database::in_memory().await.unwrap();
    let devices = DeviceRepo::new(db.pool().clone());
    devices.upsert_pairing(&pairing("d1", 1)).await.unwrap();

    assert_eq!(
        devices.rename("d1", "  Celular da sala ").await.unwrap(),
        "Celular da sala"
    );
    for bad in ["", "   ", &"n".repeat(65)] {
        assert!(matches!(
            devices.rename("d1", bad).await,
            Err(DataError::Invalid(_))
        ));
    }
    assert!(matches!(
        devices.rename("nao", "x").await,
        Err(DataError::NotFound(_))
    ));

    assert!(matches!(
        devices
            .set_defaults(
                "d1",
                &DeviceDefaults {
                    preferred_slot: Some(4),
                    ..Default::default()
                }
            )
            .await,
        Err(DataError::Invalid(_))
    ));
    assert!(matches!(
        devices
            .set_defaults(
                "d1",
                &DeviceDefaults {
                    color: Some("vermelho".into()),
                    ..Default::default()
                }
            )
            .await,
        Err(DataError::Invalid(_))
    ));
    assert!(matches!(
        devices
            .set_defaults(
                "d1",
                &DeviceDefaults {
                    default_profile_id: Some("nao-existe".into()),
                    ..Default::default()
                }
            )
            .await,
        Err(DataError::NotFound("perfil"))
    ));
    assert!(matches!(
        devices
            .set_defaults("nao", &DeviceDefaults::default())
            .await,
        Err(DataError::NotFound("dispositivo"))
    ));
    // `None` limpa.
    devices
        .set_defaults(
            "d1",
            &DeviceDefaults {
                preferred_slot: Some(1),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    devices
        .set_defaults("d1", &DeviceDefaults::default())
        .await
        .unwrap();
    assert_eq!(
        devices.find("d1").await.unwrap().unwrap().preferred_slot,
        None
    );

    devices.revoke("d1", NOW).await.unwrap();
    assert!(matches!(
        devices.revoke("nao", NOW).await,
        Err(DataError::NotFound(_))
    ));

    // Esquecer apaga o dispositivo e o histórico dele.
    let log = ConnectionLog::new(db.pool().clone());
    assert_eq!(log.list(&LogQuery::default()).await.unwrap().len(), 2); // paired + revoked
    devices.forget("d1").await.unwrap();
    assert!(devices.find("d1").await.unwrap().is_none());
    assert!(log.list(&LogQuery::default()).await.unwrap().is_empty());
    assert!(matches!(
        devices.forget("d1").await,
        Err(DataError::NotFound(_))
    ));
}

#[tokio::test]
async fn devices_are_listed_most_recently_seen_first_and_touch_keeps_the_last_ip() {
    let db = Database::in_memory().await.unwrap();
    let devices = DeviceRepo::new(db.pool().clone());
    for (id, at) in [("a", 10), ("b", 20), ("c", 30)] {
        let mut p = pairing(id, 1);
        p.now = at;
        devices.upsert_pairing(&p).await.unwrap();
    }
    devices
        .touch_seen("a", Some("192.168.0.9"), 100)
        .await
        .unwrap();
    let ids: Vec<String> = devices
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(ids, ["a", "c", "b"]);

    devices.touch_seen("a", None, 200).await.unwrap();
    let a = devices.find("a").await.unwrap().unwrap();
    assert_eq!(a.last_seen_at, Some(200));
    assert_eq!(a.last_ip.as_deref(), Some("192.168.0.9"));
}

// ------------------------------------------------------------------ histórico

#[tokio::test]
async fn the_log_filters_by_device_and_paginates_newest_first() {
    let db = Database::in_memory().await.unwrap();
    let devices = DeviceRepo::new(db.pool().clone());
    let log = ConnectionLog::new(db.pool().clone());
    devices.upsert_pairing(&pairing("d1", 1)).await.unwrap();
    devices.upsert_pairing(&pairing("d2", 2)).await.unwrap();
    for i in 0..6 {
        let (id, event) = if i % 2 == 0 {
            ("d1", LogEvent::Connected)
        } else {
            ("d2", LogEvent::Disconnected)
        };
        log.append(
            Some(id),
            event,
            Some(0),
            Some("detalhe"),
            NOW + i64::from(i),
        )
        .await
        .unwrap();
    }
    // Uma recusa de alguém desconhecido não tem dispositivo.
    log.append(None, LogEvent::Rejected, None, Some("bad_auth"), NOW + 10)
        .await
        .unwrap();

    let all = log
        .list(&LogQuery {
            limit: 3,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].event, "rejected");
    assert_eq!(all[0].device_id, None);
    assert!(all[0].id > all[1].id, "mais recente primeiro");

    let next = log
        .list(&LogQuery {
            limit: 3,
            before_id: Some(all[2].id),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(next.iter().all(|e| e.id < all[2].id));

    let only_d1 = log
        .list(&LogQuery {
            device_id: Some("d1".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(only_d1.iter().all(|e| e.device_id.as_deref() == Some("d1")));
    assert_eq!(only_d1.iter().filter(|e| e.event == "connected").count(), 3);
    assert_eq!(
        only_d1
            .iter()
            .find(|e| e.event == "connected")
            .unwrap()
            .slot,
        Some(0)
    );

    // Um dispositivo que não existe não pode ter histórico (chave estrangeira).
    assert!(
        log.append(Some("fantasma"), LogEvent::Connected, None, None, NOW)
            .await
            .is_err()
    );
}
