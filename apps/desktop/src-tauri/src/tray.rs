//! Ícone da bandeja (T-607): variantes (ocioso, com controles, erro de driver), tooltip, menu
//! com os 4 slots e as ações rápidas, tudo traduzido.
//!
//! O que decide texto, ícone e estado do menu é o [`TrayModel`], puro e testado. O resto é cola
//! com o Tauri: [`install`] cria o ícone e [`spawn_updater`] o mantém em dia a partir dos eventos
//! do `Core`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::broadcast::error::RecvError;
use vynpad_core::SlotView;
use vynpad_driver::DriverStatus;
use vynpad_server::{Core, CoreEvent};
use vynpad_storage::Language;

use crate::shell;
use crate::state::AppState;

/// Id do ícone (o Tauri guarda um só).
pub const TRAY_ID: &str = "main";

/// De quanto em quanto tempo o ícone é conferido mesmo sem evento (o driver pode cair sozinho).
const POLL: Duration = Duration::from_secs(2);
/// A latência no menu muda de 5 em 5 ms, para o menu não ser refeito a cada oscilação.
const RTT_STEP_MS: u32 = 5;

const ICON_NORMAL: &[u8] = include_bytes!("../icons/tray/normal.ico");
const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray/active.ico");
const ICON_ERROR: &[u8] = include_bytes!("../icons/tray/error.ico");

/// Qual ícone mostrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    /// Nenhum controle conectado.
    Normal,
    /// Pelo menos um controle conectado.
    Active,
    /// O driver de controles virtuais não está funcionando.
    Error,
}

/// O que o usuário pode pedir pelo menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    Pair,
    PauseAll,
    Settings,
    Quit,
}

impl MenuAction {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Pair => "pair",
            Self::PauseAll => "pause_all",
            Self::Settings => "settings",
            Self::Quit => "quit",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        [
            Self::Open,
            Self::Pair,
            Self::PauseAll,
            Self::Settings,
            Self::Quit,
        ]
        .into_iter()
        .find(|a| a.id() == id)
    }
}

/// A linha de um slot no menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotLine {
    pub slot: u8,
    /// Nome do dispositivo; `None` com o slot livre.
    pub device: Option<String>,
    /// Última latência medida, arredondada.
    pub rtt_ms: Option<u32>,
    pub paused: bool,
    /// A conexão caiu e o slot espera o celular voltar.
    pub reconnecting: bool,
}

/// Tudo de que a bandeja depende.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayModel {
    pub language: Language,
    pub driver_ready: bool,
    pub slots: [SlotLine; 4],
}

/// Os textos do menu e do tooltip num idioma.
struct Text {
    open: &'static str,
    pair: &'static str,
    pause_all: &'static str,
    settings: &'static str,
    quit: &'static str,
    free: &'static str,
    reconnecting: &'static str,
    paused: &'static str,
    idle: &'static str,
    one: &'static str,
    many: &'static str,
    driver_error: &'static str,
}

const PT: Text = Text {
    open: "Abrir VynPad",
    pair: "Parear novo dispositivo",
    pause_all: "Pausar todos os controles",
    settings: "Configurações",
    quit: "Sair",
    free: "livre",
    reconnecting: "reconectando",
    paused: "pausado",
    idle: "nenhum controle conectado",
    one: "1 controle conectado",
    many: "controles conectados",
    driver_error: "driver com problema",
};

const EN: Text = Text {
    open: "Open VynPad",
    pair: "Pair a new device",
    pause_all: "Pause all controllers",
    settings: "Settings",
    quit: "Quit",
    free: "free",
    reconnecting: "reconnecting",
    paused: "paused",
    idle: "no controllers connected",
    one: "1 controller connected",
    many: "controllers connected",
    driver_error: "driver problem",
};

const fn text(language: Language) -> &'static Text {
    match language {
        Language::PtBr => &PT,
        Language::En => &EN,
    }
}

impl TrayModel {
    /// Slots com um celular (conectado, reconectando ou pausado).
    pub fn connected(&self) -> usize {
        self.slots.iter().filter(|s| s.device.is_some()).count()
    }

