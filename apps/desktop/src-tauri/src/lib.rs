//! Shell Tauri do server: janela, plugins e a ponte tipada com a UI.
//!
//! A ponte (T-506) é gerada por `tauri-specta`: os commands (`commands.rs`) e os eventos
//! (`events.rs`) viram `src/lib/bindings.ts`, então mudar uma assinatura em Rust quebra o
//! `pnpm typecheck` até o frontend acompanhar. O `Core` (`vynpad-server`) mora no
//! [`AppState`]; os commands só traduzem tipos e delegam a ele.

mod autostart;
mod commands;
mod driver;
mod dto;
mod error;
mod events;
mod live;
mod logging;
mod notify;
mod shell;
mod state;
mod system;
mod tray;
mod updates;
mod window;

use std::sync::Arc;
use std::time::Duration;

use tauri::{Manager, RunEvent};
use tauri_specta::{Builder, collect_commands, collect_events};
use vynpad_server::CoreConfig;

use crate::events::InputLive;
use crate::state::AppState;

/// Quanto o app espera o `Core` se despedir dos clients ao fechar.
const EXIT_GRACE: Duration = Duration::from_secs(3);
/// Onde o `bindings.ts` é gerado, relativo à pasta `src-tauri`.
#[cfg(debug_assertions)]
const BINDINGS_PATH: &str = "../src/lib/bindings.ts";

/// Os commands e eventos expostos à UI. Genérico no runtime para os testes usarem o mock.
fn specta_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .commands(collect_commands![
            commands::app_info,
            commands::log_from_frontend,
            driver::driver_status,
            driver::open_vigem_download,
            driver::xinput_slots,
            commands::get_settings,
            commands::update_settings,
            commands::list_profiles,
            commands::get_profile,
            commands::save_profile,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::import_profile,
            commands::export_profile,
            commands::restore_default_profile,
            commands::list_devices,
            commands::rename_device,
            commands::revoke_device,
            commands::forget_device,
            commands::set_device_defaults,
            commands::get_connection_log,
            commands::get_slots,
            commands::disconnect_slot,
            commands::set_slot_paused,
            commands::swap_slots,
            commands::test_rumble,
            commands::get_slot_profiles,
            commands::set_slot_profile,
            commands::get_metrics,
            shell::take_pending_navigation,
            commands::set_ui_visible,
            system::firewall_status,
            system::fix_firewall,
            system::test_connection,
            system::open_network_settings,
            system::open_logs_dir,
            system::logs_dir,
            system::clear_history,
            system::regenerate_identity,
            system::restart_app,
            system::export_diagnostics,
            commands::start_pairing,
            commands::cancel_pairing,
            commands::answer_approval,
            commands::get_nearby_devices,
            commands::get_server_info,
            commands::list_network_interfaces,
            commands::subscribe_live_input,
            commands::unsubscribe_live_input,
            commands::reset_live_input,
            window::window_chrome,
            updates::check_for_update,
            updates::pending_update,
            updates::install_update,
        ])
        .events(collect_events![
            events::SettingsChanged,
            events::MetricsTick,
            window::WindowChromeChanged,
            shell::TrayHint,
            notify::NoticeEvent,
            shell::TrayNavigate,
            events::SlotsChanged,
            events::DevicesChanged,
            events::DevicesPaired,
            events::DevicesDiscovered,
            events::SessionStarted,
            events::SessionEnded,
            events::SessionState,
            events::ConnectionRejected,
            events::ReservationExpired,
            events::ApprovalRequested,
            events::ApprovalResolved,
            events::DriverChanged,
            events::InputLive,
            updates::UpdateAvailable,
        ])
        // Timestamps em epoch ms cabem com folga em um `number` do JS (2^53).
        .dangerously_cast_bigints_to_number()
        // No frontend os commands lançam o `AppError`, o que combina com o React Query.
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
}

