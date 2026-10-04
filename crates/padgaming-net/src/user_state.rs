//! O estado do usuário no Windows: há um programa em tela cheia (jogo, apresentação)?
//!
//! Serve para o app não mostrar avisos por cima de um jogo (T-608).

/// `QUNS_BUSY`, `QUNS_RUNNING_D3D_FULL_SCREEN` e `QUNS_PRESENTATION_MODE` do
/// `SHQueryUserNotificationState`: um programa em tela cheia (ou uma apresentação) ocupa o
/// usuário, e o sistema mesmo segura as notificações.
pub const fn fullscreen_from_state(raw: i32) -> bool {
    matches!(raw, 2..=4)
}

/// Um programa em tela cheia está em uso agora. Fora do Windows (ou se o sistema não responder),
/// `false`.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn fullscreen_busy() -> bool {
    use windows::Win32::UI::Shell::SHQueryUserNotificationState;
    // SAFETY: chamada sem argumentos que só lê o estado do shell.
    unsafe { SHQueryUserNotificationState() }.is_ok_and(|state| fullscreen_from_state(state.0))
}

/// Fora do Windows não há como saber.
#[cfg(not(windows))]
pub const fn fullscreen_busy() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_estados_de_tela_cheia_do_windows_sao_reconhecidos() {
        // QUNS_NOT_PRESENT = 1, BUSY = 2, D3D_FULL_SCREEN = 3, PRESENTATION_MODE = 4,
        // ACCEPTS_NOTIFICATIONS = 5, QUIET_TIME = 6, APP = 7.
        for raw in [2, 3, 4] {
            assert!(fullscreen_from_state(raw), "{raw}");
        }
        for raw in [0, 1, 5, 6, 7] {
            assert!(!fullscreen_from_state(raw), "{raw}");
        }
    }

    #[test]
    fn a_consulta_ao_sistema_nao_quebra() {
        // O valor depende de a máquina ter um jogo aberto agora; só garante que responde.
        let _ = fullscreen_busy();
    }
}
