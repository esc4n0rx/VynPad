//! Dispositivos próximos: os clients que anunciam `_vynpad-client._tcp` (PROTOCOLO §3.1).
//!
//! [`NearbyTable`] é a lógica pura (inserção, expiração de 15 s e debounce de 500 ms), testada
//! com tempo simulado. [`NearbyBrowser`] liga a tabela ao `mdns-sd`.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use vynpad_protocol::control::Uuid;
use vynpad_protocol::discovery::{CLIENT_SERVICE, txt};

use super::DiscoveryError;

/// Sem anúncio por este tempo o dispositivo some da lista.
pub const EXPIRE_AFTER: Duration = Duration::from_secs(15);
/// As mudanças são agrupadas por este tempo antes de avisar a UI.
pub const DEBOUNCE: Duration = Duration::from_millis(500);
/// Sem ser visto por este tempo, o browser pede ao `mdns-sd` que confirme o anúncio.
pub const VERIFY_AFTER: Duration = Duration::from_secs(8);
/// Prazo da confirmação.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(4);
/// Intervalo entre passadas de manutenção do browser.
const POLL: Duration = Duration::from_millis(100);

/// Um aparelho anunciando `_vynpad-client._tcp` na rede.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearbyDevice {
    /// `device_id` do aparelho (TXT `id`).
    pub id: Uuid,
    /// Nome exibido (TXT `name`).
    pub name: String,
    /// Modelo (TXT `model`).
    pub model: String,
    /// Endereço IPv4.
    pub ip: Ipv4Addr,
    /// Porta anunciada (só informativa: o server nunca conecta nela).
    pub port: u16,
    /// Há quanto tempo o anúncio foi visto pela última vez.
    pub seen_ago: Duration,
    /// O aparelho já está pareado (e não revogado) neste server.
    pub paired: bool,
}

/// O que um anúncio diz, sem o tempo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    /// Nome completo da instância mDNS (chave da tabela).
    pub fullname: String,
    /// `device_id`.
    pub id: Uuid,
    /// Nome exibido.
    pub name: String,
    /// Modelo.
    pub model: String,
    /// Endereço IPv4.
    pub ip: Ipv4Addr,
    /// Porta.
    pub port: u16,
}

struct Entry {
    announcement: Announcement,
    last_seen: Instant,
    verified_at: Option<Instant>,
}

/// A lista de dispositivos próximos, sem I/O.
#[derive(Default)]
pub struct NearbyTable {
    entries: HashMap<String, Entry>,
    dirty_since: Option<Instant>,
}

impl NearbyTable {
    fn mark_dirty(&mut self, now: Instant) {
        self.dirty_since.get_or_insert(now);
    }

    /// Anúncio visto agora. Só muda a lista (e o debounce) se algo mudou de fato.
    pub fn upsert(&mut self, announcement: Announcement, now: Instant) {
        let changed = if let Some(entry) = self.entries.get_mut(&announcement.fullname) {
            entry.last_seen = now;
            entry.verified_at = None;
            let changed = entry.announcement != announcement;
            entry.announcement = announcement;
            changed
        } else {
            self.entries.insert(
                announcement.fullname.clone(),
                Entry {
                    announcement,
                    last_seen: now,
                    verified_at: None,
                },
            );
            true
        };
        if changed {
            self.mark_dirty(now);
        }
    }

    /// O aparelho se despediu (ou o `mdns-sd` o removeu).
    pub fn remove(&mut self, fullname: &str, now: Instant) {
        if self.entries.remove(fullname).is_some() {
            self.mark_dirty(now);
        }
    }

    /// Remove quem não é visto há [`EXPIRE_AFTER`].
    pub fn expire(&mut self, now: Instant) {
        let before = self.entries.len();
        self.entries
            .retain(|_, e| now.saturating_duration_since(e.last_seen) < EXPIRE_AFTER);
        if self.entries.len() != before {
            self.mark_dirty(now);
        }
    }

    /// Instâncias que valem uma confirmação ativa: caladas há [`VERIFY_AFTER`] e ainda não
    /// confirmadas nesta rodada. Marca-as como confirmadas.
    pub fn take_stale(&mut self, now: Instant) -> Vec<String> {
        let mut stale = Vec::new();
        for (name, entry) in &mut self.entries {
            if entry.verified_at.is_none()
                && now.saturating_duration_since(entry.last_seen) >= VERIFY_AFTER
            {
                entry.verified_at = Some(now);
                stale.push(name.clone());
            }
        }
        stale
    }

    /// Passou o debounce desde a primeira mudança pendente? Se sim, consome a pendência.
    pub fn take_due(&mut self, now: Instant) -> bool {
        match self.dirty_since {
            Some(since) if now.saturating_duration_since(since) >= DEBOUNCE => {
                self.dirty_since = None;
                true
            }
            _ => false,
        }
    }

