//! Detecção e instalação do `ViGEmBus` (Windows).
//!
//! - Estado do serviço `ViGEmBus` pelo gerenciador de serviços (SCM).
//! - Versão instalada, lida do registro (chave de desinstalação do instalador).
//! - Sem o driver, a UI avisa e leva o usuário à página oficial de download.

use std::io;
use std::os::windows::process::CommandExt as _;
use std::process::Command;

use windows_service::service::{ServiceAccess, ServiceState as ScmState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

use crate::DriverStatus;

const SERVICE_NAME: &str = "ViGEmBus";
const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_KEY_WOW64: &str =
    r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
/// Nome de exibição do `ViGEmBus` na lista de programas instalados.
const DISPLAY_NAME: &str = "ViGEm Bus Driver";
/// Versão mínima exigida, mostrada na UI quando o driver instalado é recusado.
pub const REQUIRED_VERSION: &str = "1.16";

const ERROR_SERVICE_DOES_NOT_EXIST: i32 = 1060;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Estado do serviço `ViGEmBus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Não existe: o driver não está instalado.
    Missing,
    /// Instalado e parado.
    Stopped,
    /// Rodando.
    Running,
    /// Subindo, parando ou pausado.
    Transitioning,
    /// Não foi possível consultar (permissão, por exemplo).
    Unknown,
}

/// Consulta o serviço `ViGEmBus` no SCM.
pub fn service_state() -> ServiceState {
    let Ok(manager) = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
    else {
        return ServiceState::Unknown;
    };
    match manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
        Ok(service) => match service.query_status().map(|s| s.current_state) {
            Ok(ScmState::Running) => ServiceState::Running,
            Ok(ScmState::Stopped) => ServiceState::Stopped,
            Ok(_) => ServiceState::Transitioning,
            Err(_) => ServiceState::Unknown,
        },
        Err(windows_service::Error::Winapi(e))
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST) =>
        {
            ServiceState::Missing
        }
        Err(_) => ServiceState::Unknown,
    }
}

/// Versão do `ViGEmBus` instalada, se o registro a informar.
pub fn installed_version() -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    [UNINSTALL_KEY, UNINSTALL_KEY_WOW64]
        .into_iter()
        .find_map(|path| {
            let root = hklm.open_subkey_with_flags(path, KEY_READ).ok()?;
            root.enum_keys().flatten().find_map(|name| {
                let key = root.open_subkey_with_flags(name, KEY_READ).ok()?;
                let display: String = key.get_value("DisplayName").ok()?;
                (display == DISPLAY_NAME)
                    .then(|| key.get_value::<String, _>("DisplayVersion").ok())
                    .flatten()
            })
        })
}

/// Traduz uma falha ao conectar no barramento para o status mostrado ao usuário.
///
/// Pura (recebe o estado do serviço e a versão), para ser testável sem o driver.
pub fn classify_bus_error(
    error: vigem_client::Error,
    service: ServiceState,
    version: Option<String>,
) -> DriverStatus {
    use vigem_client::Error;
    match error {
        Error::BusNotFound => match service {
            ServiceState::Missing => DriverStatus::NotInstalled,
            ServiceState::Stopped => DriverStatus::ServiceStopped,
            _ => DriverStatus::Error(error.to_string()),
        },
        Error::BusVersionMismatch => DriverStatus::Outdated {
            found: version.unwrap_or_else(|| "desconhecida".to_owned()),
            required: REQUIRED_VERSION.to_owned(),
        },
        other => DriverStatus::Error(other.to_string()),
    }
}

// ---------------------------------------------------------------------- download

/// Abre a página de download do `ViGEmBus` no navegador padrão.
pub fn open_download_page() -> io::Result<()> {
    // O Explorer entrega URLs `https` ao navegador padrão, sem passar por um shell.
    Command::new("explorer.exe")
        .arg(crate::VIGEM_DOWNLOAD_URL)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_missing_driver_and_stopped_service() {
        use vigem_client::Error;
        assert_eq!(
            classify_bus_error(Error::BusNotFound, ServiceState::Missing, None),
            DriverStatus::NotInstalled
        );
        assert_eq!(
            classify_bus_error(Error::BusNotFound, ServiceState::Stopped, None),
            DriverStatus::ServiceStopped
        );
        assert!(matches!(
            classify_bus_error(Error::BusNotFound, ServiceState::Running, None),
            DriverStatus::Error(_)
        ));
    }

    #[test]
    fn version_mismatch_reports_the_installed_version() {
        use vigem_client::Error;
        assert_eq!(
            classify_bus_error(
                Error::BusVersionMismatch,
                ServiceState::Running,
                Some("1.9.0".into())
            ),
            DriverStatus::Outdated {
                found: "1.9.0".into(),
                required: REQUIRED_VERSION.into()
            }
        );
        assert!(matches!(
            classify_bus_error(Error::BusVersionMismatch, ServiceState::Running, None),
            DriverStatus::Outdated { found, .. } if found == "desconhecida"
        ));
    }

    #[test]
    fn service_query_does_not_panic_whatever_the_machine_has() {
        let _ = service_state();
        let _ = installed_version();
    }
}
