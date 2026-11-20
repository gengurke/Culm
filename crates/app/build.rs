//! The Rune commit Culm is built on (`RUNE_REV`, for "Copy debug info"), from the workspace's `Cargo.lock`.
//! Windows: the icon Explorer and the Start menu show for culm.exe, and its version info (Properties → Details).
//! The icon `assets/icon/culm.ico` is made from `packaging/culm.svg` by `packaging/icons.sh`.

#[allow(dead_code)]
#[path = "src/brand.rs"]
mod brand;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/culm.ico");
    println!("cargo:rerun-if-changed=src/brand.rs");
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    let lock = std::fs::read_to_string("../../Cargo.lock").unwrap_or_default();
    let rev = lock
        .lines()
        .find_map(|l| l.strip_prefix("source = \"git+https://github.com/anomalyco/rune?"))
        .and_then(|l| l.split_once('#'))
        .map_or("unknown", |(_, sha)| sha.trim_end_matches('"'));
    println!("cargo:rustc-env=RUNE_REV={}", &rev[..rev.len().min(12)]);
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
