//! Erros da camada de persistência.

use std::fmt;
use std::path::PathBuf;

/// Uma migration que falhou (ou que não pôde nem começar).
#[derive(Debug)]
pub struct MigrationFailure {
    /// Versão da migration que falhou, quando se sabe.
    pub version: Option<i64>,
    /// Descrição (o nome do arquivo depois do número).
    pub description: Option<String>,
    /// O erro técnico, para o log e para o suporte.
    pub cause: String,
    /// O backup feito antes de migrar, se havia dados a proteger.
    pub backup: Option<PathBuf>,
    /// O banco voltou ao estado do backup.
    pub restored: bool,
    /// O banco era novo (sem dados) e foi descartado para o próximo boot recomeçar limpo.
    pub discarded_fresh: bool,
}

impl fmt::Display for MigrationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Não foi possível atualizar o banco de dados do PadGaming")?;
        match (self.version, &self.description) {
            (Some(v), Some(d)) => write!(f, " (etapa {v}: {d})")?,
            (Some(v), None) => write!(f, " (etapa {v})")?,
            _ => {}
        }
        f.write_str(". ")?;
        if self.restored {
            match &self.backup {
                Some(b) => write!(
                    f,
                    "Seus dados foram restaurados do backup ({}). Feche o app e tente de novo; \
                     se o erro continuar, envie o diagnóstico ao suporte.",
                    b.display()
                )?,
                None => f.write_str("Seus dados foram restaurados.")?,
            }
        } else if self.discarded_fresh {
            f.write_str(
                "O banco ainda não tinha dados e foi descartado; abrir o app de novo recomeça do \
                 zero. Se o erro continuar, envie o diagnóstico ao suporte.",
            )?;
        } else {
            match &self.backup {
                Some(b) => write!(
                    f,
                    "A restauração automática falhou; o backup está em {}. Envie o diagnóstico ao \
                     suporte antes de mexer nos arquivos.",
                    b.display()
                )?,
                None => f.write_str("Envie o diagnóstico ao suporte.")?,
            }
        }
        write!(f, " Detalhe técnico: {}", self.cause)
    }
}

/// Falha ao abrir ou migrar o banco.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Falha ao acessar um arquivo ou pasta.
    #[error("não consegui acessar {}: {source}", path.display())]
    Io {
        /// Caminho envolvido.
        path: PathBuf,
        /// Erro original.
        source: std::io::Error,
    },
    /// Erro do SQLite/sqlx.
    #[error("erro no banco de dados: {0}")]
    Database(#[from] sqlx::Error),
    /// Uma migration falhou; o `Display` já é uma mensagem para o usuário.
    #[error("{0}")]
    Migration(Box<MigrationFailure>),
    /// O banco foi criado por uma versão mais nova do app.
    #[error(
        "este banco de dados foi criado por uma versão mais nova do PadGaming (etapa {found}, \
         esta versão conhece até a {latest}). Atualize o app para abri-lo."
    )]
    TooNew {
        /// Maior migration aplicada no banco.
        found: i64,
        /// Maior migration que este app conhece.
        latest: i64,
    },
}

impl StorageError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Texto para mostrar ao usuário.
    pub fn user_message(&self) -> String {
        self.to_string()
    }
}

/// Falha de uma operação de dados (repositórios e serviços), depois de o banco estar aberto.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    /// Não existe o registro pedido.
    #[error("{0} não encontrado")]
    NotFound(&'static str),
    /// O dado não passou na validação (o texto diz o motivo, para mostrar ao usuário).
    #[error("{0}")]
    Invalid(String),
    /// O perfil embutido não pode ser editado nem apagado; duplique-o para personalizar.
    #[error(
        "o perfil padrão é do app e não pode ser alterado nem apagado; duplique-o para personalizar"
    )]
    Builtin,
    /// O perfil está em uso; apagar exige confirmação.
    #[error("o perfil está em uso por {devices} dispositivo(s) e {slots} controle(s) ativo(s)")]
    InUse {
        /// Dispositivos que o têm como perfil padrão.
        devices: u32,
        /// Controles ativos usando o perfil agora.
        slots: u32,
    },
    /// Arquivo de perfil (`.padprofile`) inválido.
    #[error("arquivo de perfil inválido: {0}")]
    Import(String),
    /// Erro do SQLite/sqlx.
    #[error("erro no banco de dados: {0}")]
    Database(#[from] sqlx::Error),
}