    /// A lista atual, ordenada por nome. `paired` diz se o `device_id` já é pareado.
    pub fn snapshot(&self, now: Instant, paired: &dyn Fn(&Uuid) -> bool) -> Vec<NearbyDevice> {
        let mut list: Vec<NearbyDevice> = self
            .entries
            .values()
            .map(|e| NearbyDevice {
                id: e.announcement.id.clone(),
                name: e.announcement.name.clone(),
                model: e.announcement.model.clone(),
                ip: e.announcement.ip,
                port: e.announcement.port,
                seen_ago: now.saturating_duration_since(e.last_seen),
                paired: paired(&e.announcement.id),
            })
            .collect();
        list.sort_by(|a, b| (&a.name, a.id.as_str()).cmp(&(&b.name, b.id.as_str())));
        list
    }

    /// Quantos dispositivos estão na lista.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// A lista está vazia?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Quem responde "este `device_id` já é pareado?" (na T-403, o `DeviceStore`).
pub type PairedLookup = Arc<dyn Fn(&Uuid) -> bool + Send + Sync>;
/// Quem recebe a lista atualizada (na T-506, o evento `devices:discovered`).
pub type OnNearby = Arc<dyn Fn(Vec<NearbyDevice>) + Send + Sync>;

/// O browser de `_vynpad-client._tcp` em execução.
pub struct NearbyBrowser {
    table: Arc<Mutex<NearbyTable>>,
    paired: PairedLookup,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NearbyBrowser {
    /// Começa a procurar clients no `daemon`. `on_change` é chamado (na thread do browser) com a
    /// lista completa, no máximo a cada 500 ms.
    pub fn start(
        daemon: &ServiceDaemon,
        paired: PairedLookup,
        on_change: OnNearby,
    ) -> Result<Self, DiscoveryError> {
        let events = daemon.browse(CLIENT_SERVICE)?;
        let table = Arc::new(Mutex::new(NearbyTable::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = std::thread::Builder::new()
            .name("discovery-browse".into())
            .spawn({
                let (table, paired, stop) =
                    (Arc::clone(&table), Arc::clone(&paired), Arc::clone(&stop));
                let daemon = daemon.clone();
                move || run(&daemon, &events, &table, &paired, &on_change, &stop)
            })?;
        Ok(Self {
            table,
            paired,
            stop,
            thread: Some(thread),
        })
    }

    /// A lista neste instante (para a UI pedir ao abrir a tela).
    pub fn snapshot(&self) -> Vec<NearbyDevice> {
        self.table
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .snapshot(Instant::now(), &*self.paired)
    }

    /// Para de procurar.
    pub fn shutdown(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for NearbyBrowser {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// Converte um anúncio resolvido pelo `mdns-sd`. `None` se faltar o `id` ou ele for inválido,
/// ou se não houver IPv4.
fn announcement_from(resolved: &mdns_sd::ResolvedService) -> Option<Announcement> {
    let id = Uuid::parse(resolved.get_property_val_str(txt::ID)?).ok()?;
    let mut addrs: Vec<Ipv4Addr> = resolved.get_addresses_v4().into_iter().collect();
    // Prefere endereço privado (o de LAN) e, entre eles, o menor, para ser estável.
    addrs.sort_by_key(|ip| (!ip.is_private(), *ip));
    let ip = *addrs.first()?;
    let fullname = resolved.get_fullname().to_owned();
    let instance = fullname
        .strip_suffix(CLIENT_SERVICE)
        .map_or(fullname.as_str(), |s| s.trim_end_matches('.'))
        .to_owned();
    let name = resolved
        .get_property_val_str(txt::NAME)
        .filter(|n| !n.trim().is_empty())
        .map_or(instance, str::to_owned);
    let model = resolved
        .get_property_val_str(txt::MODEL)
        .unwrap_or_default()
        .to_owned();
    Some(Announcement {
        fullname,
        id,
        name,
        model,
        ip,
        port: resolved.get_port(),
    })
}

fn run(
    daemon: &ServiceDaemon,
    events: &mdns_sd::Receiver<ServiceEvent>,
    table: &Mutex<NearbyTable>,
    paired: &PairedLookup,
    on_change: &OnNearby,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Relaxed) {
        let event = events.recv_timeout(POLL).ok();
        let now = Instant::now();
        let mut publish = None;
        let mut to_verify = Vec::new();
        {
            let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
            match event {
                Some(ServiceEvent::ServiceResolved(resolved)) => {
                    if let Some(announcement) = announcement_from(&resolved) {
                        table.upsert(announcement, now);
                    }
                }
                Some(ServiceEvent::ServiceRemoved(_, fullname)) => table.remove(&fullname, now),
                _ => {}
            }
            table.expire(now);
            to_verify.extend(table.take_stale(now));
            if table.take_due(now) {
                publish = Some(table.snapshot(now, &**paired));
            }
        }
        for fullname in to_verify {
            // Se o aparelho ainda está lá ele responde e o anúncio se renova; se não, o
            // `mdns-sd` o remove (e a expiração de 15 s é a rede de segurança).
            let _ = daemon.verify(fullname, VERIFY_TIMEOUT);
        }
        if let Some(list) = publish {
            on_change(list);
        }
    }
    let _ = daemon.stop_browse(CLIENT_SERVICE);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ann(n: u8, name: &str) -> Announcement {
        Announcement {
            fullname: format!("{name}._vynpad-client._tcp.local."),
            id: Uuid::parse(&format!("00000000-0000-4000-8000-0000000000{n:02}")).unwrap(),
            name: name.into(),
            model: "Pixel 8".into(),
            ip: Ipv4Addr::new(192, 168, 0, 40 + n),
            port: 50000,
        }
    }

    fn never(_: &Uuid) -> bool {
        false
    }

    #[test]
    fn a_new_device_appears_after_the_debounce_and_not_before() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(1, "Pixel"), t0);
        assert!(!table.take_due(t0 + Duration::from_millis(499)));
        assert!(table.take_due(t0 + Duration::from_millis(500)));
        // Já consumido: sem mudança nova, nada a avisar.
        assert!(!table.take_due(t0 + Duration::from_secs(5)));
        let list = table.snapshot(t0 + Duration::from_millis(500), &never);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Pixel");
        assert_eq!(list[0].seen_ago, Duration::from_millis(500));
    }

    #[test]
    fn a_burst_of_changes_becomes_one_notification() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        for (i, name) in ["A", "B", "C"].into_iter().enumerate() {
            table.upsert(
                ann(u8::try_from(i).unwrap(), name),
                t0 + Duration::from_millis(100 * i as u64),
            );
        }
        assert!(!table.take_due(t0 + Duration::from_millis(499)));
        assert!(table.take_due(t0 + Duration::from_millis(500)));
        assert!(!table.take_due(t0 + Duration::from_millis(600)));
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn repeating_the_same_announcement_refreshes_the_clock_without_notifying() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(1, "Pixel"), t0);
        assert!(table.take_due(t0 + DEBOUNCE));
        table.upsert(ann(1, "Pixel"), t0 + Duration::from_secs(3));
        assert!(!table.take_due(t0 + Duration::from_secs(4)));
        let list = table.snapshot(t0 + Duration::from_secs(4), &never);
        assert_eq!(list[0].seen_ago, Duration::from_secs(1));
    }

    #[test]
    fn a_changed_name_or_ip_notifies() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(1, "Pixel"), t0);
        assert!(table.take_due(t0 + DEBOUNCE));
        let mut moved = ann(1, "Pixel");
        moved.ip = Ipv4Addr::new(192, 168, 0, 99);
        table.upsert(moved, t0 + Duration::from_secs(1));
        assert!(table.take_due(t0 + Duration::from_secs(2)));
    }

