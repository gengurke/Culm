//! Windows: the icon Explorer and the Start menu show for culm.exe, and its version info (Properties → Details).
//! The icon is `packaging/culm.svg` at 16–256 px (`crates/app/assets/icon/culm.ico`); rebuild it with
//! `for s in 16 24 32 48 64 128 256; do inkscape packaging/culm.svg -w $s -h $s -o $s.png; done` and
//! `icotool -c -o crates/app/assets/icon/culm.ico 16.png 24.png 32.png 48.png 64.png -r 128.png -r 256.png`.

#[allow(dead_code)]
#[path = "src/brand.rs"]
mod brand;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/culm.ico");
    println!("cargo:rerun-if-changed=src/brand.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon/culm.ico")
        .set("ProductName", brand::APP_NAME)
        .set("FileDescription", brand::APP_NAME)
        .set("OriginalFilename", "culm.exe")
        .set("LegalCopyright", "GPL-3.0-or-later");
    res.compile()
        .expect("the Windows resources (icon, version info) compile");
}
