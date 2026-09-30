//! `input:live` sob demanda: o backend só amostra e emite o estado dos controles enquanto alguma
//! tela estiver assinando o slot (`subscribe_live_input` / `unsubscribe_live_input`).
//!
//! A amostragem lê o último estado enviado ao driver (`SlotManager::live_report`, sem lock) a
//! ~60 Hz e só emite o que mudou. Sem nenhuma assinatura, a tarefa dorme e não toca no hot path.

use std::sync::Mutex;
use std::time::Duration;

use padgaming_core::SlotManager;
use padgaming_driver::{SlotId, XReport};
use tokio::sync::Notify;

use crate::dto::InputReportDto;

/// Intervalo entre amostras (~60 Hz).
const SAMPLE_INTERVAL: Duration = Duration::from_millis(16);
const SLOTS: usize = 4;

/// Quantas telas assinam cada slot.
#[derive(Default)]
pub struct LiveInput {
    counts: Mutex<[u32; SLOTS]>,
    wake: Notify,
}

impl LiveInput {
    fn counts(&self) -> std::sync::MutexGuard<'_, [u32; SLOTS]> {
        self.counts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Alguém assina o `slot` (0–3). Devolve `false` para um slot inválido.
    pub fn subscribe(&self, slot: u8) -> bool {
        let Some(count) = self.counts().get_mut(usize::from(slot)).map(|c| {
            *c = c.saturating_add(1);
            *c
        }) else {
            return false;
        };
        if count == 1 {
            self.wake.notify_one();
        }
        true
    }

    /// Alguém deixa de assinar o `slot`. Cancelar sem ter assinado não faz nada.
    pub fn unsubscribe(&self, slot: u8) -> bool {
        self.counts()
            .get_mut(usize::from(slot))
            .map(|c| *c = c.saturating_sub(1))
            .is_some()
    }

    /// Zera todas as assinaturas: uma janela recarregada perdeu as suas e o backend ainda as
    /// conta.
    pub fn reset(&self) {
        *self.counts() = [0; SLOTS];
    }

    /// Alguma tela está assinando?
    pub fn is_active(&self) -> bool {
        self.counts().iter().any(|c| *c > 0)
    }

    fn subscribed(&self) -> [bool; SLOTS] {
        self.counts().map(|c| c > 0)
    }

    /// Uma passada: o estado dos slots assinados que mudou desde `last`. Slot sem assinatura não
    /// é lido nem lembrado.
    pub fn sample(
        &self,
        slots: &SlotManager,
        last: &mut [Option<XReport>; SLOTS],
    ) -> Vec<(u8, InputReportDto)> {
        let mut changed = Vec::new();
        for (index, subscribed) in self.subscribed().into_iter().enumerate() {
            if !subscribed {
                last[index] = None;
                continue;
            }
            let Some(slot) = u8::try_from(index).ok().and_then(SlotId::new) else {
                continue;
            };
            let current = slots.live_report(slot);
            if current != last[index] {
                last[index] = current;
                if let Some(report) = current {
                    changed.push((slot.index(), report.into()));
                }
            }
        }
        changed
    }

