fn main() {
    // `tauri-build` solo incrusta el manifiesto de Windows en el binario de la app, no en
    // el de los tests. Sin él, Windows carga una versión antigua de comctl32 y los tests
    // mueren al arrancar (STATUS_ENTRYPOINT_NOT_FOUND). Por eso se lo quitamos a
    // tauri-build y lo incrustamos nosotros en *todos* los binarios con el enlazador.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("error en el build script de Tauri");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed=windows-app-manifest.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
}
