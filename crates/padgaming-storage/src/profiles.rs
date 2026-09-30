//! Perfis de mapeamento (`profiles`): repositório, serviço (CRUD, duplicar, importar/exportar
//! `.padprofile`) e a troca a quente no `SlotManager` através de [`ProfileHost`].

use std::sync::Arc;

use padgaming_mapping::{CompiledProfile, Profile, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::DataError;
use crate::seed::{self, BUILTIN_PROFILE_ID, now_ms};

/// Tamanho máximo do nome do perfil.
pub const MAX_NAME_LEN: usize = 64;
/// Tamanho máximo da descrição.
pub const MAX_DESCRIPTION_LEN: usize = 500;
/// Tamanho máximo do nome do ícone.
pub const MAX_ICON_LEN: usize = 32;
/// Maior arquivo `.padprofile` aceito na importação.
pub const MAX_IMPORT_BYTES: usize = 256 * 1024;
/// Valor de `format` no `.padprofile`.
pub const PADPROFILE_FORMAT: &str = "padprofile";
/// Versão do envelope `.padprofile`.
pub const PADPROFILE_VERSION: u32 = 1;

/// Metadados de um perfil (sem o JSON do mapeamento; barato de listar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSummary {
    /// UUID.
    pub id: String,
    /// Nome.
    pub name: String,
    /// Descrição.
    pub description: Option<String>,
    /// Nome do ícone/emoji.
    pub icon: Option<String>,
    /// Perfil do app (somente leitura).
    pub is_builtin: bool,
    /// Versão do schema do `config`.
    pub schema_version: u32,
    /// Criação (epoch ms).
    pub created_at: i64,
    /// Última alteração (epoch ms).
    pub updated_at: i64,
}

/// Um perfil completo.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredProfile {
    /// Metadados.
    pub summary: ProfileSummary,
    /// A configuração de mapeamento.
    pub profile: Profile,
}

/// Dados para criar ou editar um perfil.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileInput {
    /// Nome (1–64 caracteres).
    pub name: String,
    /// Descrição (até 500).
    pub description: Option<String>,
    /// Ícone (até 32).
    pub icon: Option<String>,
    /// A configuração. Precisa validar e compilar.
    pub profile: Profile,
}

/// O perfil que acabou valendo para um controle.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    /// `id` do perfil (o padrão, se o pedido não existia ou estava inválido).
    pub id: String,
    /// O perfil compilado.
    pub compiled: Arc<CompiledProfile>,
}

/// Quem aplica um perfil nos controles ativos (o `SlotManager`, na camada de composição).
pub trait ProfileHost: Send + Sync {
    /// O perfil `id` mudou: todo controle que o usa passa a usar `compiled` na hora, sem
    /// reconectar.
    fn apply(&self, id: &str, compiled: Arc<CompiledProfile>);

    /// Quantos controles ativos usam o perfil `id`.
    fn slots_using(&self, id: &str) -> u32;

    /// O perfil `id` foi apagado com controles usando-o: eles passam para `fallback`.
    fn removed(&self, id: &str, fallback: Arc<CompiledProfile>);
}

/// [`ProfileHost`] que não faz nada (testes, ou app sem controles ativos).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHost;

impl ProfileHost for NoHost {
    fn apply(&self, _id: &str, _compiled: Arc<CompiledProfile>) {}

    fn slots_using(&self, _id: &str) -> u32 {
        0
    }

    fn removed(&self, _id: &str, _fallback: Arc<CompiledProfile>) {}
}

struct Row {
    id: String,
    name: String,
    description: Option<String>,
    icon: Option<String>,
    is_builtin: bool,
    schema_version: i64,
    created_at: i64,
    updated_at: i64,
}

