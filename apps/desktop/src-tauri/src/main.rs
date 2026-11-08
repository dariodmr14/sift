// En release, evita que se abra una consola junto a la ventana en Windows. NO QUITAR.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sift_desktop_lib::run();
}
