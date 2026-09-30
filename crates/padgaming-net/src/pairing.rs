//! Pareamento e autenticação de dispositivos (PROTOCOLO §4.3).
//!
//! [`PairingService`] é a interface que o servidor de controle (T-302) usa; ela não conhece
//! socket nem TLS, então cada caminho de `auth` é testável aqui.
//! [`PairingManager`] é a implementação: guarda em memória os tokens de pareamento e o PIN,
//! aplica o rate limit e delega a persistência dos dispositivos à trait [`DeviceStore`]
//! (implementada sobre o SQLite na T-403).
//!
//! Regras:
//! - `pairing_token`: 128 bits, uso único, TTL de 5 min. `pin`: 6 dígitos, uso único, TTL de 2 min.
//! - PIN errado 5 vezes bloqueia por 60 s. A contagem é feita **por IP e global** (o §4.3 do
//!   protocolo pede a global; a por IP é a defesa em profundidade da T-103). Durante o
//!   bloqueio até o PIN correto recebe `locked`.
//! - `device_token`: 256 bits; só o SHA-256 é persistido. Comparações em tempo constante.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use padgaming_protocol::Secret;
use padgaming_protocol::control::{Auth, Bytes, Hello, Reject, RejectReason, Uuid};
use sha2::{Digest, Sha256};
use subtle::{Choice, ConstantTimeEq};

use crate::random_bytes;

/// Limite de tokens de pareamento simultâneos (evita crescer sem fim se a UI gerar QRs em laço).
const MAX_ACTIVE_TOKENS: usize = 8;

/// Parâmetros de tempo e de rate limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingConfig {
    /// Validade do token do QR.
    pub token_ttl: Duration,
    /// Validade do PIN.
    pub pin_ttl: Duration,
    /// PINs errados tolerados antes do bloqueio.
    pub max_pin_failures: u32,
    /// Duração do bloqueio.
    pub lockout: Duration,
}

impl Default for PairingConfig {
    fn default() -> Self {
        Self {
            token_ttl: Duration::from_mins(5),
            pin_ttl: Duration::from_mins(2),
            max_pin_failures: 5,
            lockout: Duration::from_secs(60),
        }
    }
}

/// Relógio injetável (nos testes avança sem `sleep`).
pub trait Clock: Send + Sync {
    /// Instante atual (monotônico).
    fn now(&self) -> Instant;
}

/// Relógio do sistema.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Falha da camada de persistência.
#[derive(Debug, thiserror::Error)]
#[error("falha no armazenamento de dispositivos: {0}")]
pub struct StoreError(pub String);

/// Dispositivo guardado. O token em si nunca é persistido, só o hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRecord {
    /// Identificador estável do aparelho.
    pub device_id: Uuid,
    /// Nome exibido.
    pub name: String,
    /// Modelo do aparelho.
    pub model: String,
    /// Versão do SO do aparelho.
    pub os_version: String,
    /// SHA-256 do `device_token`.
    pub token_hash: [u8; 32],
    /// O usuário revogou o dispositivo.
    pub revoked: bool,
}

/// Persistência dos dispositivos pareados; implementada sobre o SQLite na T-403.
///
/// É síncrona de propósito: as consultas são pontuais e a implementação pode manter um cache
/// em memória. Quem chamar de código async e usar I/O bloqueante deve isolar com `spawn_blocking`.
pub trait DeviceStore: Send + Sync {
    /// Busca o dispositivo pelo `device_id`.
    fn find(&self, device_id: &Uuid) -> Result<Option<DeviceRecord>, StoreError>;

    /// Grava (ou substitui) o dispositivo. Parear de novo um `device_id` já conhecido troca o
    /// token e remove a revogação: o usuário autorizou explicitamente com um novo QR/PIN.
    fn save(&self, record: DeviceRecord) -> Result<(), StoreError>;
}

/// Como o dispositivo se autenticou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// Primeiro pareamento (token do QR ou PIN).
    Pairing,
    /// Reconexão com o `device_token`.
    DeviceToken,
}