/// Sobe o `Core` e liga tudo o que depende dele à webview.
fn start_core(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = vynpad_storage::default_data_dir()
        .ok_or("não achei a pasta de dados do usuário para guardar o banco")?;
    tracing::info!(dir = %data_dir.display(), "pasta de dados");
    let state = tauri::async_runtime::block_on(AppState::start(CoreConfig::new(data_dir)))?;
    let (core, live) = (Arc::clone(&state.core), Arc::clone(&state.live));
    app.manage(state);

    // Autostart e bandeja dependem do `Core`; a janela, de ele estar de pé.
    let settings = core.settings().get();
    autostart::apply(app.handle(), settings.autostart);
    events::spawn_pump(app.handle().clone(), &core);
    notify::spawn(app.handle().clone(), Arc::clone(&core));
    match tray::install(app.handle(), &core) {
        Ok(icon) => tray::spawn_updater(app.handle().clone(), Arc::clone(&core), icon),
        Err(error) => tracing::warn!(%error, "não consegui criar o ícone da bandeja"),
    }
    // Iniciar minimizado: a janela só nasce quando alguém pedir (bandeja ou segunda instância).
    if !shell::should_start_minimized(std::env::args(), settings.start_minimized) {
        shell::show_main_window(app.handle());
    }

    // O `input:live` só é amostrado enquanto alguma tela assina.
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        live.run(core.slots(), move |slot, report| {
            use tauri_specta::Event as _;
            let _ = InputLive { slot, report }.emit(&handle);
        })
        .await;
    });
    driver::spawn_watcher(app.handle());
    updates::spawn_checker(app.handle().clone());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    tracing::info!(
        app = env!("CARGO_PKG_VERSION"),
        core = vynpad_core::VERSION,
        logs = ?logging::log_dir(),
        "VynPad iniciando"
    );

    let builder = specta_builder::<tauri::Wry>();
    // Só nos builds de debug: o arquivo é versionado e o CI confere que está em dia.
    #[cfg(debug_assertions)]
    if let Err(error) = builder.export(specta_typescript::Typescript::default(), BINDINGS_PATH) {
        tracing::warn!(%error, "não consegui gerar o bindings.ts");
    }

    let app_builder = tauri::Builder::default();

    // O single-instance precisa ser o primeiro plugin registrado.
    #[cfg(desktop)]
    let app_builder = app_builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tracing::info!("segunda instância aberta; focando a janela existente");
        shell::show_main_window(app);
    }));

    // Lembra tamanho e posição da janela entre execuções.
    #[cfg(desktop)]
    let app_builder = app_builder.plugin(tauri_plugin_window_state::Builder::default().build());

    app_builder
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            // O material real é aplicado quando a janela nasce (`shell::show_main_window`).
            app.manage(window::ChromeState::new(window::FALLBACK_CHROME));
            app.manage(shell::PendingNav::default());
            app.manage(updates::UpdateState::new(app.handle().clone()));
            start_core(app)
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .on_window_event(shell::on_window_event)
        .build(tauri::generate_context!())
        .expect("falha ao iniciar o VynPad")
        .run(|app, event| {
            // Com "fechar vai para a bandeja", fechar a última janela não encerra o app. Só o
            // "Sair" da bandeja (que traz um código de saída) encerra.
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = &event
                && let Some(state) = app.try_state::<AppState>()
                && state.core.settings().get().close_to_tray
            {
                api.prevent_exit();
            }
            if let RunEvent::Exit = event
                && let Some(state) = app.try_state::<AppState>()
            {
                // Avisa os clients (`bye`), tira o anúncio da rede e fecha o banco.
                let core = Arc::clone(&state.core);
                let _ =
                    tauri::async_runtime::block_on(tokio::time::timeout(EXIT_GRACE, async move {
                        core.stop().await;
                    }));
            }
        });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{Value, json};
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{
        INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
    };
    use tauri::webview::InvokeRequest;
    use vynpad_driver::MockDriver;
    use vynpad_storage::Ports;

    use super::*;

    /// Um app de teste com o `Core` de verdade (banco em pasta temporária, `MockDriver`, portas
    /// livres, sem mDNS), atendendo os commands pela camada IPC do Tauri.
    struct Harness {
        webview: tauri::WebviewWindow<MockRuntime>,
        state: Arc<AppState>,
        _dir: tempfile::TempDir,
        _app: tauri::App<MockRuntime>,
    }

    impl Harness {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let mut config = CoreConfig::new(dir.path());
            config.driver = Some(Arc::new(MockDriver::default()));
            config.bind_ip = "127.0.0.1".parse().unwrap();
            config.ports_override = Some(Ports {
                control: 0,
                input: 0,
                discovery: 0,
            });
            config.mdns = false;
            let core = tauri::async_runtime::block_on(vynpad_server::Core::start(config))
                .expect("core de teste");
            let state = Arc::new(AppState::with_core(core));

            let bridge = specta_builder::<MockRuntime>();
            let app = mock_builder()
                .invoke_handler(bridge.invoke_handler())
                .setup({
                    let bridge = specta_builder::<MockRuntime>();
                    move |app| {
                        bridge.mount_events(app);
                        Ok(())
                    }
                })
                .build(mock_context(noop_assets()))
                .expect("app de teste");
            // O `State` do Tauri é por valor; os testes mantêm o próprio `Arc` para inspecionar.
            app.manage(AppState {
                core: Arc::clone(&state.core),
                live: Arc::clone(&state.live),
                #[cfg(windows)]
                xinput: None,
            });
            let webview = tauri::WebviewWindowBuilder::new(
                &app,
                crate::shell::MAIN_WINDOW,
                tauri::WebviewUrl::default(),
            )
            .build()
            .expect("janela de teste");
            Self {
                webview,
                state,
                _dir: dir,
                _app: app,
            }
        }

        /// Chama um command como o frontend faria.
        fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
            get_ipc_response(
                &self.webview,
                InvokeRequest {
                    cmd: cmd.into(),
                    callback: CallbackFn(0),
                    error: CallbackFn(1),
                    url: "http://tauri.localhost".parse().unwrap(),
                    body: InvokeBody::Json(args),
                    headers: tauri::http::HeaderMap::default(),
                    invoke_key: INVOKE_KEY.to_string(),
                },
            )
            .map(|body| body.deserialize::<Value>().expect("resposta JSON"))
        }

        fn ok(&self, cmd: &str, args: Value) -> Value {
            self.invoke(cmd, args)
                .unwrap_or_else(|e| panic!("{cmd} falhou: {e}"))
        }

        fn err_code(&self, cmd: &str, args: Value) -> String {
            let err = self
                .invoke(cmd, args)
                .expect_err(&format!("{cmd} deveria falhar"));
            err["code"].as_str().unwrap_or_default().to_owned()
        }
    }

    #[test]
    fn app_info_devolve_as_versoes() {
        let h = Harness::new();
        let info = h.ok("app_info", json!({}));
        assert_eq!(info["app_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(info["core_version"], vynpad_core::VERSION);
    }

    #[test]
    fn driver_status_e_xinput_slots_respondem_pela_camada_ipc() {
        let h = Harness::new();
        assert_eq!(
            h.ok("driver_status", json!({})),
            json!({ "state": "ready" })
        );
        assert_eq!(h.ok("xinput_slots", json!({})).as_array().unwrap().len(), 4);
    }

    #[test]
    fn log_do_frontend_valido_e_aceito_e_o_grande_demais_e_recusado() {
        let h = Harness::new();
        assert_eq!(
            h.invoke(
                "log_from_frontend",
                json!({ "level": "info", "message": "olá" })
            ),
            Ok(Value::Null)
        );
        let message = "x".repeat(commands::MAX_FRONTEND_LOG_BYTES + 1);
        let err = h
            .invoke(
                "log_from_frontend",
                json!({ "level": "info", "message": message }),
            )
            .unwrap_err();
        assert_eq!(err["code"], "invalid_input");
        assert_eq!(err["message_key"], "errors.invalid_input");
    }

    #[test]
    fn command_desconhecido_e_recusado() {
        let h = Harness::new();
        assert!(h.invoke("nao_existe", json!({})).is_err());
    }

    #[test]
    fn configuracoes_fazem_o_ciclo_completo_e_validam() {
        let h = Harness::new();
        let mut settings = h.ok("get_settings", json!({}));
        assert_eq!(settings["language"], "pt-BR");
        assert_eq!(settings["ports"]["control"], 47800);

        settings["theme"] = json!("light");
        settings["require_approval"] = json!(true);
        let saved = h.ok("update_settings", json!({ "settings": settings }));
        assert_eq!(saved["theme"], "light");
        assert_eq!(h.ok("get_settings", json!({}))["require_approval"], true);

        // Valor inválido: o code diz o que fazer e o detalhe explica.
        settings["accent"] = json!("azul");
        let err = h
            .invoke("update_settings", json!({ "settings": settings }))
            .unwrap_err();
        assert_eq!(err["code"], "invalid_input");
        assert!(err["details"].as_str().unwrap().contains("#RRGGBB"));
        assert_eq!(h.ok("get_settings", json!({}))["accent"], "#0A84FF");
    }

    #[test]
    fn perfis_criam_salvam_duplicam_exportam_e_protegem_o_padrao() {
        let h = Harness::new();
        let list = h.ok("list_profiles", json!({}));
        let builtin = &list.as_array().unwrap()[0];
        assert_eq!(builtin["is_builtin"], true);
        let builtin_id = builtin["id"].as_str().unwrap().to_owned();

        // O padrão é protegido, mas pode ser duplicado.
        let full = h.ok("get_profile", json!({ "id": builtin_id }));
        let profile = full["profile"].clone();
        assert_eq!(
            h.err_code(
                "save_profile",
                json!({ "id": builtin_id, "input": { "name": "x", "description": null, "icon": null, "profile": profile } })
            ),
            "builtin_profile"
        );
        assert_eq!(
            h.err_code(
                "delete_profile",
                json!({ "id": builtin_id, "confirm": true })
            ),
            "builtin_profile"
        );

        let created = h.ok(
            "save_profile",
            json!({ "id": null, "input": { "name": "Meu", "description": "d", "icon": null, "profile": profile } }),
        );
        let id = created["summary"]["id"].as_str().unwrap().to_owned();
        assert_eq!(created["summary"]["name"], "Meu");

        let copy = h.ok("duplicate_profile", json!({ "id": id, "name": null }));
        assert_eq!(copy["summary"]["name"], "Meu (cópia)");

        let text = h.ok("export_profile", json!({ "id": id }));
        let imported = h.ok("import_profile", json!({ "text": text }));
        assert_ne!(imported["summary"]["id"], created["summary"]["id"]);
        assert_eq!(
            h.err_code("import_profile", json!({ "text": "lixo" })),
            "import_invalid"
        );

        h.ok("delete_profile", json!({ "id": id, "confirm": false }));
        assert_eq!(h.err_code("get_profile", json!({ "id": id })), "not_found");
        assert_eq!(
            h.ok("list_profiles", json!({})).as_array().unwrap().len(),
            3
        );
        h.ok("restore_default_profile", json!({}));
    }

    #[test]
    fn perfil_em_uso_pede_confirmacao_com_os_numeros() {
        let h = Harness::new();
        let full = h.ok(
            "get_profile",
            json!({ "id": vynpad_storage::seed::BUILTIN_PROFILE_ID }),
        );
        let created = h.ok(
            "save_profile",
            json!({ "id": null, "input": { "name": "Em uso", "description": null, "icon": null, "profile": full["profile"] } }),
        );
        let id = created["summary"]["id"].as_str().unwrap().to_owned();
        // Um dispositivo pareado com esse perfil como padrão.
        tauri::async_runtime::block_on(async {
            use vynpad_storage::{DeviceDefaults, NewPairing};
            let devices = h.state.core.devices();
            devices
                .upsert_pairing(&NewPairing {
                    id: "d1".into(),
                    name: "Pixel".into(),
                    model: None,
                    os_version: None,
                    token_hash: [1; 32],
                    now: 1,
                })
                .await
                .unwrap();
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
        });
        let err = h
            .invoke("delete_profile", json!({ "id": id, "confirm": false }))
            .unwrap_err();
        assert_eq!(err["code"], "profile_in_use");
        assert_eq!(err["in_use"], json!({ "devices": 1, "slots": 0 }));
        h.ok("delete_profile", json!({ "id": id, "confirm": true }));
    }

    #[test]
    fn dispositivos_e_historico_respondem_e_erros_de_id_sao_not_found() {
        let h = Harness::new();
        assert_eq!(h.ok("list_devices", json!({})), json!([]));
        assert_eq!(
            h.ok(
                "get_connection_log",
                json!({ "query": { "device_id": null, "before_id": null, "limit": null } })
            ),
            json!([])
        );
        for cmd in ["revoke_device", "forget_device"] {
            assert_eq!(h.err_code(cmd, json!({ "id": "nao-existe" })), "not_found");
        }
        assert_eq!(
            h.err_code("rename_device", json!({ "id": "nao-existe", "name": "x" })),
            "not_found"
        );
        assert_eq!(
            h.err_code(
                "set_device_defaults",
                json!({ "id": "x", "defaults": { "preferred_slot": 7, "default_profile_id": null, "color": null } })
            ),
            "invalid_input"
        );
    }

    #[test]
    fn slots_mostram_os_quatro_livres_e_recusam_indice_invalido() {
        let h = Harness::new();
        let slots = h.ok("get_slots", json!({}));
        assert_eq!(slots["free_count"], 4);
        assert_eq!(slots["slots"].as_array().unwrap().len(), 4);
        assert_eq!(slots["slots"][0]["state"], "free");
        assert_eq!(
            h.err_code("disconnect_slot", json!({ "slot": 9 })),
            "invalid_input"
        );
        assert_eq!(
            h.err_code("set_slot_paused", json!({ "slot": 9, "paused": true })),
            "invalid_input"
        );
        assert_eq!(
            h.err_code("swap_slots", json!({ "a": 0, "b": 7 })),
            "invalid_input"
        );
        // Dois slots vazios: nada a trocar.
        assert_eq!(
            h.err_code("swap_slots", json!({ "a": 0, "b": 2 })),
            "not_found"
        );
        assert_eq!(
            h.ok("get_slot_profiles", json!({})),
            json!([null, null, null, null])
        );
        // Slot vazio: nada a desconectar.
        assert_eq!(
            h.err_code("disconnect_slot", json!({ "slot": 0 })),
            "not_found"
        );
    }

    #[test]
    fn info_do_server_e_rede_respondem() {
        let h = Harness::new();
        let info = h.ok("get_server_info", json!({}));
        assert_eq!(info["free_slots"], 4);
        assert!(info["control_port"].as_u64().unwrap() > 0);
        assert_eq!(info["fingerprint_short"].as_str().unwrap().len(), 16);
        assert_eq!(h.ok("get_nearby_devices", json!({})), json!([]));
        assert!(h.ok("list_network_interfaces", json!({})).is_array());
    }

    #[test]
    fn pareamento_devolve_qr_e_pin_ou_avisa_que_nao_ha_rede() {
        let h = Harness::new();
        match h.invoke("start_pairing", json!({})) {
            Ok(offer) => {
                assert!(offer["qr_svg"].as_str().unwrap().contains("<svg"));
                assert_eq!(offer["pin"].as_str().unwrap().len(), 6);
                assert!(offer["uri"].as_str().unwrap().starts_with("vynpad://pair?"));
                h.ok("cancel_pairing", json!({}));
            }
            Err(err) => assert_eq!(err["code"], "no_network"),
        }
        assert_eq!(
            h.ok(
                "answer_approval",
                json!({ "requestId": 12345, "accept": true })
            ),
            json!(false),
            "um pedido inexistente não é aceito"
        );
    }

    #[test]
    fn input_ao_vivo_so_liga_com_assinatura() {
        let h = Harness::new();
        assert!(!h.state.live.is_active(), "sem tela assinando, nada roda");
        h.ok("subscribe_live_input", json!({ "slot": 1 }));
        assert!(h.state.live.is_active());
        h.ok("unsubscribe_live_input", json!({ "slot": 1 }));
        assert!(!h.state.live.is_active());
        // Uma janela recarregada zera o que ficou contado.
        h.ok("subscribe_live_input", json!({ "slot": 2 }));
        h.ok("subscribe_live_input", json!({ "slot": 2 }));
        h.ok("reset_live_input", json!({}));
        assert!(!h.state.live.is_active());
        assert_eq!(
            h.err_code("subscribe_live_input", json!({ "slot": 8 })),
            "invalid_input"
        );
        assert!(!h.state.live.is_active());
    }

    /// O `bindings.ts` versionado é o que o gerador produz hoje. Se este teste falha, um command,
    /// evento ou tipo mudou em Rust: rode `UPDATE_BINDINGS=1 cargo test -p vynpad-desktop
    /// bindings` e versione o resultado.
    #[test]
    fn bindings_ts_esta_em_dia() {
        let committed =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts");
        let out = tempfile::tempdir().unwrap();
        let generated = out.path().join("bindings.ts");
        specta_builder::<MockRuntime>()
            .export(specta_typescript::Typescript::default(), &generated)
            .expect("exportar bindings");
        let generated = std::fs::read_to_string(&generated)
            .unwrap()
            .replace("\r\n", "\n");

        if std::env::var_os("UPDATE_BINDINGS").is_some() {
            std::fs::write(&committed, &generated).unwrap();
            return;
        }
        let current = std::fs::read_to_string(&committed)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            current == generated,
            "src/lib/bindings.ts está desatualizado; rode `UPDATE_BINDINGS=1 cargo test -p vynpad-desktop bindings_ts`"
        );
    }
}
