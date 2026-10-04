//! Configurações do app: a struct tipada [`AppSettings`], o repositório e o serviço que avisa
//! quando algo muda (`settings:changed`).

use std::sync::{Arc, PoisonError, RwLock};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::error::DataError;
use crate::seed::now_ms;

/// Idioma da interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Language {
    /// Português do Brasil.
    #[serde(rename = "pt-BR")]
    PtBr,
    /// Inglês.
    #[serde(rename = "en")]
    En,
}

/// Tema da interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Escuro.
    Dark,
    /// Claro.
    Light,
    /// Segue o sistema.
    System,
}

/// Portas do server (mudam com reinício do app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Ports {
    /// Canal de controle (TCP).
    pub control: u16,
    /// Input (UDP).
    pub input: u16,
    /// Descoberta por broadcast (UDP).
    pub discovery: u16,
}

/// As configurações do app. Cada campo é uma linha da tabela `settings`.
// São chaves independentes da tabela `settings`; agrupar os booleanos não deixaria o modelo
// mais claro.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct AppSettings {
    /// Idioma.
    pub language: Language,
    /// Tema.
    pub theme: Theme,
    /// Cor de destaque, `#RRGGBB`.
    pub accent: String,
    /// Portas.
    pub ports: Ports,
    /// Iniciar com o Windows.
    pub autostart: bool,
    /// Iniciar minimizado na bandeja.
    pub start_minimized: bool,
    /// O onboarding já foi concluído.
    pub onboarding_completed: bool,
    /// Pedir confirmação para novos dispositivos.
    pub require_approval: bool,
    /// Nome do server mostrado no celular; vazio usa o nome do computador. Vale na próxima
    /// vez que o app abrir.
    pub server_name: String,
    /// Fechar a janela deixa o app na bandeja (em vez de encerrar).
    pub close_to_tray: bool,
    /// Perfil que um dispositivo novo (sem perfil próprio) recebe; `None` usa o "Padrão".
    pub default_profile_id: Option<String>,
    /// Taxa máxima de envio sugerida aos clients, em Hz (125 ou 250).
    pub send_rate_hz: u16,
    /// Por quanto tempo o slot fica reservado depois da queda da conexão, em segundos.
    pub reservation_secs: u16,
    /// Efeito de vidro (Mica/Acrylic) na janela.
    pub transparency: bool,
    /// Reduz as animações da interface.
    pub reduce_motion: bool,
    /// O aviso "continua rodando na bandeja" já foi mostrado.
    pub tray_hint_shown: bool,
    /// Avisar quando um celular conecta, reconecta ou desconecta.
    pub notify_connections: bool,
    /// Avisar sobre pareamentos, pedidos de aprovação e conexões recusadas.
    pub notify_pairing: bool,
    /// Avisar quando o driver de controle virtual falha.
    pub notify_driver: bool,
    /// Não avisar enquanto um jogo ocupa a tela cheia.
    pub notify_skip_fullscreen: bool,
}

impl Default for AppSettings {
    /// Igual aos seeds de `seed::DEFAULT_SETTINGS`.
    fn default() -> Self {
        Self {
            language: Language::PtBr,
            theme: Theme::Dark,
            accent: "#0A84FF".to_owned(),
            ports: Ports {
                control: 47800,
                input: 47801,
                discovery: 47802,
            },
            autostart: false,
            start_minimized: false,
            onboarding_completed: false,
            require_approval: false,
            server_name: String::new(),
            close_to_tray: true,
            default_profile_id: None,
            send_rate_hz: 250,
            reservation_secs: 30,
            transparency: true,
            reduce_motion: false,
            tray_hint_shown: false,
            notify_connections: true,
            notify_pairing: true,
            notify_driver: true,
            notify_skip_fullscreen: true,
        }
    }
}

/// Taxas de envio que o app oferece (Hz).
pub const SEND_RATES_HZ: [u16; 2] = [125, 250];
/// Faixa do tempo de reserva do slot, em segundos.
pub const RESERVATION_SECS: std::ops::RangeInclusive<u16> = 10..=120;
/// Tamanho máximo do nome do server, em caracteres.
pub const SERVER_NAME_MAX: usize = 40;

