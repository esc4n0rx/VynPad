//! Commands de sistema das Configurações e do onboarding: firewall, teste de conexão, pasta de
//! logs, privacidade (histórico e identidade) e reinício do app.
//!
//! Firewall e UAC bloqueiam a thread por um tempo (COM, elevação), então rodam em
//! `spawn_blocking` e não seguram a webview.

use std::time::{Duration, SystemTime};
use vynpad_net::firewall::{
    self, FirewallError, FirewallPorts, FirewallStatus, NETWORK_SETTINGS_URI, Transport,
};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use vynpad_server::{DeviceSummary, Diagnostics, LogFile};

use crate::error::{AppError, ErrorCode};
use crate::state::AppState;

type Result<T> = std::result::Result<T, AppError>;

/// Uma porta de que o server precisa e se o firewall a libera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct FirewallPortDto {
    /// Para que serve (texto técnico, em português).
    pub purpose: String,
    pub port: u16,
    /// `tcp` ou `udp`.
    pub protocol: String,
    pub allowed: bool,
}

/// O estado do firewall do Windows para o `VynPad`.
// São quatro fatos independentes que a UI mostra; agrupá-los só esconderia o que cada um diz.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct FirewallStatusDto {
    /// O sistema tem o firewall do Windows (fora dele, nada a fazer).
    pub applicable: bool,
    pub firewall_on: bool,
    /// Nada bloqueia o app: o firewall está desligado ou todas as portas estão liberadas.
    pub ok: bool,
    /// Alguma rede ativa é pública: as regras do app não valem nela.
    pub public_network: bool,
    pub ports: Vec<FirewallPortDto>,
}

impl From<&FirewallStatus> for FirewallStatusDto {
    fn from(status: &FirewallStatus) -> Self {
        Self {
            applicable: status.applicable,
            firewall_on: status.firewall_on,
            ok: status.ok(),
            public_network: status.public_network_warning(),
            ports: status
                .ports
                .iter()
                .map(|p| FirewallPortDto {
                    purpose: p.requirement.purpose.to_owned(),
                    port: p.requirement.port,
                    protocol: match p.requirement.transport {
                        Transport::Tcp => "tcp",
                        Transport::Udp => "udp",
                    }
                    .to_owned(),
                    allowed: p.allowed,
                })
                .collect(),
        }
    }
}

/// O resultado de uma sonda do "Testar conexão".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProbeDto {
    pub what: String,
    pub ok: bool,
}

impl From<FirewallError> for AppError {
    fn from(error: FirewallError) -> Self {
        match error {
            FirewallError::Cancelled => Self::new(ErrorCode::Cancelled, None),
            FirewallError::Unsupported => Self::new(ErrorCode::Unsupported, None),
            other => Self::internal(&other),
        }
    }
}

fn firewall_ports(state: &AppState) -> FirewallPorts {
    let ports = state.core.ports();
    FirewallPorts {
        control_tcp: ports.control,
        input_udp: ports.input,
        discovery_udp: ports.discovery,
    }
}

