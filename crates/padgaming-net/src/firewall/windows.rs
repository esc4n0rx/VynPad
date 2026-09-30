//! Leitura das regras (COM `INetFwPolicy2`) e elevação (UAC) no Windows.
//!
//! Junto com `input/platform.rs`, é um dos poucos lugares com `unsafe` desta crate: chamadas
//! COM e `ShellExecuteExW`.
#![allow(unsafe_code)]

use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::NetworkManagement::WindowsFirewall::{
    INetFwPolicy2, INetFwRule, NET_FW_ACTION_ALLOW, NET_FW_PROFILE_TYPE2, NET_FW_RULE_DIR_IN,
    NetFwPolicy2,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Ole::IEnumVARIANT;
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::{
    SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{HRESULT, Interface, PCWSTR, w};

use super::{
    ActiveProfiles, FirewallError, FirewallPorts, FirewallStatus, PROFILE_DOMAIN, PROFILE_PRIVATE,
    PROFILE_PUBLIC, RuleInfo, evaluate,
};

/// `HRESULT_FROM_WIN32(ERROR_CANCELLED)`: o usuário negou o UAC.
const CANCELLED: HRESULT = HRESULT(0x8007_04C7_u32.cast_signed());
/// Espera máxima pelo comando elevado.
const ELEVATED_TIMEOUT_MS: u32 = 60_000;

fn system(error: &windows::core::Error) -> FirewallError {
    FirewallError::System(error.to_string())
}

/// Desfaz o `CoInitializeEx` da thread ao sair.
struct ComGuard(bool);

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: par do `CoInitializeEx` bem-sucedido feito nesta mesma thread.
            unsafe { CoUninitialize() };
        }
    }
}

/// Lê o estado do firewall. O COM roda numa thread própria: não mexe no apartamento COM (nem
/// no modo de inicialização) da thread de quem chamou.
pub(super) fn check(exe: &Path, ports: FirewallPorts) -> Result<FirewallStatus, FirewallError> {
    let exe = exe.to_owned();
    std::thread::Builder::new()
        .name("firewall-check".into())
        .spawn(move || read(&exe, ports))
        .map_err(|e| FirewallError::System(e.to_string()))?
        .join()
        .map_err(|_| FirewallError::System("a leitura do firewall falhou".into()))?
}

fn read(exe: &Path, ports: FirewallPorts) -> Result<FirewallStatus, FirewallError> {
    // SAFETY: cada thread inicializa o próprio COM e o `ComGuard` o desfaz na saída.
    let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let _guard = ComGuard(init.is_ok());

    // SAFETY: `NetFwPolicy2` é o CLSID do objeto em processo; o tipo pedido é o da sua interface.
    let policy: INetFwPolicy2 =
        unsafe { CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| system(&e))?;
    // SAFETY: chamadas COM sem ponteiros próprios; o `policy` é válido.
    let mask = unsafe { policy.CurrentProfileTypes() }.map_err(|e| system(&e))?;
    let profiles = ActiveProfiles::from_mask(mask);

    let firewall_on = [PROFILE_DOMAIN, PROFILE_PRIVATE, PROFILE_PUBLIC]
        .into_iter()
        .filter(|bit| mask & bit != 0)
        // Se a consulta falhar, assume ligado: é o lado seguro (avisa em vez de calar).
        .any(|bit| {
            // SAFETY: como acima.
            unsafe { policy.get_FirewallEnabled(NET_FW_PROFILE_TYPE2(bit)) }
                .map_or(true, windows::Win32::Foundation::VARIANT_BOOL::as_bool)
        });

    let rules = read_rules(&policy)?;
    Ok(evaluate(&rules, exe, ports, profiles, firewall_on))
}

fn read_rules(policy: &INetFwPolicy2) -> Result<Vec<RuleInfo>, FirewallError> {
    // SAFETY: todas as chamadas abaixo são COM sobre objetos válidos, com buffers locais.
    unsafe {
        let collection = policy.Rules().map_err(|e| system(&e))?;
        let enumerator: IEnumVARIANT = collection
            ._NewEnum()
            .and_then(|unknown| unknown.cast())
            .map_err(|e| system(&e))?;
        let mut rules = Vec::new();
        loop {
            let mut item = [VARIANT::default()];
            let mut fetched = 0u32;
            let hr = enumerator.Next(&mut item, &raw mut fetched);
            if hr.0 != 0 || fetched == 0 {
                break; // S_OK com 1 item; S_FALSE ou erro encerra.
            }
            // Uma regra que não abre (permissão, formato) é pulada, não derruba a leitura.
            let Ok(unknown) = windows::core::IUnknown::try_from(&item[0]) else {
                continue;
            };
            let Ok(rule) = unknown.cast::<INetFwRule>() else {
                continue;
            };
            if let Some(info) = describe(&rule) {
                rules.push(info);
            }
        }
        Ok(rules)
    }
}

/// # Safety
/// `rule` deve ser uma interface COM válida.
unsafe fn describe(rule: &INetFwRule) -> Option<RuleInfo> {
    // SAFETY: garantido pelo chamador.
    unsafe {
        let application = rule
            .ApplicationName()
            .ok()
            .map(|b| b.to_string())
            .filter(|s| !s.is_empty());
        Some(RuleInfo {
            name: rule.Name().ok()?.to_string(),
            enabled: rule.Enabled().ok()?.as_bool(),
            inbound: rule.Direction().ok()? == NET_FW_RULE_DIR_IN,
            allow: rule.Action().ok()? == NET_FW_ACTION_ALLOW,
            application,
            protocol: rule.Protocol().ok()?,
            local_ports: rule
                .LocalPorts()
                .ok()
                .map(|b| b.to_string())
                .unwrap_or_default(),
            profiles: rule.Profiles().ok()?,
        })
    }
}

/// Executa `command` com `cmd /c` elevado (UAC) e espera. Um único pedido de permissão cobre
/// todos os `netsh` encadeados.
pub(super) fn run_elevated(command: &str) -> Result<(), FirewallError> {
    // `/s` faz o `cmd` tirar só as aspas de fora, preservando as de dentro.
    let parameters: Vec<u16> = format!("/d /s /c \"{command}\"")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: u32::try_from(size_of::<SHELLEXECUTEINFOW>()).unwrap_or(0),
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: w!("cmd.exe"),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: `info` é válido e `parameters` (terminado em zero) vive até depois da chamada.
    if let Err(error) = unsafe { ShellExecuteExW(&raw mut info) } {
        return Err(if error.code() == CANCELLED {
            FirewallError::Cancelled
        } else {
            system(&error)
        });
    }
    let process = info.hProcess;
    if process.is_invalid() {
        return Err(FirewallError::System(
            "o Windows não devolveu o processo elevado".into(),
        ));
    }
    // SAFETY: `process` é um handle de processo válido que este código possui e fecha abaixo.
    unsafe {
        let waited = WaitForSingleObject(process, ELEVATED_TIMEOUT_MS);
        let mut code = 0u32;
        let got = GetExitCodeProcess(process, &raw mut code);
        let _ = CloseHandle(process);
        if waited != WAIT_OBJECT_0 {
            Err(FirewallError::System(
                "o comando elevado não terminou a tempo".into(),
            ))
        } else if got.is_err() {
            Err(FirewallError::System(
                "não consegui ler o resultado do comando elevado".into(),
            ))
        } else if code != 0 {
            Err(FirewallError::CommandFailed(code))
        } else {
            Ok(())
        }
    }
}