    #[test]
    fn a_device_that_stops_announcing_disappears_within_15_seconds() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(1, "Pixel"), t0);
        table.upsert(ann(2, "Galaxy"), t0);
        // O Galaxy continua anunciando; o Pixel some.
        table.upsert(ann(2, "Galaxy"), t0 + Duration::from_secs(10));
        table.expire(t0 + Duration::from_millis(14_999));
        assert_eq!(table.len(), 2);
        table.expire(t0 + Duration::from_secs(15));
        let list = table.snapshot(t0 + Duration::from_secs(15), &never);
        assert_eq!(
            list.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["Galaxy"]
        );
        assert!(
            table.take_due(t0 + Duration::from_secs(16)),
            "a saída também notifica"
        );
    }

    #[test]
    fn goodbye_removes_at_once_and_unknown_names_are_harmless() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(1, "Pixel"), t0);
        assert!(table.take_due(t0 + DEBOUNCE));
        table.remove(
            "outro._vynpad-client._tcp.local.",
            t0 + Duration::from_secs(1),
        );
        assert!(!table.take_due(t0 + Duration::from_secs(2)), "nada mudou");
        table.remove(&ann(1, "Pixel").fullname, t0 + Duration::from_secs(2));
        assert!(table.is_empty());
        assert!(table.take_due(t0 + Duration::from_secs(3)));
    }

    #[test]
    fn stale_entries_are_verified_once_per_silence() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(1, "Pixel"), t0);
        assert!(table.take_stale(t0 + Duration::from_secs(7)).is_empty());
        assert_eq!(table.take_stale(t0 + Duration::from_secs(8)).len(), 1);
        assert!(
            table.take_stale(t0 + Duration::from_secs(9)).is_empty(),
            "só uma vez"
        );
        // Um anúncio novo reabre a possibilidade.
        table.upsert(ann(1, "Pixel"), t0 + Duration::from_secs(10));
        assert_eq!(table.take_stale(t0 + Duration::from_secs(18)).len(), 1);
    }

    #[test]
    fn the_snapshot_marks_paired_devices_and_sorts_by_name() {
        let t0 = Instant::now();
        let mut table = NearbyTable::default();
        table.upsert(ann(2, "Zeta"), t0);
        table.upsert(ann(1, "Alfa"), t0);
        let paired_id = ann(1, "Alfa").id;
        let list = table.snapshot(t0, &|id| *id == paired_id);
        assert_eq!(
            list.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["Alfa", "Zeta"]
        );
        assert_eq!(
            list.iter().map(|d| d.paired).collect::<Vec<_>>(),
            [true, false]
        );
    }
}
