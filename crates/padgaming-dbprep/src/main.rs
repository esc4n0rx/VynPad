//! Cria `<pasta>/padgaming.db` com as migrations de `crates/padgaming-storage/migrations`
//! (sem seeds), para o `sqlx` verificar as queries em compilação. Usado por
//! `scripts/sqlx-prepare.sh`.
//!
//! ```text
//! cargo run -p padgaming-dbprep -- target/sqlx-prepare
//! ```

use std::path::PathBuf;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("uso: padgaming-dbprep <pasta>")?,
    );
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("padgaming.db");
    let migrations =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../padgaming-storage/migrations");

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&file)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await?;
    Migrator::new(migrations).await?.run(&pool).await?;
    pool.close().await;
    println!("{}", file.display());
    Ok(())
}
