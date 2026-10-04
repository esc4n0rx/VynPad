//! Watchdog das sessões (PROTOCOLO §9, T-304): nunca deixa um botão preso no jogo.
//!
//! A cada `watchdog_interval` (50 ms) confere há quanto tempo cada sessão não manda pacote UDP
//! válido:
//! - `neutral_after` (250 ms): solta o controle virtual, que continua plugado;
//! - `stalled_after` (5 s): a UI passa a mostrar "Reconectando…".
//!
//! E, a cada `reservation_check` (1 s), libera as reservas de slot vencidas: 30 s depois da
//! queda do TLS o controle é desplugado.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::{MissedTickBehavior, interval};

use super::server::Shared;
use super::{ControlConfig, ControlEvent, SessionHealth};

/// Estado do fluxo de input para um silêncio de `idle`.
pub(super) fn classify(idle: Duration, config: &ControlConfig) -> SessionHealth {
    if idle >= config.stalled_after {
        SessionHealth::Stalled
    } else if idle >= config.neutral_after {
        SessionHealth::Neutral
    } else {
        SessionHealth::Live
    }
}

pub(super) async fn run(shared: Arc<Shared>) {
    let mut stop = shared.shutdown.subscribe();
    let mut ticker = interval(shared.config.watchdog_interval);
    // Se o processo engasgar, não tenta "recuperar" os ticks perdidos em rajada.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut last_reservation_check = Instant::now();
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = stop.changed() => break,
        }
        let now = Instant::now();
        poll_sessions(&shared, now);
        if now.duration_since(last_reservation_check) >= shared.config.reservation_check {
            last_reservation_check = now;
            for slot in shared.slots.tick() {
                tracing::info!(
                    slot = slot.index(),
                    "reserva do slot venceu: controle desplugado"
                );
                shared.emit(ControlEvent::ReservationExpired { slot });
            }
        }
    }
}

/// Uma passada sobre as sessões vivas, no instante `now`.
fn poll_sessions(shared: &Shared, now: Instant) {
    let mut changes = Vec::new();
    {
        let mut live = shared.live();
        for (id, session) in live.iter_mut() {
            let health = classify(session.activity.idle_for(now), &shared.config);
            if health != session.health {
                let before = std::mem::replace(&mut session.health, health);
                changes.push((*id, session.slot, before, health));
            }
        }
    }
    for (session_id, slot, before, health) in changes {
        // Só solta ao sair de `Live`; `Neutral` → `Stalled` não tem nada novo a soltar.
        if before == SessionHealth::Live && health != SessionHealth::Live {
            shared.slots.neutralize_session(session_id);
        }
        tracing::debug!(session_id, ?health, "estado do input da sessão mudou");
        shared.emit(ControlEvent::SessionState {
            session_id,
            slot,
            health,
        });
    }
}

/// O sistema retomou da suspensão: solta tudo e revalida o driver.
pub(super) fn resumed(shared: &Shared) {
    let sessions: Vec<u32> = shared.live().keys().copied().collect();
    for session_id in sessions {
        shared.slots.neutralize_session(session_id);
    }
    let status = shared.slots.driver_status();
    tracing::info!(
        ?status,
        "sistema retomou: controles soltos e driver revalidado"
    );
    shared.emit(ControlEvent::DriverRechecked(status));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_thresholds_follow_the_protocol_timeouts() {
        let config = ControlConfig::default();
        let ms = Duration::from_millis;
        assert_eq!(classify(ms(0), &config), SessionHealth::Live);
        assert_eq!(classify(ms(249), &config), SessionHealth::Live);
        assert_eq!(classify(ms(250), &config), SessionHealth::Neutral);
        assert_eq!(classify(ms(4_999), &config), SessionHealth::Neutral);
        assert_eq!(classify(ms(5_000), &config), SessionHealth::Stalled);
    }
}