impl From<Row> for ProfileSummary {
    fn from(row: Row) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            icon: row.icon,
            is_builtin: row.is_builtin,
            schema_version: u32::try_from(row.schema_version).unwrap_or(0),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Leitura e gravação da tabela `profiles`.
#[derive(Debug, Clone)]
pub struct ProfileRepo {
    pool: SqlitePool,
}

impl ProfileRepo {
    /// Repositório sobre `pool`.
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Todos os perfis: o embutido primeiro, depois por nome.
    pub async fn list(&self) -> Result<Vec<ProfileSummary>, DataError> {
        Ok(sqlx::query_as!(
            Row,
            r#"SELECT id, name, description, icon, is_builtin AS "is_builtin: bool",
                      schema_version, created_at, updated_at
               FROM profiles ORDER BY is_builtin DESC, name COLLATE NOCASE, id"#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(ProfileSummary::from)
        .collect())
    }

    /// Metadados e JSON de um perfil.
    pub async fn get(&self, id: &str) -> Result<Option<(ProfileSummary, String)>, DataError> {
        let row = sqlx::query!(
            r#"SELECT id, name, description, icon, is_builtin AS "is_builtin: bool",
                      schema_version, config, created_at, updated_at
               FROM profiles WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| {
            (
                ProfileSummary::from(Row {
                    id: r.id,
                    name: r.name,
                    description: r.description,
                    icon: r.icon,
                    is_builtin: r.is_builtin,
                    schema_version: r.schema_version,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                }),
                r.config,
            )
        }))
    }

