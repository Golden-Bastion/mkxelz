//! Desktop entry point (for development). On Android the app starts via `android_main` in lib.rs.

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result<()> {
    mkxelz_app::run_desktop()
}

#[cfg(target_os = "android")]
fn main() {}
