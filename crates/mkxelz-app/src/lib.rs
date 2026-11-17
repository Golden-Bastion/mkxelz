//! MkXelz GUI. Touch-first egui app; the same code runs on desktop for development.

mod app;

pub use app::MkXelzApp;

#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> eframe::Result<()> {
    let data_dir = std::env::var("MKXELZ_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("mkxelz-data"));
    let options = eframe::NativeOptions {
        // Phone-shaped window so the layout matches the Android target.
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([420.0, 860.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MkXelz",
        options,
        Box::new(move |cc| Ok(Box::new(MkXelzApp::new(cc, data_dir)))),
    )
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    // App-specific storage: /sdcard/Android/data/<package>/files
    let data_dir = app
        .external_data_path()
        .or_else(|| app.internal_data_path())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "MkXelz",
        options,
        Box::new(move |cc| Ok(Box::new(MkXelzApp::new(cc, data_dir)))),
    );
}