    async fn insert(&self, summary: &ProfileSummary, config: &str) -> Result<(), DataError> {
        let schema_version = i64::from(summary.schema_version);
        sqlx::query!(
            "INSERT INTO profiles (id, name, description, icon, is_builtin, schema_version, config, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 0, ?, ?, ?, ?)",
            summary.id,
            summary.name,
            summary.description,
            summary.icon,
            schema_version,
            config,
            summary.created_at,
            summary.updated_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, summary: &ProfileSummary, config: &str) -> Result<(), DataError> {
        let schema_version = i64::from(summary.schema_version);
        let done = sqlx::query!(
            "UPDATE profiles SET name = ?, description = ?, icon = ?, schema_version = ?, config = ?, updated_at = ? \
             WHERE id = ? AND is_builtin = 0",
            summary.name,
            summary.description,
            summary.icon,
            schema_version,
            config,
            summary.updated_at,
            summary.id
        )
        .execute(&self.pool)
        .await?;
        if done.rows_affected() == 0 {
            return Err(DataError::NotFound("perfil"));
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DataError> {
        let done = sqlx::query!("DELETE FROM profiles WHERE id = ? AND is_builtin = 0", id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(DataError::NotFound("perfil"));
        }
        Ok(())
    }

    /// Quantos dispositivos têm o perfil como padrão.
    pub async fn devices_using(&self, id: &str) -> Result<u32, DataError> {
        let row = sqlx::query!(
            r#"SELECT count(*) AS "count!: i64" FROM devices WHERE default_profile_id = ?"#,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(u32::try_from(row.count).unwrap_or(u32::MAX))
    }
}

/// O envelope de um arquivo `.padprofile`.
#[derive(Debug, Serialize, Deserialize)]
struct PadProfile {
    format: String,
    version: u32,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    profile: Profile,
}

fn clean_text(value: Option<String>, max: usize, what: &str) -> Result<Option<String>, DataError> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(DataError::Invalid(format!(
            "{what} passa de {max} caracteres"
        )));
    }
    Ok(Some(value))
}

/// Valida os textos e o mapeamento. Devolve a entrada normalizada e o perfil compilado.
fn prepare(input: ProfileInput) -> Result<(ProfileInput, CompiledProfile), DataError> {
    let name = input.name.trim().to_owned();
    let len = name.chars().count();
    if len == 0 || len > MAX_NAME_LEN {
        return Err(DataError::Invalid(format!(
            "o nome do perfil precisa ter de 1 a {MAX_NAME_LEN} caracteres"
        )));
    }
    let description = clean_text(input.description, MAX_DESCRIPTION_LEN, "a descrição")?;
    let icon = clean_text(input.icon, MAX_ICON_LEN, "o nome do ícone")?;
    let mut profile = input.profile;
    // O nome do perfil é o da linha; o JSON acompanha para o arquivo exportado ser coerente.
    profile.name.clone_from(&name);
    let compiled = CompiledProfile::compile(&profile)
        .map_err(|e| DataError::Invalid(format!("perfil inválido: {e}")))?;
    Ok((
        ProfileInput {
            name,
            description,
            icon,
            profile,
        },
        compiled,
    ))
}

/// Serviço de perfis.
pub struct ProfileService {
    repo: ProfileRepo,
    pool: SqlitePool,
    host: Arc<dyn ProfileHost>,
}

impl ProfileService {
    /// Serviço sobre `pool`, que avisa `host` quando um perfil muda.
    pub fn new(pool: SqlitePool, host: Arc<dyn ProfileHost>) -> Self {
        Self {
            repo: ProfileRepo::new(pool.clone()),
            pool,
            host,
        }
    }

    /// O repositório (consultas simples, como `devices_using`).
    pub const fn repo(&self) -> &ProfileRepo {
        &self.repo
    }

    /// Todos os perfis (sem o JSON).
    pub async fn list(&self) -> Result<Vec<ProfileSummary>, DataError> {
        self.repo.list().await
    }

    /// Um perfil completo. `config` corrompido ou de outro schema vira `DataError::Invalid`.
    pub async fn get(&self, id: &str) -> Result<StoredProfile, DataError> {
        let (summary, config) = self
            .repo
            .get(id)
            .await?
            .ok_or(DataError::NotFound("perfil"))?;
        let profile = Profile::from_json(&config).map_err(|e| {
            DataError::Invalid(format!("o perfil \"{}\" está inválido: {e}", summary.name))
        })?;
        Ok(StoredProfile { summary, profile })
    }

    /// O perfil já compilado (o do `id`, ou o padrão se `id` for `None`, não existir ou estiver
    /// corrompido: o controle nunca fica sem perfil).
    pub async fn compiled_or_default(&self, id: Option<&str>) -> Arc<CompiledProfile> {
        self.resolve_or_default(id).await.compiled
    }

    /// Como [`Self::compiled_or_default`], dizendo também qual perfil acabou valendo.
    pub async fn resolve_or_default(&self, id: Option<&str>) -> ResolvedProfile {
        if let Some(id) = id {
            match self.get(id).await {
                Ok(stored) => match CompiledProfile::compile(&stored.profile) {
                    Ok(compiled) => {
                        return ResolvedProfile {
                            id: id.to_owned(),
                            compiled: Arc::new(compiled),
                        };
                    }
                    Err(error) => tracing::warn!(id, %error, "perfil não compila: usando o padrão"),
                },
                Err(error) => tracing::warn!(id, %error, "perfil indisponível: usando o padrão"),
            }
        }
        ResolvedProfile {
            id: BUILTIN_PROFILE_ID.to_owned(),
            compiled: Arc::new(
                CompiledProfile::compile(&Profile::standard()).expect("perfil padrão é válido"),
            ),
        }
    }

    /// Cria um perfil do usuário.
    pub async fn create(&self, input: ProfileInput) -> Result<StoredProfile, DataError> {
        let (input, _compiled) = prepare(input)?;
        let now = now_ms();
        let summary = ProfileSummary {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            description: input.description,
            icon: input.icon,
            is_builtin: false,
            schema_version: SCHEMA_VERSION,
            created_at: now,
            updated_at: now,
        };
        self.repo.insert(&summary, &input.profile.to_json()).await?;
        Ok(StoredProfile {
            summary,
            profile: input.profile,
        })
    }

    /// Salva as alterações e aplica na hora nos controles que usam o perfil (sem reconectar).
    /// O perfil embutido não se edita: duplique-o.
    pub async fn save(&self, id: &str, input: ProfileInput) -> Result<StoredProfile, DataError> {
        let current = self.get(id).await.or_else(|error| match error {
            // Um perfil corrompido pode ser regravado (é como o usuário o conserta).
            DataError::Invalid(_) => Ok(StoredProfile {
                summary: ProfileSummary {
                    id: id.to_owned(),
                    name: String::new(),
                    description: None,
                    icon: None,
                    is_builtin: false,
                    schema_version: SCHEMA_VERSION,
                    created_at: now_ms(),
                    updated_at: now_ms(),
                },
                profile: Profile::identity(),
            }),
            other => Err(other),
        })?;
        if current.summary.is_builtin || id == BUILTIN_PROFILE_ID {
            return Err(DataError::Builtin);
        }
        let (input, compiled) = prepare(input)?;
        let summary = ProfileSummary {
            name: input.name,
            description: input.description,
            icon: input.icon,
            schema_version: SCHEMA_VERSION,
            updated_at: now_ms(),
            ..current.summary
        };
        self.repo.update(&summary, &input.profile.to_json()).await?;
        self.host.apply(id, Arc::new(compiled));
        Ok(StoredProfile {
            summary,
            profile: input.profile,
        })
    }

    /// Copia um perfil (inclusive o embutido) como perfil do usuário.
    pub async fn duplicate(
        &self,
        id: &str,
        new_name: Option<&str>,
    ) -> Result<StoredProfile, DataError> {
        let original = self.get(id).await?;
        let name = new_name.map_or_else(
            || {
                let base: String = original
                    .summary
                    .name
                    .chars()
                    .take(MAX_NAME_LEN - " (cópia)".chars().count())
                    .collect();
                format!("{base} (cópia)")
            },
            str::to_owned,
        );
        self.create(ProfileInput {
            name,
            description: original.summary.description,
            icon: original.summary.icon,
            profile: original.profile,
        })
        .await
    }

    /// Apaga um perfil. O embutido nunca; um perfil em uso (padrão de algum dispositivo ou
    /// ativo num controle) só com `confirm = true`, e então os dispositivos ficam sem perfil
    /// padrão e os controles ativos passam para o perfil "Padrão".
    pub async fn delete(&self, id: &str, confirm: bool) -> Result<(), DataError> {
        let (summary, _) = self
            .repo
            .get(id)
            .await?
            .ok_or(DataError::NotFound("perfil"))?;
        if summary.is_builtin {
            return Err(DataError::Builtin);
        }
        let devices = self.repo.devices_using(id).await?;
        let slots = self.host.slots_using(id);
        if (devices > 0 || slots > 0) && !confirm {
            return Err(DataError::InUse { devices, slots });
        }
        self.repo.delete(id).await?;
        if slots > 0 {
            let fallback = self.compiled_or_default(Some(BUILTIN_PROFILE_ID)).await;
            self.host.removed(id, fallback);
        }
        Ok(())
    }

    /// Exporta como arquivo `.padprofile` (JSON).
    pub async fn export(&self, id: &str) -> Result<String, DataError> {
        let stored = self.get(id).await?;
        let file = PadProfile {
            format: PADPROFILE_FORMAT.to_owned(),
            version: PADPROFILE_VERSION,
            name: stored.summary.name,
            description: stored.summary.description,
            icon: stored.summary.icon,
            profile: stored.profile,
        };
        serde_json::to_string_pretty(&file).map_err(|e| DataError::Import(e.to_string()))
    }

    /// Importa um `.padprofile` como um perfil novo do usuário (o `id` é sempre novo).
    pub async fn import(&self, text: &str) -> Result<StoredProfile, DataError> {
        if text.len() > MAX_IMPORT_BYTES {
            return Err(DataError::Import(format!(
                "o arquivo passa de {} KiB",
                MAX_IMPORT_BYTES / 1024
            )));
        }
        let file: PadProfile = serde_json::from_str(text)
            .map_err(|e| DataError::Import(format!("não é um perfil do PadGaming ({e})")))?;
        if file.format != PADPROFILE_FORMAT {
            return Err(DataError::Import("não é um arquivo .padprofile".into()));
        }
        if file.version != PADPROFILE_VERSION {
            return Err(DataError::Import(format!(
                "versão {} do arquivo não é suportada (esta versão do app lê a {PADPROFILE_VERSION})",
                file.version
            )));
        }
        self.create(ProfileInput {
            name: file.name,
            description: file.description,
            icon: file.icon,
            profile: file.profile,
        })
        .await
        .map_err(|error| match error {
            DataError::Invalid(why) => DataError::Import(why),
            other => other,
        })
    }

    /// "Restaurar padrão": recria/repõe o perfil embutido e aplica nos controles que o usam.
    pub async fn restore_builtin(&self) -> Result<(), DataError> {
        seed::apply_builtin_profile(&self.pool, now_ms()).await?;
        let compiled = self.compiled_or_default(Some(BUILTIN_PROFILE_ID)).await;
        self.host.apply(BUILTIN_PROFILE_ID, compiled);
        Ok(())
    }
}