    /// Há controles e todos estão pausados (o item do menu vira "retomar").
    pub fn all_paused(&self) -> bool {
        let mut occupied = self.slots.iter().filter(|s| s.device.is_some()).peekable();
        occupied.peek().is_some() && occupied.all(|s| s.paused)
    }

    pub fn icon(&self) -> IconKind {
        if !self.driver_ready {
            IconKind::Error
        } else if self.connected() > 0 {
            IconKind::Active
        } else {
            IconKind::Normal
        }
    }

    /// "`VynPad` — 2 controles conectados".
    pub fn tooltip(&self) -> String {
        let t = text(self.language);
        let status = match self.connected() {
            0 => t.idle.to_owned(),
            1 => t.one.to_owned(),
            n => format!("{n} {}", t.many),
        };
        if self.driver_ready {
            format!("VynPad — {status}")
        } else {
            format!("VynPad — {}", t.driver_error)
        }
    }

    /// "P1 · Galaxy da Ana · 21 ms", "P2 · livre".
    pub fn slot_label(&self, line: &SlotLine) -> String {
        let t = text(self.language);
        let head = format!("P{}", line.slot + 1);
        let Some(device) = &line.device else {
            return format!("{head} · {}", t.free);
        };
        let tail = if line.reconnecting {
            t.reconnecting.to_owned()
        } else if line.paused {
            t.paused.to_owned()
        } else {
            line.rtt_ms
                .map_or_else(String::new, |ms| format!("{ms} ms"))
        };
        if tail.is_empty() {
            format!("{head} · {device}")
        } else {
            format!("{head} · {device} · {tail}")
        }
    }
}

/// Arredonda a latência para o degrau do menu.
pub fn round_rtt(ms: f64) -> u32 {
    // Valor pequeno e não negativo: a conversão não perde nada que importe.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ms = ms.max(0.0).round() as u32;
    ((ms + RTT_STEP_MS / 2) / RTT_STEP_MS) * RTT_STEP_MS
}

fn icon_image(kind: IconKind) -> Option<Image<'static>> {
    let bytes = match kind {
        IconKind::Normal => ICON_NORMAL,
        IconKind::Active => ICON_ACTIVE,
        IconKind::Error => ICON_ERROR,
    };
    Image::from_bytes(bytes).ok()
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, model: &TrayModel) -> tauri::Result<Menu<R>> {
    let t = text(model.language);
    let mut menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id(MenuAction::Open.id(), t.open).build(app)?)
        .separator();
    for line in &model.slots {
        let item = MenuItemBuilder::with_id(format!("slot-{}", line.slot), model.slot_label(line))
            .enabled(false)
            .build(app)?;
        menu = menu.item(&item);
    }
    menu.separator()
        .item(&MenuItemBuilder::with_id(MenuAction::Pair.id(), t.pair).build(app)?)
        .item(
            &CheckMenuItemBuilder::with_id(MenuAction::PauseAll.id(), t.pause_all)
                .checked(model.all_paused())
                .enabled(model.connected() > 0)
                .build(app)?,
        )
        .item(&MenuItemBuilder::with_id(MenuAction::Settings.id(), t.settings).build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(MenuAction::Quit.id(), t.quit).build(app)?)
        .build()
}

/// Lê o estado atual do `Core` e monta o modelo da bandeja.
fn model_from(core: &Core, names: &HashMap<String, String>, rtt: &[Option<u32>; 4]) -> TrayModel {
    let snapshot = core.slots().snapshot();
    let language = core.settings().get().language;
    let slots = std::array::from_fn(|i| {
        let view = &snapshot.slots[i];
        let device = view.device.as_ref().map(|d| {
            names
                .get(d.as_str())
                .cloned()
                .unwrap_or_else(|| "?".to_owned())
        });
        SlotLine {
            slot: view.slot.index(),
            device,
            rtt_ms: rtt[i],
            paused: view.paused,
            reconnecting: matches!(view.state, SlotView::Suspended { .. }),
        }
    });
    TrayModel {
        language,
        driver_ready: matches!(core.driver_status(), DriverStatus::Ready),
        slots,
    }
}

async fn device_names(core: &Core) -> HashMap<String, String> {
    core.devices()
        .list()
        .await
        .map(|list| list.into_iter().map(|d| (d.id, d.name)).collect())
        .unwrap_or_default()
}