/// Portas abaixo desta são reservadas ao sistema.
const MIN_PORT: u16 = 1024;

impl AppSettings {
    /// Confere as faixas. A mensagem é para mostrar ao usuário.
    pub fn validate(&self) -> Result<(), DataError> {
        let invalid = |why: &str| Err(DataError::Invalid(why.to_owned()));
        let hex = self.accent.strip_prefix('#').unwrap_or("");
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return invalid("a cor de destaque precisa estar no formato #RRGGBB");
        }
        let Ports {
            control,
            input,
            discovery,
        } = self.ports;
        if [control, input, discovery].iter().any(|p| *p < MIN_PORT) {
            return invalid("as portas precisam ser 1024 ou mais");
        }
        if control == input || control == discovery || input == discovery {
            return invalid("as três portas precisam ser diferentes");
        }
        if [control, input, discovery].contains(&5353) {
            return invalid("a porta 5353 é usada pelo mDNS");
        }
        if self.server_name.chars().count() > SERVER_NAME_MAX
            || self.server_name.chars().any(char::is_control)
        {
            return invalid("o nome do server tem até 40 caracteres, sem controles");
        }
        if !SEND_RATES_HZ.contains(&self.send_rate_hz) {
            return invalid("a taxa de envio precisa ser 125 ou 250 Hz");
        }
        if !RESERVATION_SECS.contains(&self.reservation_secs) {
            return invalid("o tempo de reserva do slot vai de 10 a 120 segundos");
        }
        Ok(())
    }

    /// Cada campo como `(chave, JSON)`.
    fn entries(&self) -> Result<[(&'static str, String); 20], serde_json::Error> {
        Ok([
            ("language", serde_json::to_string(&self.language)?),
            ("theme", serde_json::to_string(&self.theme)?),
            ("accent", serde_json::to_string(&self.accent)?),
            ("ports", serde_json::to_string(&self.ports)?),
            ("autostart", serde_json::to_string(&self.autostart)?),
            (
                "start_minimized",
                serde_json::to_string(&self.start_minimized)?,
            ),
            (
                "onboarding_completed",
                serde_json::to_string(&self.onboarding_completed)?,
            ),
            (
                "require_approval",
                serde_json::to_string(&self.require_approval)?,
            ),
            ("server_name", serde_json::to_string(&self.server_name)?),
            ("close_to_tray", serde_json::to_string(&self.close_to_tray)?),
            (
                "default_profile_id",
                serde_json::to_string(&self.default_profile_id)?,
            ),
            ("send_rate_hz", serde_json::to_string(&self.send_rate_hz)?),
            (
                "reservation_secs",
                serde_json::to_string(&self.reservation_secs)?,
            ),
            ("transparency", serde_json::to_string(&self.transparency)?),
            ("reduce_motion", serde_json::to_string(&self.reduce_motion)?),
            (
                "tray_hint_shown",
                serde_json::to_string(&self.tray_hint_shown)?,
            ),
            (
                "notify_connections",
                serde_json::to_string(&self.notify_connections)?,
            ),
            (
                "notify_pairing",
                serde_json::to_string(&self.notify_pairing)?,
            ),
            ("notify_driver", serde_json::to_string(&self.notify_driver)?),
            (
                "notify_skip_fullscreen",
                serde_json::to_string(&self.notify_skip_fullscreen)?,
            ),
        ])
    }

    /// Aplica o valor de uma chave. Chave desconhecida é ignorada (pode ser de uma versão
    /// futura do app).
    fn apply(&mut self, key: &str, json: &str) -> Result<(), serde_json::Error> {
        match key {
            "language" => self.language = serde_json::from_str(json)?,
            "theme" => self.theme = serde_json::from_str(json)?,
            "accent" => self.accent = serde_json::from_str(json)?,
            "ports" => self.ports = serde_json::from_str(json)?,
            "autostart" => self.autostart = serde_json::from_str(json)?,
            "start_minimized" => self.start_minimized = serde_json::from_str(json)?,
            "onboarding_completed" => self.onboarding_completed = serde_json::from_str(json)?,
            "require_approval" => self.require_approval = serde_json::from_str(json)?,
            "server_name" => self.server_name = serde_json::from_str(json)?,
            "close_to_tray" => self.close_to_tray = serde_json::from_str(json)?,
            "default_profile_id" => self.default_profile_id = serde_json::from_str(json)?,
            "send_rate_hz" => self.send_rate_hz = serde_json::from_str(json)?,
            "reservation_secs" => self.reservation_secs = serde_json::from_str(json)?,
            "transparency" => self.transparency = serde_json::from_str(json)?,
            "reduce_motion" => self.reduce_motion = serde_json::from_str(json)?,
            "tray_hint_shown" => self.tray_hint_shown = serde_json::from_str(json)?,
            "notify_connections" => self.notify_connections = serde_json::from_str(json)?,
            "notify_pairing" => self.notify_pairing = serde_json::from_str(json)?,
            "notify_driver" => self.notify_driver = serde_json::from_str(json)?,
            "notify_skip_fullscreen" => {
                self.notify_skip_fullscreen = serde_json::from_str(json)?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Leitura e gravação da tabela `settings`.
#[derive(Debug, Clone)]
pub struct SettingsRepo {
    pool: SqlitePool,
}

impl SettingsRepo {
    /// Repositório sobre `pool`.
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lê todas as configurações. Uma chave ausente, desconhecida ou com JSON inválido não
    /// derruba o app: vale o padrão daquele campo (e o problema vai para o log).
    pub async fn load(&self) -> Result<AppSettings, DataError> {
        let rows = sqlx::query!("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await?;
        let mut settings = AppSettings::default();
        for row in rows {
            if let Err(error) = settings.apply(&row.key, &row.value) {
                tracing::warn!(key = %row.key, %error, "configuração inválida no banco: usando o padrão");
            }
        }
        Ok(settings)
    }

    /// Grava todas as configurações numa transação.
    pub async fn save(&self, settings: &AppSettings) -> Result<(), DataError> {
        let now = now_ms();
        let entries = settings
            .entries()
            .map_err(|e| DataError::Invalid(e.to_string()))?;
        let mut tx = self.pool.begin().await?;
        for (key, value) in entries {
            sqlx::query!(
                "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                key,
                value,
                now
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

/// Configurações em memória + persistência + aviso de mudança.
pub struct SettingsService {
    repo: SettingsRepo,
    current: RwLock<Arc<AppSettings>>,
    changes: broadcast::Sender<Arc<AppSettings>>,
}

impl SettingsService {
    /// Carrega as configurações do banco.
    pub async fn load(repo: SettingsRepo) -> Result<Self, DataError> {
        let current = repo.load().await?;
        let (changes, _) = broadcast::channel(16);
        Ok(Self {
            repo,
            current: RwLock::new(Arc::new(current)),
            changes,
        })
    }

    /// As configurações atuais (sem ir ao banco).
    pub fn get(&self) -> Arc<AppSettings> {
        Arc::clone(&self.current.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Avisos de `settings:changed`: a UI e o core assinam.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<AppSettings>> {
        self.changes.subscribe()
    }

    /// Valida, grava e avisa. Sem mudança real, não grava nem avisa.
    pub async fn update(&self, new: AppSettings) -> Result<Arc<AppSettings>, DataError> {
        new.validate()?;
        if *self.get() == new {
            return Ok(self.get());
        }
        self.repo.save(&new).await?;
        let new = Arc::new(new);
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = Arc::clone(&new);
        // Sem ouvintes não é erro.
        let _ = self.changes.send(Arc::clone(&new));
        Ok(new)
    }

    /// Aplica `change` sobre as configurações atuais (`update` com leitura-modificação-escrita).
    pub async fn modify(
        &self,
        change: impl FnOnce(&mut AppSettings),
    ) -> Result<Arc<AppSettings>, DataError> {
        let mut next = (*self.get()).clone();
        change(&mut next);
        self.update(next).await
    }
}