/// Resultado de uma autenticação bem-sucedida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authenticated {
    /// Dispositivo autenticado.
    pub device_id: Uuid,
    /// Método usado.
    pub method: AuthMethod,
    /// Token recém-emitido; só no primeiro pareamento (vai no `welcome`).
    pub device_token: Option<Secret<Bytes<32>>>,
}

/// Motivo da falha de autenticação.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// O client deve receber esse `reject` e a conexão fecha.
    #[error("autenticação recusada: {:?}", .0.reason)]
    Rejected(Reject),
    /// Erro interno (armazenamento). O server deve logar e fechar sem detalhar ao client.
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl AuthError {
    fn reject(reason: RejectReason) -> Self {
        Self::Rejected(Reject {
            reason,
            retry_after_s: None,
            supported_versions: None,
            message: None,
        })
    }
}

/// Autentica o `hello` de um client.
pub trait PairingService: Send + Sync {
    /// Valida `hello.auth` para o client em `peer`.
    ///
    /// Num primeiro pareamento, depois de a credencial ser aceita e **antes** de gravar o
    /// dispositivo, `approve` decide se ele entra (a confirmação "Pedir confirmação para novos
    /// dispositivos", T-302). Se recusar, nada é gravado e o resultado é `bad_auth`. Não é
    /// chamado na reconexão com `device_token`. `approve` pode bloquear (a espera é de até 30 s).
    fn authenticate_with(
        &self,
        hello: &Hello,
        peer: IpAddr,
        approve: &dyn Fn(&Hello) -> bool,
    ) -> Result<Authenticated, AuthError>;

    /// Como [`Self::authenticate_with`], aprovando todo novo dispositivo.
    fn authenticate(&self, hello: &Hello, peer: IpAddr) -> Result<Authenticated, AuthError> {
        self.authenticate_with(hello, peer, &|_| true)
    }
}

struct Expiring<T> {
    value: T,
    expires: Instant,
}

#[derive(Default)]
struct FailureCounter {
    count: u32,
    locked_until: Option<Instant>,
}

#[derive(Default)]
struct State {
    tokens: Vec<Expiring<[u8; 16]>>,
    pin: Option<Expiring<[u8; 6]>>,
    global: FailureCounter,
    per_ip: HashMap<IpAddr, FailureCounter>,
}

/// Implementação de [`PairingService`] com estado em memória.
pub struct PairingManager<S, C = SystemClock> {
    store: S,
    clock: C,
    config: PairingConfig,
    state: Mutex<State>,
}

impl<S: DeviceStore> PairingManager<S> {
    /// Cria o gerenciador com o relógio do sistema.
    pub fn new(store: S, config: PairingConfig) -> Self {
        Self::with_clock(store, config, SystemClock)
    }
}

