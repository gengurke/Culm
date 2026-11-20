//! Windows: the icon Explorer and the Start menu show for culm.exe, and its version info (Properties → Details).
//! The icon `assets/icon/culm.ico` is made from `packaging/culm.svg` by `packaging/icons.sh`.

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
        .set("LegalCopyright", "© Culm contributors, GPL-3.0-or-later");
    res.compile()
        .expect("the Windows resources (icon, version info) compile");
}
