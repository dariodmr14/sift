//! Punto de entrada de la app Tauri. De momento solo abre la ventana principal.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Tauri");
}
