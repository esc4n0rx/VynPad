// Evita a janela extra de console no Windows em release. NÃO REMOVER.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    padgaming_desktop_lib::run();
}