impl<S: DeviceStore, C: Clock> PairingManager<S, C> {
    /// Cria o gerenciador com um relógio específico.
    pub fn with_clock(store: S, config: PairingConfig, clock: C) -> Self {
        Self {
            store,
            clock,
            config,
            state: Mutex::default(),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // Nenhuma seção crítica deixa o estado inconsistente ao entrar em pânico.
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Emite um token de pareamento para o QR code (128 bits, uso único, TTL de 5 min).
    pub fn issue_pairing_token(&self) -> Secret<[u8; 16]> {
        let now = self.clock.now();
        let token = random_bytes::<16>();
        let mut state = self.state();
        state.tokens.retain(|t| t.expires > now);
        if state.tokens.len() >= MAX_ACTIVE_TOKENS {
            state.tokens.remove(0);
        }
        state.tokens.push(Expiring {
            value: token,
            expires: now + self.config.token_ttl,
        });
        Secret::new(token)
    }

    /// Emite um PIN de 6 dígitos (TTL de 2 min), substituindo o anterior.
    pub fn issue_pin(&self) -> Secret<String> {
        let now = self.clock.now();
        let digits = random_digits();
        self.state().pin = Some(Expiring {
            value: digits,
            expires: now + self.config.pin_ttl,
        });
        Secret::new(digits.iter().map(|d| char::from(b'0' + d)).collect())
    }

    /// Cancela o pareamento em andamento: descarta todos os tokens e o PIN.
    pub fn cancel_pairing(&self) {
        let mut state = self.state();
        state.tokens.clear();
        state.pin = None;
    }

    /// Tempo restante do bloqueio para `peer`, se houver.
    fn lock_remaining(state: &State, peer: IpAddr, now: Instant) -> Option<Duration> {
        [Some(&state.global), state.per_ip.get(&peer)]
            .into_iter()
            .flatten()
            .filter_map(|c| c.locked_until)
            .filter(|until| *until > now)
            .map(|until| until - now)
            .max()
    }

    fn locked(remaining: Duration) -> AuthError {
        // Arredonda para cima: nunca manda o client tentar antes da hora.
        let secs = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
        AuthError::Rejected(Reject {
            reason: RejectReason::Locked,
            retry_after_s: Some(u32::try_from(secs).unwrap_or(u32::MAX)),
            supported_versions: None,
            message: None,
        })
    }

    fn register_pin_failure(&self, state: &mut State, peer: IpAddr, now: Instant) {
        let (max, lockout) = (self.config.max_pin_failures, self.config.lockout);
        let bump = |counter: &mut FailureCounter| {
            // Um bloqueio já vencido zera a contagem.
            if counter.locked_until.is_some_and(|until| until <= now) {
                *counter = FailureCounter::default();
            }
            counter.count += 1;
            if counter.count >= max {
                counter.locked_until = Some(now + lockout);
                true
            } else {
                false
            }
        };
        let global_locked = bump(&mut state.global);
        let ip_locked = bump(state.per_ip.entry(peer).or_default());
        if global_locked || ip_locked {
            tracing::warn!(%peer, global = global_locked, "PIN errado demais: pareamento por PIN bloqueado");
        }
    }

    fn authenticate_pin(&self, pin: &str, peer: IpAddr) -> Result<(), AuthError> {
        let now = self.clock.now();
        let mut state = self.state();
        if let Some(remaining) = Self::lock_remaining(&state, peer, now) {
            return Err(Self::locked(remaining));
        }
        // Limpa entradas vencidas para o mapa não crescer.
        state
            .per_ip
            .retain(|_, c| c.count > 0 && c.locked_until.is_none_or(|until| until > now));

        let matched = match state.pin.as_ref().filter(|p| p.expires > now) {
            Some(active) => {
                let given = pin.as_bytes();
                // O tamanho do PIN não é segredo (sempre 6); o conteúdo é comparado sem atalho.
                given.len() == active.value.len()
                    && bool::from(digits_as_ascii(active.value).ct_eq(given))
            }
            None => false,
        };
        if matched {
            state.pin = None;
            state.global = FailureCounter::default();
            state.per_ip.remove(&peer);
            return Ok(());
        }
        self.register_pin_failure(&mut state, peer, now);
        // A 5ª falha já bloqueia: o client recebe `locked` em vez de `bad_auth`.
        match Self::lock_remaining(&state, peer, now) {
            Some(remaining) => Err(Self::locked(remaining)),
            None => Err(AuthError::reject(RejectReason::BadAuth)),
        }
    }

    fn authenticate_pairing_token(&self, token: &[u8; 16]) -> Result<(), AuthError> {
        let now = self.clock.now();
        let mut state = self.state();
        state.tokens.retain(|t| t.expires > now);
        // Percorre todos os tokens, sem parar no primeiro que casar.
        let mut found = None;
        for (i, t) in state.tokens.iter().enumerate() {
            if bool::from(t.value.ct_eq(token)) {
                found = Some(i);
            }
        }
        match found {
            Some(i) => {
                state.tokens.swap_remove(i);
                Ok(())
            }
            None => Err(AuthError::reject(RejectReason::BadAuth)),
        }
    }

    fn authenticate_device_token(
        &self,
        device_id: &Uuid,
        token: &[u8; 32],
    ) -> Result<Authenticated, AuthError> {
        let given: [u8; 32] = Sha256::digest(token).into();
        let Some(record) = self.store.find(device_id)? else {
            return Err(AuthError::reject(RejectReason::BadAuth));
        };
        let matches: Choice = record.token_hash.ct_eq(&given);
        if !bool::from(matches) {
            return Err(AuthError::reject(RejectReason::BadAuth));
        }
        // A revogação só é revelada a quem prova conhecer o token.
        if record.revoked {
            return Err(AuthError::reject(RejectReason::Revoked));
        }
        Ok(Authenticated {
            device_id: record.device_id,
            method: AuthMethod::DeviceToken,
            device_token: None,
        })
    }

    fn enroll(
        &self,
        hello: &Hello,
        approve: &dyn Fn(&Hello) -> bool,
    ) -> Result<Authenticated, AuthError> {
        if !approve(hello) {
            return Err(AuthError::Rejected(Reject {
                reason: RejectReason::BadAuth,
                retry_after_s: None,
                supported_versions: None,
                message: Some("pareamento recusado no computador".into()),
            }));
        }
        let token = random_bytes::<32>();
        self.store.save(DeviceRecord {
            device_id: hello.device_id.clone(),
            name: hello.name.clone(),
            model: hello.model.clone(),
            os_version: hello.os_ver.clone(),
            token_hash: Sha256::digest(token).into(),
            revoked: false,
        })?;
        Ok(Authenticated {
            device_id: hello.device_id.clone(),
            method: AuthMethod::Pairing,
            device_token: Some(Secret::new(Bytes(token))),
        })
    }
}

impl<S: DeviceStore, C: Clock> PairingService for PairingManager<S, C> {
    fn authenticate_with(
        &self,
        hello: &Hello,
        peer: IpAddr,
        approve: &dyn Fn(&Hello) -> bool,
    ) -> Result<Authenticated, AuthError> {
        match &hello.auth {
            Auth::DeviceToken { token } => {
                self.authenticate_device_token(&hello.device_id, &token.expose().0)
            }
            Auth::PairingToken { token } => {
                self.authenticate_pairing_token(&token.expose().0)?;
                self.enroll(hello, approve)
            }
            Auth::Pin { pin } => {
                self.authenticate_pin(pin.expose(), peer)?;
                self.enroll(hello, approve)
            }
        }
    }
}

/// Seis dígitos uniformes (0–9), por rejeição para não ter viés de módulo.
fn random_digits() -> [u8; 6] {
    let mut digits = [0u8; 6];
    let mut filled = 0;
    while filled < digits.len() {
        for byte in random_bytes::<16>() {
            // 250 = maior múltiplo de 10 que cabe em um byte.
            if byte < 250 && filled < digits.len() {
                digits[filled] = byte % 10;
                filled += 1;
            }
        }
    }
    digits
}

fn digits_as_ascii(digits: [u8; 6]) -> [u8; 6] {
    digits.map(|d| b'0' + d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestClock {
        base: Instant,
        offset: Arc<Mutex<Duration>>,
    }

    impl TestClock {
        fn new() -> Self {
            Self {
                base: Instant::now(),
                offset: Arc::default(),
            }
        }
        fn advance(&self, by: Duration) {
            *self.offset.lock().unwrap() += by;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            self.base + *self.offset.lock().unwrap()
        }
    }

    #[derive(Clone, Default)]
    struct MemStore(Arc<Mutex<HashMap<String, DeviceRecord>>>);

    impl DeviceStore for MemStore {
        fn find(&self, id: &Uuid) -> Result<Option<DeviceRecord>, StoreError> {
            Ok(self.0.lock().unwrap().get(id.as_str()).cloned())
        }
        fn save(&self, r: DeviceRecord) -> Result<(), StoreError> {
            self.0
                .lock()
                .unwrap()
                .insert(r.device_id.as_str().to_owned(), r);
            Ok(())
        }
    }

    struct FailingStore;

    impl DeviceStore for FailingStore {
        fn find(&self, _: &Uuid) -> Result<Option<DeviceRecord>, StoreError> {
            Err(StoreError("disco cheio".into()))
        }
        fn save(&self, _: DeviceRecord) -> Result<(), StoreError> {
            Err(StoreError("disco cheio".into()))
        }
    }

    const DEVICE: &str = "123e4567-e89b-42d3-a456-426614174000";
    const OTHER_DEVICE: &str = "223e4567-e89b-42d3-a456-426614174000";

    fn peer(last: u8) -> IpAddr {
        IpAddr::from([192, 168, 0, last])
    }

    fn hello_for(device: &str, auth: Auth) -> Hello {
        serde_json::from_value(serde_json::json!({
            "proto_ver": 1,
            "device_id": device,
            "name": "Pixel",
            "model": "Pixel 8",
            "os_ver": "15",
            "caps": { "gyro": true, "rumble": true },
            "auth": serde_json::to_value(auth).unwrap(),
        }))
        .unwrap()
    }

    fn by_token(token: [u8; 16]) -> Auth {
        Auth::PairingToken {
            token: Secret::new(Bytes(token)),
        }
    }

    fn by_pin(pin: &str) -> Auth {
        Auth::Pin {
            pin: Secret::new(pin.to_owned()),
        }
    }

    fn by_device_token(token: [u8; 32]) -> Auth {
        Auth::DeviceToken {
            token: Secret::new(Bytes(token)),
        }
    }

    fn setup() -> (PairingManager<MemStore, TestClock>, MemStore, TestClock) {
        let (store, clock) = (MemStore::default(), TestClock::new());
        let manager =
            PairingManager::with_clock(store.clone(), PairingConfig::default(), clock.clone());
        (manager, store, clock)
    }

    fn reason(err: AuthError) -> RejectReason {
        match err {
            AuthError::Rejected(r) => r.reason,
            AuthError::Store(e) => panic!("erro de armazenamento inesperado: {e}"),
        }
    }

    fn wrong_pin(good: &str) -> &'static str {
        if good == "000000" { "111111" } else { "000000" }
    }

    // ------------------------------------------------------------ token do QR

    #[test]
    fn pairing_token_enrolls_device_and_returns_device_token() {
        let (manager, store, _) = setup();
        let token = manager.issue_pairing_token();
        let hello = hello_for(DEVICE, by_token(*token.expose()));
        let ok = manager.authenticate(&hello, peer(2)).unwrap();
        assert_eq!(ok.method, AuthMethod::Pairing);
        let device_token = ok.device_token.expect("token no primeiro pareamento");

        let saved = store.find(&hello.device_id).unwrap().unwrap();
        assert_eq!(saved.name, "Pixel");
        assert!(!saved.revoked);
        assert_eq!(
            saved.token_hash,
            <[u8; 32]>::from(Sha256::digest(device_token.expose().0))
        );
    }

    #[test]
    fn approval_denied_does_not_enroll_and_skips_reconnection() {
        let (manager, store, _) = setup();
        let token = manager.issue_pairing_token();
        let hello = hello_for(DEVICE, by_token(*token.expose()));
        let err = manager
            .authenticate_with(&hello, peer(2), &|_| false)
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
        assert!(store.find(&hello.device_id).unwrap().is_none());

        // A reconexão com device_token não passa pelo gate.
        let token = manager.issue_pairing_token();
        let hello = hello_for(DEVICE, by_token(*token.expose()));
        let paired = manager.authenticate(&hello, peer(2)).unwrap();
        let again = hello_for(
            DEVICE,
            Auth::DeviceToken {
                token: Secret::new(*paired.device_token.unwrap().expose()),
            },
        );
        assert!(
            manager
                .authenticate_with(&again, peer(2), &|_| false)
                .is_ok()
        );
    }

    #[test]
    fn pairing_token_is_single_use() {
        let (manager, _, _) = setup();
        let token = *manager.issue_pairing_token().expose();
        let hello = hello_for(DEVICE, by_token(token));
        manager.authenticate(&hello, peer(2)).unwrap();
        let err = manager.authenticate(&hello, peer(2)).unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn pairing_token_expires_after_five_minutes() {
        let (manager, _, clock) = setup();
        let token = *manager.issue_pairing_token().expose();
        clock.advance(Duration::from_secs(299));
        // Ainda válido um segundo antes de expirar (sem consumir: outro token para o teste).
        let still = *manager.issue_pairing_token().expose();
        assert!(
            manager
                .authenticate(&hello_for(DEVICE, by_token(still)), peer(2))
                .is_ok()
        );
        clock.advance(Duration::from_secs(1));
        let err = manager
            .authenticate(&hello_for(OTHER_DEVICE, by_token(token)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn unknown_pairing_token_is_rejected_and_does_not_enroll() {
        let (manager, store, _) = setup();
        manager.issue_pairing_token();
        let hello = hello_for(DEVICE, by_token([0xAB; 16]));
        let err = manager.authenticate(&hello, peer(2)).unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
        assert!(store.find(&hello.device_id).unwrap().is_none());
    }

    #[test]
    fn several_pairing_tokens_can_be_active_and_cancel_clears_them() {
        let (manager, _, _) = setup();
        let a = *manager.issue_pairing_token().expose();
        let b = *manager.issue_pairing_token().expose();
        assert!(
            manager
                .authenticate(&hello_for(DEVICE, by_token(a)), peer(2))
                .is_ok()
        );
        manager.cancel_pairing();
        let err = manager
            .authenticate(&hello_for(OTHER_DEVICE, by_token(b)), peer(3))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn active_tokens_are_capped() {
        let (manager, _, _) = setup();
        let first = *manager.issue_pairing_token().expose();
        for _ in 0..MAX_ACTIVE_TOKENS {
            manager.issue_pairing_token();
        }
        let err = manager
            .authenticate(&hello_for(DEVICE, by_token(first)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    // ------------------------------------------------------------ PIN

    #[test]
    fn pin_has_six_digits_and_enrolls() {
        let (manager, _, _) = setup();
        let pin = manager.issue_pin();
        assert_eq!(pin.expose().len(), 6);
        assert!(pin.expose().bytes().all(|b| b.is_ascii_digit()));
        let ok = manager
            .authenticate(&hello_for(DEVICE, by_pin(pin.expose())), peer(2))
            .unwrap();
        assert!(ok.device_token.is_some());
    }

    #[test]
    fn pin_is_single_use() {
        let (manager, _, _) = setup();
        let pin = manager.issue_pin();
        let hello = hello_for(DEVICE, by_pin(pin.expose()));
        manager.authenticate(&hello, peer(2)).unwrap();
        let err = manager.authenticate(&hello, peer(2)).unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn pin_expires_after_two_minutes() {
        let (manager, _, clock) = setup();
        let pin = manager.issue_pin();
        clock.advance(Duration::from_mins(2));
        let err = manager
            .authenticate(&hello_for(DEVICE, by_pin(pin.expose())), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn pin_without_active_pin_is_rejected() {
        let (manager, _, _) = setup();
        let err = manager
            .authenticate(&hello_for(DEVICE, by_pin("123456")), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn malformed_pin_lengths_are_rejected() {
        let (manager, _, _) = setup();
        let pin = manager.issue_pin();
        for bad in ["", "12345", "1234567", &format!("{} ", pin.expose())] {
            let err = manager
                .authenticate(&hello_for(DEVICE, by_pin(bad)), peer(2))
                .unwrap_err();
            assert!(matches!(
                reason(err),
                RejectReason::BadAuth | RejectReason::Locked
            ));
        }
    }

    #[test]
    fn sixth_wrong_pin_attempt_is_locked_and_correct_pin_stays_locked() {
        let (manager, _, clock) = setup();
        let pin = manager.issue_pin();
        let wrong = wrong_pin(pin.expose());
        let hello_wrong = hello_for(DEVICE, by_pin(wrong));

        for attempt in 1..=4 {
            let err = manager.authenticate(&hello_wrong, peer(2)).unwrap_err();
            assert_eq!(reason(err), RejectReason::BadAuth, "tentativa {attempt}");
        }
        // 5ª falha: já responde locked com o tempo de espera.
        let AuthError::Rejected(r) = manager.authenticate(&hello_wrong, peer(2)).unwrap_err()
        else {
            panic!("esperava reject");
        };
        assert_eq!(r.reason, RejectReason::Locked);
        assert_eq!(r.retry_after_s, Some(60));

        // 6ª tentativa, até com o PIN certo, segue bloqueada.
        let hello_right = hello_for(DEVICE, by_pin(pin.expose()));
        let err = manager.authenticate(&hello_right, peer(2)).unwrap_err();
        assert_eq!(reason(err), RejectReason::Locked);

        // O tempo restante diminui e, passados 60 s, o PIN certo funciona.
        clock.advance(Duration::from_secs(30));
        let AuthError::Rejected(r) = manager.authenticate(&hello_right, peer(2)).unwrap_err()
        else {
            panic!("esperava reject");
        };
        assert_eq!(r.retry_after_s, Some(30));
        clock.advance(Duration::from_secs(30));
        assert!(manager.authenticate(&hello_right, peer(2)).is_ok());
    }

    #[test]
    fn lockout_is_global_across_ips() {
        let (manager, _, _) = setup();
        let pin = manager.issue_pin();
        let hello_wrong = hello_for(DEVICE, by_pin(wrong_pin(pin.expose())));
        // 5 falhas espalhadas por IPs diferentes ainda bloqueiam todo mundo (§4.3).
        for last in 10..15 {
            let _ = manager.authenticate(&hello_wrong, peer(last));
        }
        let err = manager
            .authenticate(&hello_for(DEVICE, by_pin(pin.expose())), peer(99))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::Locked);
    }

    #[test]
    fn successful_pin_resets_the_failure_count() {
        let (manager, _, _) = setup();
        for _ in 0..3 {
            let pin = manager.issue_pin();
            let wrong = hello_for(DEVICE, by_pin(wrong_pin(pin.expose())));
            for _ in 0..4 {
                let err = manager.authenticate(&wrong, peer(2)).unwrap_err();
                assert_eq!(reason(err), RejectReason::BadAuth);
            }
            assert!(
                manager
                    .authenticate(&hello_for(DEVICE, by_pin(pin.expose())), peer(2))
                    .is_ok()
            );
        }
    }

    #[test]
    fn failures_after_an_expired_lockout_start_from_zero() {
        let (manager, _, clock) = setup();
        let pin = manager.issue_pin();
        let wrong = hello_for(DEVICE, by_pin(wrong_pin(pin.expose())));
        for _ in 0..5 {
            let _ = manager.authenticate(&wrong, peer(2));
        }
        clock.advance(Duration::from_secs(61));
        let err = manager.authenticate(&wrong, peer(2)).unwrap_err();
        assert_eq!(
            reason(err),
            RejectReason::BadAuth,
            "1ª falha após o bloqueio"
        );
    }

    #[test]
    fn locked_pin_does_not_block_paired_devices() {
        let (manager, _, _) = setup();
        let token = *manager.issue_pairing_token().expose();
        let ok = manager
            .authenticate(&hello_for(DEVICE, by_token(token)), peer(2))
            .unwrap();
        let device_token = ok.device_token.unwrap();

        let pin = manager.issue_pin();
        let wrong = hello_for(OTHER_DEVICE, by_pin(wrong_pin(pin.expose())));
        for _ in 0..5 {
            let _ = manager.authenticate(&wrong, peer(3));
        }
        let hello = hello_for(DEVICE, by_device_token(device_token.expose().0));
        assert!(manager.authenticate(&hello, peer(2)).is_ok());
    }

    // ------------------------------------------------------------ device token

    fn enroll(manager: &PairingManager<MemStore, TestClock>) -> [u8; 32] {
        let token = *manager.issue_pairing_token().expose();
        let ok = manager
            .authenticate(&hello_for(DEVICE, by_token(token)), peer(2))
            .unwrap();
        ok.device_token.unwrap().expose().0
    }

    #[test]
    fn device_token_reconnects_without_issuing_another() {
        let (manager, _, _) = setup();
        let token = enroll(&manager);
        let ok = manager
            .authenticate(&hello_for(DEVICE, by_device_token(token)), peer(2))
            .unwrap();
        assert_eq!(ok.method, AuthMethod::DeviceToken);
        assert!(ok.device_token.is_none());
    }

    #[test]
    fn wrong_device_token_or_unknown_device_is_bad_auth() {
        let (manager, _, _) = setup();
        let mut token = enroll(&manager);
        token[0] ^= 1;
        let err = manager
            .authenticate(&hello_for(DEVICE, by_device_token(token)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
        let err = manager
            .authenticate(&hello_for(OTHER_DEVICE, by_device_token(token)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn token_of_another_device_is_not_accepted() {
        let (manager, _, _) = setup();
        let token = enroll(&manager);
        let err = manager
            .authenticate(&hello_for(OTHER_DEVICE, by_device_token(token)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn revoked_device_gets_revoked_only_with_the_right_token() {
        let (manager, store, _) = setup();
        let token = enroll(&manager);
        let id = Uuid::parse(DEVICE).unwrap();
        let mut record = store.find(&id).unwrap().unwrap();
        record.revoked = true;
        store.save(record).unwrap();

        let err = manager
            .authenticate(&hello_for(DEVICE, by_device_token(token)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::Revoked);

        // Com token errado, nada é revelado sobre a revogação.
        let err = manager
            .authenticate(&hello_for(DEVICE, by_device_token([1; 32])), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn pairing_again_replaces_the_token_and_clears_revocation() {
        let (manager, store, _) = setup();
        let old = enroll(&manager);
        let id = Uuid::parse(DEVICE).unwrap();
        let mut record = store.find(&id).unwrap().unwrap();
        record.revoked = true;
        store.save(record).unwrap();

        let new = enroll(&manager);
        assert_ne!(old, new);
        let ok = manager.authenticate(&hello_for(DEVICE, by_device_token(new)), peer(2));
        assert!(ok.is_ok());
        let err = manager
            .authenticate(&hello_for(DEVICE, by_device_token(old)), peer(2))
            .unwrap_err();
        assert_eq!(reason(err), RejectReason::BadAuth);
    }

    #[test]
    fn store_never_sees_the_plain_device_token() {
        let (manager, store, _) = setup();
        let token = enroll(&manager);
        let saved = store.find(&Uuid::parse(DEVICE).unwrap()).unwrap().unwrap();
        assert_ne!(saved.token_hash, token);
        assert!(!format!("{saved:?}").contains(&format!("{:?}", token.to_vec())));
    }

    // ------------------------------------------------------------ erros de armazenamento

    #[test]
    fn store_failures_are_internal_errors_not_rejects() {
        let manager = PairingManager::new(FailingStore, PairingConfig::default());
        let err = manager
            .authenticate(&hello_for(DEVICE, by_device_token([1; 32])), peer(2))
            .unwrap_err();
        assert!(matches!(err, AuthError::Store(_)));

        let token = *manager.issue_pairing_token().expose();
        let err = manager
            .authenticate(&hello_for(DEVICE, by_token(token)), peer(2))
            .unwrap_err();
        assert!(matches!(err, AuthError::Store(_)));
    }

    #[test]
    fn random_digits_cover_the_whole_range() {
        let mut seen = [false; 10];
        for _ in 0..200 {
            for d in random_digits() {
                assert!(d < 10);
                seen[usize::from(d)] = true;
            }
        }
        assert!(seen.iter().all(|s| *s));
    }
}
