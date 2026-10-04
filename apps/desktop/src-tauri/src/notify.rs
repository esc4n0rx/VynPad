//! Avisos sobre o que acontece com os celulares e com o driver (T-608).
//!
//! Quem decide é o backend, porque a janela pode estar fechada: com a janela à vista o aviso vai
//! para a UI como toast (`notice`), sem ela vira notificação nativa do sistema. O que é puro e
//! testado fica aqui em cima: o agrupamento de eventos repetidos ([`Coalescer`]), a política
//! (tipos ligados, jogo em tela cheia, janela à vista) e os textos. O resto é cola.
//!
//! Agrupar: eventos iguais (mesmo tipo, mesmo dispositivo) esperam [`QUIET`] sem novidade (ou
//! [`MAX`] no total) e saem como **um** aviso com a contagem ("reconectou 10 vezes"), em vez de
//! dez avisos.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;
use tauri_specta::Event;
use tokio::sync::broadcast::error::RecvError;
use vynpad_driver::DriverStatus;
use vynpad_net::control::{ControlEvent, SessionEnd};
use vynpad_protocol::control::RejectReason;
use vynpad_server::{ApprovalEvent, Core, CoreEvent};
use vynpad_storage::{AppSettings, Language};

use crate::shell;

/// Sem eventos novos por este tempo, o grupo sai.
pub const QUIET: Duration = Duration::from_millis(1500);
/// Um grupo nunca espera mais que isto, mesmo com eventos chegando (no máximo um aviso a cada 5 s).
pub const MAX: Duration = Duration::from_secs(5);
/// Uma conexão que volta em até este tempo depois de cair conta como "reconectou".
const RECONNECT_WINDOW: Duration = Duration::from_secs(30);
/// De quanto em quanto tempo os grupos prontos saem e o driver é conferido.
const TICK: Duration = Duration::from_millis(500);

/// A que opção das configurações um aviso pertence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Conectou, reconectou ou desconectou.
    Connections,
    /// Pareou, pediu aprovação ou foi recusado.
    Pairing,
    /// O driver de controle virtual.
    Driver,
}

/// O que aconteceu.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Connected { slot: u8 },
    Reconnected { slot: u8 },
    Disconnected { slot: u8 },
    Paired,
    Rejected { reason: &'static str },
    DriverDown,
    DriverUp,
}

impl Kind {
    pub const fn category(&self) -> Category {
        match self {
            Self::Connected { .. } | Self::Reconnected { .. } | Self::Disconnected { .. } => {
                Category::Connections
            }
            Self::Paired | Self::Rejected { .. } => Category::Pairing,
            Self::DriverDown | Self::DriverUp => Category::Driver,
        }
    }

    const fn level(&self) -> Level {
        match self {
            Self::Connected { .. } | Self::Reconnected { .. } | Self::Paired | Self::DriverUp => {
                Level::Success
            }
            Self::Disconnected { .. } => Level::Info,
            Self::Rejected { .. } | Self::DriverDown => Level::Error,
        }
    }
}

/// O tom do aviso (a cor do toast).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Info,
    Success,
    Error,
}

/// Um acontecimento, antes de agrupar. O `who` separa os grupos (o dispositivo, ou um id fixo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Happening {
    pub who: String,
    /// Nome para mostrar (o do dispositivo).
    pub name: String,
    pub kind: Kind,
}

/// Um aviso pronto para sair: o acontecimento e quantas vezes ele se repetiu no grupo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub name: String,
    pub kind: Kind,
    pub count: u32,
}

impl Notice {
    pub fn category(&self) -> Category {
        self.kind.category()
    }

    pub fn level(&self) -> Level {
        self.kind.level()
    }
}

struct Group {
    event: Happening,
    count: u32,
    first_at: Instant,
    last_at: Instant,
}

/// Junta acontecimentos iguais que chegam em sequência.
#[derive(Default)]
pub struct Coalescer {
    groups: HashMap<(Kind, String), Group>,
}

impl Coalescer {
    pub fn push(&mut self, event: Happening, now: Instant) {
        let key = (event.kind.clone(), event.who.clone());
        self.groups
            .entry(key)
            .and_modify(|g| {
                g.count += 1;
                g.last_at = now;
            })
            .or_insert(Group {
                event,
                count: 1,
                first_at: now,
                last_at: now,
            });
    }

