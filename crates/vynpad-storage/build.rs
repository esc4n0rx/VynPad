//! Recompila quando uma migration muda: o `sqlx::migrate!` embute os arquivos, mas o cargo
//! não sabe que eles são entradas da compilação.

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