    /// A tarefa de amostragem: dorme sem assinantes e amostra a ~60 Hz com algum.
    pub async fn run(&self, slots: &SlotManager, mut emit: impl FnMut(u8, InputReportDto)) {
        let mut last = [None; SLOTS];
        loop {
            // Espera a primeira assinatura, sem perder um aviso que chegue entre a checagem e a
            // espera (o `Notify` guarda uma permissão).
            while !self.is_active() {
                self.wake.notified().await;
            }
            let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            while self.is_active() {
                ticker.tick().await;
                for (slot, report) in self.sample(slots, &mut last) {
                    emit(slot, report);
                }
            }
            last = [None; SLOTS];
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use padgaming_driver::MockDriver;
    use padgaming_protocol::control::Uuid;

    use super::*;

    fn manager() -> SlotManager {
        SlotManager::new(Arc::new(MockDriver::default()))
    }

    fn plug(slots: &SlotManager, report: XReport) {
        let device = Uuid::parse("00000000-0000-4000-8000-000000000001").unwrap();
        slots.assign(&device, 7, None).unwrap();
        slots.submit(7, &report).unwrap();
    }

    fn pressed() -> XReport {
        XReport {
            buttons: 1,
            lx: 1000,
            ..XReport::default()
        }
    }

    #[test]
    fn without_subscribers_nothing_is_sampled_or_emitted() {
        let live = LiveInput::default();
        let slots = manager();
        plug(&slots, pressed());
        let mut last = [None; SLOTS];
        assert!(!live.is_active());
        assert!(live.sample(&slots, &mut last).is_empty());
        assert_eq!(last, [None; SLOTS], "nem lembra o estado");
    }

    #[test]
    fn a_subscription_emits_the_state_once_and_then_only_changes() {
        let live = LiveInput::default();
        let slots = manager();
        plug(&slots, pressed());
        assert!(live.subscribe(0));
        let mut last = [None; SLOTS];

        let first = live.sample(&slots, &mut last);
        assert_eq!(first, vec![(0, pressed().into())]);
        // Nada mudou: nada a emitir.
        assert!(live.sample(&slots, &mut last).is_empty());
        // Mudou.
        slots
            .submit(
                7,
                &XReport {
                    lx: -5,
                    ..pressed()
                },
            )
            .unwrap();
        let next = live.sample(&slots, &mut last);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].1.lx, -5);
    }

    #[test]
    fn only_subscribed_slots_are_read_and_unsubscribing_stops_everything() {
        let live = LiveInput::default();
        let slots = manager();
        plug(&slots, pressed());
        let mut last = [None; SLOTS];
        assert!(live.subscribe(2), "assinar um slot vazio é permitido");
        assert!(
            live.sample(&slots, &mut last).is_empty(),
            "slot 0 não foi pedido"
        );

        live.subscribe(0);
        live.subscribe(0);
        assert_eq!(live.sample(&slots, &mut last).len(), 1);
        // Duas telas assinam; uma sai e o slot continua ativo.
        live.unsubscribe(0);
        assert!(live.is_active());
        live.unsubscribe(0);
        live.unsubscribe(2);
        assert!(!live.is_active());
        assert!(live.sample(&slots, &mut last).is_empty());
        // Cancelar demais não deixa a contagem negativa.
        live.unsubscribe(0);
        assert!(!live.is_active());
        live.subscribe(0);
        assert!(live.is_active());
    }

    #[test]
    fn invalid_slots_are_refused() {
        let live = LiveInput::default();
        assert!(!live.subscribe(4));
        assert!(!live.unsubscribe(9));
        assert!(!live.is_active());
    }

    #[tokio::test(start_paused = true)]
    async fn the_task_sleeps_without_subscribers_and_wakes_up_with_one() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let live = Arc::new(LiveInput::default());
        let slots = Arc::new(manager());
        plug(&slots, pressed());
        let emitted = Arc::new(AtomicU32::new(0));

        let task = tokio::spawn({
            let (live, slots, emitted) =
                (Arc::clone(&live), Arc::clone(&slots), Arc::clone(&emitted));
            async move {
                live.run(&slots, move |_, _| {
                    emitted.fetch_add(1, Ordering::Relaxed);
                })
                .await;
            }
        });

        // Segundos passam sem nenhuma assinatura: nada é emitido.
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert_eq!(emitted.load(Ordering::Relaxed), 0);

        live.subscribe(0);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            emitted.load(Ordering::Relaxed),
            1,
            "o estado inicial, uma vez"
        );

        // Cancelou: volta a dormir, e assinar de novo emite o estado de novo.
        live.unsubscribe(0);
        tokio::time::sleep(Duration::from_millis(100)).await;
        live.subscribe(0);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(emitted.load(Ordering::Relaxed), 2);
        task.abort();
    }
}