/// Lê o estado do firewall (COM no Windows; em outros sistemas, "não se aplica").
#[tauri::command]
#[specta::specta]
pub async fn firewall_status(state: State<'_, AppState>) -> Result<FirewallStatusDto> {
    let ports = firewall_ports(&state);
    let status = tauri::async_runtime::spawn_blocking(move || {
        let exe = firewall::current_exe()?;
        firewall::check(&exe, ports).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::internal(&e))??;
    Ok(FirewallStatusDto::from(&status))
}

/// Cria as regras do app no firewall, pedindo permissão de administrador (UAC) uma vez, e
/// devolve o estado depois. Recusar o UAC devolve `cancelled`.
#[tauri::command]
#[specta::specta]
pub async fn fix_firewall(state: State<'_, AppState>) -> Result<FirewallStatusDto> {
    let ports = firewall_ports(&state);
    let status = tauri::async_runtime::spawn_blocking(move || {
        let exe = firewall::current_exe()?;
        firewall::fix(&exe, ports).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::internal(&e))??;
    Ok(FirewallStatusDto::from(&status))
}

/// "Testar conexão": confere, pela interface de LAN, se o canal de controle e o respondedor de
/// descoberta estão de pé. Não prova que o firewall deixa um celular entrar (isso só um aparelho
/// externo prova).
#[tauri::command]
#[specta::specta]
pub async fn test_connection(state: State<'_, AppState>) -> Result<Vec<ProbeDto>> {
    let ports = firewall_ports(&state);
    let Some(lan) = state.core.local_addresses().into_iter().next() else {
        return Err(AppError::new(ErrorCode::NoNetwork, None));
    };
    let probes = tauri::async_runtime::spawn_blocking(move || firewall::self_test(lan.ip, ports))
        .await
        .map_err(|e| AppError::internal(&e))?;
    Ok(probes
        .into_iter()
        .map(|p| ProbeDto {
            what: p.what.to_owned(),
            ok: p.ok,
        })
        .collect())
}

/// Abre "Rede e Internet" do Windows (para trocar a rede pública por privada).
#[tauri::command]
#[specta::specta]
pub fn open_network_settings() -> Result<()> {
    tauri_plugin_opener::open_url(NETWORK_SETTINGS_URI, None::<&str>)
        .map_err(|e| AppError::internal(&e))
}

/// Abre a pasta de logs no Explorer.
#[tauri::command]
#[specta::specta]
pub fn open_logs_dir() -> Result<()> {
    let dir = crate::logging::log_dir().ok_or_else(|| AppError::not_found("a pasta de logs"))?;
    std::fs::create_dir_all(&dir)?;
    tauri_plugin_opener::open_path(dir, None::<&str>).map_err(|e| AppError::internal(&e))
}

/// Onde ficam os logs (para mostrar na tela Sobre).
#[tauri::command]
#[specta::specta]
pub fn logs_dir() -> Option<String> {
    crate::logging::log_dir().map(|dir| dir.to_string_lossy().into_owned())
}

/// Apaga o histórico de conexões. Devolve quantas linhas saíram.
#[tauri::command]
#[specta::specta]
pub async fn clear_history(state: State<'_, AppState>) -> Result<u32> {
    let removed = state.core.clear_history().await?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}

/// Gera uma identidade nova para o server e esquece todos os dispositivos pareados. Vale por
/// inteiro depois de reiniciar o app (`restart_app`).
#[tauri::command]
#[specta::specta]
pub async fn regenerate_identity(state: State<'_, AppState>) -> Result<()> {
    Ok(state.core.regenerate_identity().await?)
}

/// Reinicia o app (a identidade nova e as portas só valem depois disso).
#[tauri::command]
#[specta::specta]
pub async fn restart_app(state: State<'_, AppState>) -> Result<()> {
    tracing::info!("reiniciando o app a pedido do usuário");
    // O `restart` encerra o processo sem passar pelo `RunEvent::Exit`: avisa os clients antes.
    state.core.stop().await;
    tauri::process::restart(&tauri::Env::default())
}

// ------------------------------------------------------------------ diagnóstico

/// Só os logs dos últimos dias entram no pacote.
const LOG_DAYS: u64 = 3;
/// De cada log, no máximo este tanto do final (os mais recentes).
const LOG_TAIL_BYTES: usize = 2 * 1024 * 1024;

/// Lê os logs recentes (`vynpad.*.log` dos últimos [`LOG_DAYS`] dias), só o final de cada um.
fn recent_logs() -> Vec<LogFile> {
    let Some(dir) = crate::logging::log_dir() else {
        return Vec::new();
    };
    let Some(limit) = SystemTime::now().checked_sub(Duration::from_secs(LOG_DAYS * 24 * 3600))
    else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(SystemTime, LogFile)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_log = std::path::Path::new(&name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("log"));
            if !(name.starts_with("vynpad") && is_log) {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            if modified < limit {
                return None;
            }
            let bytes = std::fs::read(entry.path()).ok()?;
            let tail = &bytes[bytes.len().saturating_sub(LOG_TAIL_BYTES)..];
            Some((
                modified,
                LogFile {
                    name,
                    text: String::from_utf8_lossy(tail).into_owned(),
                },
            ))
        })
        .collect();
    files.sort_by_key(|(at, _)| *at);
    files.into_iter().map(|(_, file)| file).collect()
}

fn info_text(state: &AppState, firewall: Option<&FirewallStatus>) -> String {
    let ports = state.core.ports();
    let settings = state.core.settings().get();
    let slots = state.core.slots().snapshot();
    let used = slots.slots.iter().filter(|s| s.device.is_some()).count();
    let mut lines = vec![
        format!(
            "VynPad {} (núcleo {})",
            env!("CARGO_PKG_VERSION"),
            vynpad_core::VERSION
        ),
        format!(
            "Sistema: {} {}{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            crate::window::windows_build()
                .map_or_else(String::new, |build| format!(" (build {build})"))
        ),
        format!("Idioma: {:?}", settings.language),
        format!("Driver: {:?}", state.core.driver_status()),
        format!(
            "Portas: controle {} / input {} / descoberta {}",
            ports.control, ports.input, ports.discovery
        ),
        format!(
            "Impressão digital do server: {}",
            vynpad_protocol::discovery::short_fingerprint(&state.core.fingerprint())
        ),
        format!("Slots em uso: {used} de {}", slots.slots.len()),
    ];
    lines.push(match firewall {
        Some(status) => format!(
            "Firewall: aplicável={}, ligado={}, ok={}, rede pública={}",
            status.applicable,
            status.firewall_on,
            status.ok(),
            status.public_network_warning()
        ),
        None => "Firewall: não consegui ler".to_owned(),
    });
    lines.join("\n")
}

/// Exporta o pacote de diagnóstico (`.zip`) para a pasta Downloads (ou a Área de Trabalho),
/// mostra o arquivo no Explorer e devolve o caminho. Sem tokens nem chaves; com `mask_ips`, os
/// IPs também saem mascarados.
#[tauri::command]
#[specta::specta]
pub async fn export_diagnostics(state: State<'_, AppState>, mask_ips: bool) -> Result<String> {
    let ports = firewall_ports(&state);
    let firewall = tauri::async_runtime::spawn_blocking(move || {
        let exe = firewall::current_exe().ok()?;
        firewall::check(&exe, ports).ok()
    })
    .await
    .ok()
    .flatten();

    let devices = state
        .core
        .devices()
        .list()
        .await
        .map_err(vynpad_server::CoreError::from)?
        .into_iter()
        .map(|d| DeviceSummary {
            id: d.id,
            name: d.name,
            model: d.model,
            os_version: d.os_version,
            paired_at: d.paired_at,
            last_seen_at: d.last_seen_at,
            revoked: d.revoked,
            preferred_slot: d.preferred_slot,
        })
        .collect();
    let metrics = crate::dto::MetricsDto::from(&state.core.metrics());
    let diagnostics = Diagnostics {
        info: info_text(&state, firewall.as_ref()),
        settings_json: serde_json::to_string_pretty(&*state.core.settings().get())
            .map_err(|e| AppError::internal(&e))?,
        interfaces: state
            .core
            .local_addresses()
            .into_iter()
            .map(|i| (i.name, i.ip.to_string()))
            .collect(),
        metrics_json: serde_json::to_string_pretty(&metrics).map_err(|e| AppError::internal(&e))?,
        devices,
        logs: recent_logs(),
        mask_ips,
    };
    let bytes = vynpad_server::build_zip(&diagnostics).map_err(|e| AppError::internal(&e))?;

    let dir = dirs::download_dir()
        .or_else(dirs::desktop_dir)
        .or_else(dirs::home_dir)
        .ok_or_else(|| AppError::not_found("a pasta de Downloads"))?;
    let path = dir.join(format!(
        "vynpad-diagnostico-{}.zip",
        vynpad_server::utc_stamp(vynpad_storage::seed::now_ms())
    ));
    std::fs::write(&path, bytes)?;
    tracing::info!(path = %path.display(), "pacote de diagnóstico exportado");
    // Mostrar no Explorer é conveniência: se falhar, o arquivo já está salvo.
    if let Err(error) = tauri_plugin_opener::reveal_item_in_dir(&path) {
        tracing::warn!(%error, "não consegui mostrar o diagnóstico no Explorer");
    }
    Ok(path.to_string_lossy().into_owned())
}