/// Cria o ícone da bandeja com o estado atual.
pub fn install<R: Runtime>(app: &AppHandle<R>, core: &Core) -> tauri::Result<TrayIcon<R>> {
    let model = model_from(core, &HashMap::new(), &[None; 4]);
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(model.tooltip())
        .menu(&build_menu(app, &model)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(action) = MenuAction::from_id(event.id().as_ref()) {
                run_action(app, action);
            }
        })
        .on_tray_icon_event(|tray, event| {
            // Clique esquerdo abre (ou cria) a janela; o direito fica com o menu.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                shell::show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = icon_image(model.icon()) {
        builder = builder.icon(icon);
    }
    builder.build(app)
}

fn run_action<R: Runtime>(app: &AppHandle<R>, action: MenuAction) {
    match action {
        MenuAction::Open => shell::show_main_window(app),
        MenuAction::Pair => shell::navigate(app, "/devices?pair=1"),
        MenuAction::Settings => shell::navigate(app, "/settings"),
        MenuAction::PauseAll => pause_all(app),
        MenuAction::Quit => {
            tracing::info!("saindo pelo menu da bandeja");
            app.exit(0);
        }
    }
}

/// Pausa todos os controles com celular; se já estão todos pausados, retoma.
fn pause_all<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let slots = state.core.slots();
    let snapshot = slots.snapshot();
    let occupied: Vec<_> = snapshot
        .slots
        .iter()
        .filter(|s| s.device.is_some())
        .collect();
    let resume = !occupied.is_empty() && occupied.iter().all(|s| s.paused);
    for slot in occupied {
        let _ = slots.set_paused(slot.slot, !resume);
    }
}

/// Mantém ícone, tooltip e menu em dia com os eventos do `Core`.
pub fn spawn_updater<R: Runtime>(app: AppHandle<R>, core: Arc<Core>, tray: TrayIcon<R>) {
    tauri::async_runtime::spawn(async move {
        let mut events = core.subscribe();
        let mut names = device_names(&core).await;
        let mut rtt = [None; 4];
        let mut shown: Option<TrayModel> = None;
        let mut poll = tokio::time::interval(POLL);
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(CoreEvent::Metrics(snapshot)) => {
                        rtt = [None; 4];
                        for m in &snapshot.slots {
                            if let Some(rtt_ms) = m.rtt_last {
                                rtt[usize::from(m.slot.index())] =
                                    Some(round_rtt(rtt_ms.as_secs_f64() * 1000.0));
                            }
                        }
                    }
                    Ok(CoreEvent::DevicesChanged) => names = device_names(&core).await,
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                },
                _ = poll.tick() => {}
            }
            let model = model_from(&core, &names, &rtt);
            if shown.as_ref() == Some(&model) {
                continue;
            }
            apply(&app, &tray, &model, shown.as_ref());
            shown = Some(model);
        }
    });
}