    /// Os grupos que já podem sair (quietos ou velhos demais), do mais antigo ao mais novo.
    pub fn due(&mut self, now: Instant) -> Vec<Notice> {
        let ready: Vec<_> = self
            .groups
            .iter()
            .filter(|(_, g)| {
                now.saturating_duration_since(g.last_at) >= QUIET
                    || now.saturating_duration_since(g.first_at) >= MAX
            })
            .map(|(key, g)| (key.clone(), g.first_at))
            .collect();
        let mut ready = ready;
        ready.sort_by_key(|(_, at)| *at);
        ready
            .into_iter()
            .filter_map(|(key, _)| self.groups.remove(&key))
            .map(|g| Notice {
                name: g.event.name,
                kind: g.event.kind,
                count: g.count,
            })
            .collect()
    }
}

/// Para onde um aviso vai.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Não avisa.
    Drop,
    /// Toast dentro da janela (ela está à vista).
    Toast,
    /// Notificação do sistema (janela fechada, escondida ou minimizada).
    Native,
}

/// A política: o tipo está ligado? tem um jogo em tela cheia? a janela está à vista?
pub fn delivery(
    settings: &AppSettings,
    category: Category,
    window_visible: bool,
    fullscreen: bool,
) -> Delivery {
    let enabled = match category {
        Category::Connections => settings.notify_connections,
        Category::Pairing => settings.notify_pairing,
        Category::Driver => settings.notify_driver,
    };
    if !enabled || (fullscreen && settings.notify_skip_fullscreen) {
        Delivery::Drop
    } else if window_visible {
        Delivery::Toast
    } else {
        Delivery::Native
    }
}

fn reject_reason(reason: RejectReason) -> &'static str {
    match reason {
        RejectReason::BadAuth => "bad_auth",
        RejectReason::Revoked => "revoked",
        RejectReason::NoSlot => "no_slot",
        RejectReason::Locked => "locked",
        RejectReason::Version => "version",
        _ => "other",
    }
}

/// O texto do aviso no idioma do app.
pub fn render(notice: &Notice, language: Language) -> String {
    let pt = language == Language::PtBr;
    let name = &notice.name;
    let n = notice.count;
    let slot = |s: u8| format!("P{}", s + 1);
    match (&notice.kind, n) {
        (Kind::Connected { slot: s }, _) if pt => format!("{name} conectado como {}", slot(*s)),
        (Kind::Connected { slot: s }, _) => format!("{name} connected as {}", slot(*s)),
        (Kind::Reconnected { slot: s }, 1) if pt => format!("{name} voltou em {}", slot(*s)),
        (Kind::Reconnected { slot: s }, 1) => format!("{name} is back on {}", slot(*s)),
        (Kind::Reconnected { slot: s }, _) if pt => {
            format!("{name} reconectou {n} vezes em {}", slot(*s))
        }
        (Kind::Reconnected { slot: s }, _) => {
            format!("{name} reconnected {n} times on {}", slot(*s))
        }
        (Kind::Disconnected { slot: s }, 1) if pt => format!("{name} desconectou de {}", slot(*s)),
        (Kind::Disconnected { slot: s }, 1) => format!("{name} disconnected from {}", slot(*s)),
        (Kind::Disconnected { slot: s }, _) if pt => {
            format!("{name} desconectou {n} vezes de {}", slot(*s))
        }
        (Kind::Disconnected { slot: s }, _) => {
            format!("{name} disconnected {n} times from {}", slot(*s))
        }
        (Kind::Paired, _) if pt => format!("{name} foi pareado"),
        (Kind::Paired, _) => format!("{name} was paired"),
        (Kind::Rejected { reason }, 1) => {
            let why = reject_text(reason, pt);
            if pt {
                format!("Conexão recusada: {why}")
            } else {
                format!("Connection refused: {why}")
            }
        }
        (Kind::Rejected { reason }, _) => {
            let why = reject_text(reason, pt);
            if pt {
                format!("{n} conexões recusadas: {why}")
            } else {
                format!("{n} connections refused: {why}")
            }
        }
        (Kind::DriverDown, _) if pt => "O driver de controle virtual parou de funcionar".to_owned(),
        (Kind::DriverDown, _) => "The virtual controller driver stopped working".to_owned(),
        (Kind::DriverUp, _) if pt => "O driver de controle virtual voltou a funcionar".to_owned(),
        (Kind::DriverUp, _) => "The virtual controller driver is working again".to_owned(),
    }
}

