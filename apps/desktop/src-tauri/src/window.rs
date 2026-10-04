//! Material da janela (T-502): Mica no Windows 11, Acrylic no Windows 10 e fundo sólido quando a
//! transparência não está disponível.
//!
//! A janela é sem moldura e transparente; o que aparece "atrás" do React é decidido aqui e
//! informado à UI por [`window_chrome`], que ajusta o CSS (`data-material` / `data-corners`).

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

/// Primeiro build do Windows 11, a partir do qual o DWM arredonda os cantos e aceita o Mica.
const WINDOWS_11_BUILD: u32 = 22_000;
/// Primeiro build do Windows 10 com `SetWindowCompositionAttribute` + acrylic (1803).
const ACRYLIC_MIN_BUILD: u32 = 17_134;
/// Tint escuro do acrylic do Windows 10 (RGBA). Pouco opaco: o app já pinta `--bg-window` em cima.
#[cfg(windows)]
const ACRYLIC_TINT: (u8, u8, u8, u8) = (28, 28, 30, 90);

/// O que fica atrás do conteúdo da janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum WindowMaterial {
    Mica,
    Acrylic,
    Solid,
}

/// Quem arredonda os cantos da janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum WindowCorners {
    /// O DWM do Windows 11.
    Native,
    /// O container raiz da UI (`border-radius`), sobre uma área transparente.
    Css,
    /// Sem arredondar: o acrylic do Windows 10 borra o retângulo inteiro, então um raio em CSS
    /// deixaria cantos borrados para fora do desenho.
    Square,
}

/// O que a UI precisa saber para pintar a janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WindowChrome {
    pub material: WindowMaterial,
    pub corners: WindowCorners,
}

/// Escolhe o material a partir do build do Windows (`None` fora do Windows ou se não deu para
/// ler) e de a transparência estar ligada nas configurações do sistema.
pub fn choose_material(build: Option<u32>, transparency_enabled: bool) -> WindowMaterial {
    match build {
        _ if !transparency_enabled => WindowMaterial::Solid,
        Some(b) if b >= WINDOWS_11_BUILD => WindowMaterial::Mica,
        Some(b) if b >= ACRYLIC_MIN_BUILD => WindowMaterial::Acrylic,
        _ => WindowMaterial::Solid,
    }
}

/// Cantos conforme o material realmente aplicado e a versão do sistema.
pub fn choose_corners(build: Option<u32>, material: WindowMaterial) -> WindowCorners {
    match (build, material) {
        (Some(b), _) if b >= WINDOWS_11_BUILD => WindowCorners::Native,
        (_, WindowMaterial::Acrylic) => WindowCorners::Square,
        _ => WindowCorners::Css,
    }
}

/// Aplica o material e devolve o que de fato ficou (cai para sólido se o sistema recusar).
/// `transparency` é a preferência do app (Configurações → Aparência); o efeito só aparece se o
/// Windows também estiver com os efeitos de transparência ligados.
#[cfg(windows)]
pub fn apply<R: Runtime>(window: &tauri::WebviewWindow<R>, transparency: bool) -> WindowChrome {
    let build = windows_build();
    // Tira o efeito anterior antes de pôr (ou não) o novo: trocar a preferência vale na hora.
    let _ = window_vibrancy::clear_mica(window);
    let _ = window_vibrancy::clear_acrylic(window);
    let wanted = choose_material(build, transparency && transparency_enabled());
    let applied = match wanted {
        WindowMaterial::Mica => window_vibrancy::apply_mica(window, Some(true)),
        WindowMaterial::Acrylic => window_vibrancy::apply_acrylic(window, Some(ACRYLIC_TINT)),
        WindowMaterial::Solid => Ok(()),
    };
    let material = match applied {
        Ok(()) => wanted,
        Err(error) => {
            tracing::warn!(%error, ?wanted, "material indisponível; usando fundo sólido");
            WindowMaterial::Solid
        }
    };
    let chrome = WindowChrome {
        material,
        corners: choose_corners(build, material),
    };
    tracing::info!(?build, ?chrome, "material da janela");
    chrome
}

