// Release builds on Windows open no console window next to the app's own (debug builds keep it for logs).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

/// `packaging/culm.svg` at 256 px, the size Rune keeps (`inkscape packaging/culm.svg -w 256 -h 256 -o crates/app/assets/icon/culm-256.png`).
const ICON: &[u8] = include_bytes!("../assets/icon/culm-256.png");

fn main() {
    rune::set_app_id(addon_app::spec_app_id());
    // The icon of the running windows (title bar, taskbar, Alt+Tab, Dock). Rune applies it on Windows and macOS;
    // on Linux the desktop entry matched by the app id names it.
    rune::set_app_icon(ICON).expect("the bundled icon is a PNG");
    rune::run(addon_app::spec());
}