fn reject_text(reason: &str, pt: bool) -> &'static str {
    match (reason, pt) {
        ("bad_auth", true) => "credencial inválida",
        ("bad_auth", false) => "invalid credential",
        ("revoked", true) => "dispositivo revogado",
        ("revoked", false) => "revoked device",
        ("no_slot", true) => "sem slot livre",
        ("no_slot", false) => "no free slot",
        ("locked", true) => "muitas tentativas",
        ("locked", false) => "too many attempts",
        ("version", true) => "versão incompatível, atualize o app no celular",
        ("version", false) => "incompatible version, update the app on the phone",
        (_, true) => "motivo desconhecido",
        (_, false) => "unknown reason",
    }
}

/// O aviso para a UI mostrar como toast (`notice`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct NoticeEvent {
    pub level: Level,
    pub message: String,
}

// ------------------------------------------------------------------ cola com o Tauri

fn window_visible<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window(shell::MAIN_WINDOW)
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

fn deliver<R: Runtime>(app: &AppHandle<R>, core: &Core, notice: &Notice) {
    let settings = core.settings().get();
    let way = delivery(
        &settings,
        notice.category(),
        window_visible(app),
        vynpad_net::user_state::fullscreen_busy(),
    );
    let message = render(notice, settings.language);
    match way {
        Delivery::Drop => {}
        Delivery::Toast => {
            let event = NoticeEvent {
                level: notice.level(),
                message,
            };
            if let Err(error) = event.emit(app) {
                tracing::warn!(%error, "não consegui mostrar o aviso na janela");
            }
        }
        Delivery::Native => {
            if let Err(error) = app
                .notification()
                .builder()
                .title("VynPad")
                .body(message)
                .show()
            {
                tracing::warn!(%error, "não consegui mostrar a notificação do sistema");
            }
        }
    }
}

/// Liga o `Core` aos avisos: agrupa os eventos e entrega o que sobra, de acordo com as
/// configurações.
pub fn spawn<R: Runtime>(app: AppHandle<R>, core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        let mut events = core.subscribe();
        let mut coalescer = Coalescer::default();
        let mut ended: HashMap<String, Instant> = HashMap::new();
        let mut driver_ready: Option<bool> = None;
        let mut tick = tokio::time::interval(TICK);
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => handle(&app, &core, &event, &mut coalescer, &mut ended).await,
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                },
                _ = tick.tick() => {
                    let ready = matches!(core.driver_status(), DriverStatus::Ready);
                    // A primeira leitura só marca o ponto de partida: abrir o app com o driver
                    // ausente não é uma novidade.
                    if let Some(before) = driver_ready.replace(ready)
                        && before != ready
                    {
                        let kind = if ready { Kind::DriverUp } else { Kind::DriverDown };
                        coalescer.push(
                            Happening { who: "driver".to_owned(), name: String::new(), kind },
                            Instant::now(),
                        );
                    }
                    for notice in coalescer.due(Instant::now()) {
                        deliver(&app, &core, &notice);
                    }
                }
            }
        }
    });
}

async fn device_name(core: &Core, id: &str) -> String {
    core.devices()
        .find(id)
        .await
        .ok()
        .flatten()
        .map_or_else(|| "?".to_owned(), |d| d.name)
}