/// Fora do Windows não há Mica/Acrylic: fundo sólido e cantos pelo CSS (Linux, T-803).
#[cfg(not(windows))]
pub fn apply<R: Runtime>(_window: &tauri::WebviewWindow<R>, _transparency: bool) -> WindowChrome {
    WindowChrome {
        material: WindowMaterial::Solid,
        corners: WindowCorners::Css,
    }
}

/// Número do build do Windows (`CurrentBuildNumber`), ex.: 22631.
#[cfg(windows)]
fn windows_build() -> Option<u32> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", KEY_READ)
        .ok()?
        .get_value::<String, _>("CurrentBuildNumber")
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// "Efeitos de transparência" de Configurações → Personalização → Cores (ligado por padrão).
#[cfg(windows)]
fn transparency_enabled() -> bool {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            KEY_READ,
        )
        .and_then(|key| key.get_value::<u32, _>("EnableTransparency"))
        .map_or(true, |value| value != 0)
}

/// O material aplicado na janela principal, guardado como estado do app (muda com a preferência
/// de transparência).
pub struct ChromeState(pub Mutex<WindowChrome>);

impl ChromeState {
    pub fn new(chrome: WindowChrome) -> Self {
        Self(Mutex::new(chrome))
    }

    fn get(&self) -> WindowChrome {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, chrome: WindowChrome) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = chrome;
    }
}

/// O material da janela mudou (`window:chrome-changed`): a UI refaz o fundo e os cantos.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct WindowChromeChanged {
    pub chrome: WindowChrome,
}

/// Aplica de novo o material com a preferência `transparency`, guarda e avisa a UI.
pub fn reapply<R: Runtime>(app: &AppHandle<R>, transparency: bool) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let chrome = apply(&window, transparency);
    if let Some(state) = app.try_state::<ChromeState>() {
        state.set(chrome);
    }
    if let Err(error) = (WindowChromeChanged { chrome }).emit(app) {
        tracing::warn!(%error, "não consegui avisar a UI do novo material da janela");
    }
}

/// Como a janela está pintada, para a UI escolher fundo e cantos.
// O Tauri injeta o `State` por valor.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn window_chrome(state: tauri::State<'_, ChromeState>) -> WindowChrome {
    state.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_11_usa_mica_com_cantos_nativos() {
        let material = choose_material(Some(22_631), true);
        assert_eq!(material, WindowMaterial::Mica);
        assert_eq!(
            choose_corners(Some(22_631), material),
            WindowCorners::Native
        );
    }

    #[test]
    fn windows_10_usa_acrylic_sem_arredondar() {
        let material = choose_material(Some(19_045), true);
        assert_eq!(material, WindowMaterial::Acrylic);
        assert_eq!(
            choose_corners(Some(19_045), material),
            WindowCorners::Square
        );
    }

    #[test]
    fn transparencia_desligada_cai_para_solido() {
        assert_eq!(choose_material(Some(22_631), false), WindowMaterial::Solid);
        assert_eq!(choose_material(Some(19_045), false), WindowMaterial::Solid);
    }

    #[test]
    fn solido_no_windows_10_arredonda_pelo_css() {
        assert_eq!(
            choose_corners(Some(19_045), WindowMaterial::Solid),
            WindowCorners::Css
        );
    }

    #[test]
    fn solido_no_windows_11_mantem_cantos_nativos() {
        assert_eq!(
            choose_corners(Some(22_631), WindowMaterial::Solid),
            WindowCorners::Native
        );
    }

    #[test]
    fn sistema_antigo_ou_desconhecido_fica_solido() {
        assert_eq!(choose_material(Some(17_000), true), WindowMaterial::Solid);
        assert_eq!(choose_material(None, true), WindowMaterial::Solid);
        assert_eq!(
            choose_corners(None, WindowMaterial::Solid),
            WindowCorners::Css
        );
    }

    #[test]
    fn serializa_em_minusculas_para_a_ui() {
        let json = serde_json::to_value(WindowChrome {
            material: WindowMaterial::Mica,
            corners: WindowCorners::Native,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "material": "mica", "corners": "native" })
        );
    }
}