fn apply<R: Runtime>(
    app: &AppHandle<R>,
    tray: &TrayIcon<R>,
    model: &TrayModel,
    previous: Option<&TrayModel>,
) {
    let _ = tray.set_tooltip(Some(model.tooltip()));
    if previous.map(TrayModel::icon) != Some(model.icon()) {
        let _ = tray.set_icon(icon_image(model.icon()));
    }
    match build_menu(app, model) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(error) => tracing::warn!(%error, "não consegui atualizar o menu da bandeja"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(slot: u8, device: Option<&str>, rtt: Option<u32>) -> SlotLine {
        SlotLine {
            slot,
            device: device.map(str::to_owned),
            rtt_ms: rtt,
            paused: false,
            reconnecting: false,
        }
    }

    fn model(slots: [SlotLine; 4], driver_ready: bool, language: Language) -> TrayModel {
        TrayModel {
            language,
            driver_ready,
            slots,
        }
    }

    fn empty() -> [SlotLine; 4] {
        std::array::from_fn(|i| line(u8::try_from(i).unwrap(), None, None))
    }

    #[test]
    fn o_tooltip_conta_os_controles_no_idioma_certo() {
        let mut slots = empty();
        assert_eq!(
            model(slots.clone(), true, Language::PtBr).tooltip(),
            "VynPad — nenhum controle conectado"
        );
        slots[0] = line(0, Some("Galaxy"), Some(20));
        assert_eq!(
            model(slots.clone(), true, Language::PtBr).tooltip(),
            "VynPad — 1 controle conectado"
        );
        slots[2] = line(2, Some("Pixel"), None);
        assert_eq!(
            model(slots.clone(), true, Language::PtBr).tooltip(),
            "VynPad — 2 controles conectados"
        );
        assert_eq!(
            model(slots, true, Language::En).tooltip(),
            "VynPad — 2 controllers connected"
        );
    }

    #[test]
    fn driver_com_problema_vira_o_icone_de_erro_e_o_tooltip_diz_isso() {
        let mut slots = empty();
        slots[0] = line(0, Some("Galaxy"), None);
        let m = model(slots, false, Language::PtBr);
        assert_eq!(m.icon(), IconKind::Error);
        assert_eq!(m.tooltip(), "VynPad — driver com problema");
    }

    #[test]
    fn o_icone_segue_os_controles_conectados() {
        let m = model(empty(), true, Language::PtBr);
        assert_eq!(m.icon(), IconKind::Normal);
        let mut slots = empty();
        slots[3] = line(3, Some("Galaxy"), None);
        assert_eq!(model(slots, true, Language::PtBr).icon(), IconKind::Active);
    }

    #[test]
    fn as_linhas_dos_slots_mostram_nome_e_latencia() {
        let m = model(empty(), true, Language::PtBr);
        assert_eq!(m.slot_label(&line(1, None, None)), "P2 · livre");
        assert_eq!(
            m.slot_label(&line(0, Some("Galaxy da Ana"), Some(25))),
            "P1 · Galaxy da Ana · 25 ms"
        );
        assert_eq!(
            m.slot_label(&line(2, Some("Pixel"), None)),
            "P3 · Pixel",
            "sem medida ainda"
        );
        let mut paused = line(0, Some("Pixel"), Some(10));
        paused.paused = true;
        assert_eq!(m.slot_label(&paused), "P1 · Pixel · pausado");
        let mut reconnecting = line(0, Some("Pixel"), Some(10));
        reconnecting.reconnecting = true;
        assert_eq!(m.slot_label(&reconnecting), "P1 · Pixel · reconectando");
        let en = model(empty(), true, Language::En);
        assert_eq!(en.slot_label(&line(1, None, None)), "P2 · free");
    }

    #[test]
    fn pausar_todos_so_vale_com_controles_e_vira_retomar_quando_todos_estao_pausados() {
        let none = model(empty(), true, Language::PtBr);
        assert!(!none.all_paused(), "sem controles não há o que retomar");

        let mut slots = empty();
        slots[0] = line(0, Some("A"), None);
        slots[1] = line(1, Some("B"), None);
        assert!(!model(slots.clone(), true, Language::PtBr).all_paused());
        slots[0].paused = true;
        assert!(!model(slots.clone(), true, Language::PtBr).all_paused());
        slots[1].paused = true;
        assert!(model(slots, true, Language::PtBr).all_paused());
    }

    #[test]
    fn a_latencia_do_menu_anda_de_5_em_5_ms() {
        assert_eq!(round_rtt(0.4), 0);
        assert_eq!(round_rtt(2.4), 0);
        assert_eq!(round_rtt(2.6), 5);
        assert_eq!(round_rtt(21.0), 20);
        assert_eq!(round_rtt(23.0), 25);
        assert_eq!(round_rtt(-3.0), 0);
    }

    #[test]
    fn os_ids_do_menu_fazem_o_caminho_de_ida_e_volta() {
        for action in [
            MenuAction::Open,
            MenuAction::Pair,
            MenuAction::PauseAll,
            MenuAction::Settings,
            MenuAction::Quit,
        ] {
            assert_eq!(MenuAction::from_id(action.id()), Some(action));
        }
        assert_eq!(MenuAction::from_id("slot-0"), None);
    }

    #[test]
    fn os_icones_da_bandeja_decodificam() {
        for kind in [IconKind::Normal, IconKind::Active, IconKind::Error] {
            assert!(icon_image(kind).is_some(), "{kind:?}");
        }
    }
}