async fn handle<R: Runtime>(
    app: &AppHandle<R>,
    core: &Core,
    event: &CoreEvent,
    coalescer: &mut Coalescer,
    ended: &mut HashMap<String, Instant>,
) {
    let now = Instant::now();
    match event {
        CoreEvent::Control(ControlEvent::SessionStarted {
            slot, device_id, ..
        }) => {
            let id = device_id.as_str().to_owned();
            let back = ended
                .remove(&id)
                .is_some_and(|at| now.saturating_duration_since(at) <= RECONNECT_WINDOW);
            let slot = slot.index();
            let kind = if back {
                Kind::Reconnected { slot }
            } else {
                Kind::Connected { slot }
            };
            let name = device_name(core, &id).await;
            coalescer.push(
                Happening {
                    who: id,
                    name,
                    kind,
                },
                now,
            );
        }
        CoreEvent::Control(ControlEvent::SessionEnded {
            slot,
            device_id,
            reason,
            ..
        }) => {
            // Desconectar pela UI, revogar e fechar o app foram ações do usuário: sem aviso.
            if !matches!(
                reason,
                SessionEnd::ClientBye | SessionEnd::Dropped | SessionEnd::Timeout
            ) {
                return;
            }
            let id = device_id.as_str().to_owned();
            ended.insert(id.clone(), now);
            let name = device_name(core, &id).await;
            let kind = Kind::Disconnected { slot: slot.index() };
            coalescer.push(
                Happening {
                    who: id,
                    name,
                    kind,
                },
                now,
            );
        }
        CoreEvent::Control(ControlEvent::Paired {
            device_id, name, ..
        }) => coalescer.push(
            Happening {
                who: device_id.as_str().to_owned(),
                name: name.clone(),
                kind: Kind::Paired,
            },
            now,
        ),
        CoreEvent::Control(ControlEvent::Rejected {
            device_id, reason, ..
        }) => {
            let reason = reject_reason(*reason);
            coalescer.push(
                Happening {
                    who: format!("rejected-{reason}"),
                    name: device_id
                        .as_ref()
                        .map_or_else(String::new, |d| d.as_str().to_owned()),
                    kind: Kind::Rejected { reason },
                },
                now,
            );
        }
        // O pedido de confirmação só adianta se o usuário vê o diálogo: traz a janela para a
        // frente (a menos que um jogo ocupe a tela e a opção peça silêncio).
        CoreEvent::Approval(ApprovalEvent::Requested(_)) => {
            let settings = core.settings().get();
            if settings.notify_pairing
                && !(vynpad_net::user_state::fullscreen_busy() && settings.notify_skip_fullscreen)
            {
                shell::show_main_window(app);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(who: &str, kind: Kind) -> Happening {
        Happening {
            who: who.to_owned(),
            name: "Galaxy da Ana".to_owned(),
            kind,
        }
    }

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    #[test]
    fn um_evento_sozinho_sai_depois_do_silencio() {
        let t0 = Instant::now();
        let mut c = Coalescer::default();
        c.push(event("a", Kind::Connected { slot: 0 }), t0);
        assert!(c.due(at(t0, 1000)).is_empty(), "ainda esperando");
        let out = c.due(at(t0, 1600));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].count, 1);
        assert!(c.due(at(t0, 5000)).is_empty(), "sai uma vez só");
    }

    #[test]
    fn dez_reconexoes_em_5_segundos_viram_um_unico_aviso_com_a_contagem() {
        let t0 = Instant::now();
        let mut c = Coalescer::default();
        let mut out = Vec::new();
        // A cada 500 ms, por 5 s: o silêncio nunca chega a 1,5 s.
        for i in 0..10 {
            let now = at(t0, i * 500);
            c.push(event("a", Kind::Reconnected { slot: 0 }), now);
            out.extend(c.due(now));
        }
        assert!(out.is_empty(), "nada saiu no meio da rajada");
        out.extend(c.due(at(t0, 5000)));
        assert_eq!(out.len(), 1, "um aviso só");
        assert_eq!(out[0].count, 10);
        assert_eq!(
            render(&out[0], Language::PtBr),
            "Galaxy da Ana reconectou 10 vezes em P1"
        );
    }

    #[test]
    fn uma_rajada_longa_nao_segura_o_aviso_alem_do_maximo() {
        let t0 = Instant::now();
        let mut c = Coalescer::default();
        for i in 0..40 {
            c.push(event("a", Kind::Disconnected { slot: 1 }), at(t0, i * 250));
        }
        // Passaram 9,75 s de eventos contínuos: o limite de 5 s já tinha vencido.
        let out = c.due(at(t0, 9750));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].count, 40);
    }

    #[test]
    fn dispositivos_e_tipos_diferentes_nao_se_misturam() {
        let t0 = Instant::now();
        let mut c = Coalescer::default();
        c.push(event("a", Kind::Connected { slot: 0 }), t0);
        c.push(event("b", Kind::Connected { slot: 1 }), at(t0, 100));
        c.push(event("a", Kind::Disconnected { slot: 0 }), at(t0, 200));
        let out = c.due(at(t0, 2000));
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|n| n.count == 1));
        // Do mais antigo para o mais novo.
        assert!(matches!(out[0].kind, Kind::Connected { slot: 0 }));
    }

    #[test]
    fn um_grupo_novo_nasce_depois_que_o_anterior_saiu() {
        let t0 = Instant::now();
        let mut c = Coalescer::default();
        c.push(event("a", Kind::Paired), t0);
        assert_eq!(c.due(at(t0, 2000)).len(), 1);
        c.push(event("a", Kind::Paired), at(t0, 2100));
        let out = c.due(at(t0, 4000));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].count, 1);
    }

    fn settings() -> AppSettings {
        AppSettings::default()
    }

    #[test]
    fn com_a_janela_a_vista_vira_toast_e_sem_ela_vira_notificacao_nativa() {
        let s = settings();
        assert_eq!(
            delivery(&s, Category::Connections, true, false),
            Delivery::Toast
        );
        assert_eq!(
            delivery(&s, Category::Connections, false, false),
            Delivery::Native
        );
    }

    #[test]
    fn cada_tipo_pode_ser_desligado() {
        let mut s = settings();
        s.notify_connections = false;
        assert_eq!(
            delivery(&s, Category::Connections, false, false),
            Delivery::Drop
        );
        assert_eq!(
            delivery(&s, Category::Pairing, false, false),
            Delivery::Native
        );
        s.notify_pairing = false;
        assert_eq!(delivery(&s, Category::Pairing, true, false), Delivery::Drop);
        s.notify_driver = false;
        assert_eq!(delivery(&s, Category::Driver, false, false), Delivery::Drop);
    }

    #[test]
    fn nenhum_aviso_com_um_jogo_em_tela_cheia_quando_a_opcao_esta_ligada() {
        let mut s = settings();
        for category in [Category::Connections, Category::Pairing, Category::Driver] {
            assert_eq!(delivery(&s, category, false, true), Delivery::Drop);
            assert_eq!(delivery(&s, category, true, true), Delivery::Drop);
        }
        s.notify_skip_fullscreen = false;
        assert_eq!(
            delivery(&s, Category::Driver, false, true),
            Delivery::Native
        );
    }

    #[test]
    fn os_textos_seguem_o_idioma_e_a_contagem() {
        let n = |kind, count| Notice {
            name: "Pixel".to_owned(),
            kind,
            count,
        };
        assert_eq!(
            render(&n(Kind::Connected { slot: 2 }, 1), Language::PtBr),
            "Pixel conectado como P3"
        );
        assert_eq!(
            render(&n(Kind::Connected { slot: 2 }, 1), Language::En),
            "Pixel connected as P3"
        );
        assert_eq!(
            render(&n(Kind::Reconnected { slot: 0 }, 1), Language::PtBr),
            "Pixel voltou em P1"
        );
        assert_eq!(
            render(&n(Kind::Disconnected { slot: 0 }, 3), Language::En),
            "Pixel disconnected 3 times from P1"
        );
        assert_eq!(
            render(&n(Kind::Paired, 1), Language::PtBr),
            "Pixel foi pareado"
        );
        assert_eq!(
            render(&n(Kind::Rejected { reason: "revoked" }, 1), Language::PtBr),
            "Conexão recusada: dispositivo revogado"
        );
        assert_eq!(
            render(&n(Kind::Rejected { reason: "bad_auth" }, 4), Language::En),
            "4 connections refused: invalid credential"
        );
        assert_eq!(
            render(&n(Kind::DriverDown, 1), Language::PtBr),
            "O driver de controle virtual parou de funcionar"
        );
        assert_eq!(
            render(&n(Kind::DriverUp, 1), Language::En),
            "The virtual controller driver is working again"
        );
    }

    #[test]
    fn o_tipo_e_o_tom_de_cada_aviso() {
        assert_eq!(
            Kind::Connected { slot: 0 }.category(),
            Category::Connections
        );
        assert_eq!(Kind::Paired.category(), Category::Pairing);
        assert_eq!(Kind::Rejected { reason: "x" }.category(), Category::Pairing);
        assert_eq!(Kind::DriverDown.category(), Category::Driver);
        assert_eq!(Kind::DriverDown.level(), Level::Error);
        assert_eq!(Kind::Disconnected { slot: 0 }.level(), Level::Info);
        assert_eq!(Kind::DriverUp.level(), Level::Success);
    }
}
