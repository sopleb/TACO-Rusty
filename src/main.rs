mod audio;
mod config;
mod core;
mod intel;
mod rendering;
pub mod resources;
mod ui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    #[cfg(target_os = "linux")]
    install_linux_desktop_entry();

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("T.A.C.O. - Tactical Awareness Control Overlay")
            .with_inner_size([1280.0, 800.0])
            .with_icon(icon)
            .with_app_id("taco"),
        renderer: eframe::Renderer::Glow,
        depth_buffer: 24,
        ..Default::default()
    };

    eframe::run_native(
        "TACO",
        options,
        Box::new(|cc| Ok(Box::new(ui::app::TacoApp::new(cc)))),
    )
}

#[cfg(target_os = "linux")]
fn install_linux_desktop_entry() {
    let Some(data_dir) = dirs::data_dir() else { return };

    let icon_dir = data_dir.join("icons/hicolor/512x512/apps");
    let icon_path = icon_dir.join("taco.png");
    if !icon_path.exists() && std::fs::create_dir_all(&icon_dir).is_ok() {
        let _ = std::fs::write(&icon_path, resources::TEX_ICON);
    }

    let desktop_dir = data_dir.join("applications");
    let desktop_path = desktop_dir.join("taco.desktop");
    if !desktop_path.exists() && std::fs::create_dir_all(&desktop_dir).is_ok() {
        let exec_path = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "taco".to_string());
        let desktop_content = format!(
            "[Desktop Entry]\n\
             Name=T.A.C.O.\n\
             Comment=Tactical Awareness Control Overlay for EVE Online\n\
             Exec={exec_path}\n\
             Icon=taco\n\
             Terminal=false\n\
             Type=Application\n\
             Categories=Game;Utility;\n\
             StartupWMClass=taco\n"
        );
        let _ = std::fs::write(&desktop_path, desktop_content);
    }
}

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(resources::TEX_ICON)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(32, 32))
        .into_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}
