fn main() {
    // Comandos do programa que a página do painel pode pedir (liberados só em capabilities/remoto.json)
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["ajustar_altura", "abrir_menu"])),
    )
    .expect("não deu para preparar o programa");
}
