// The Windows exe carries the app icon as resource 1, which is where Explorer and GPUI's
// window class both look for it.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-env-changed=MARKETEER_UPDATER_PUBKEY");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("assets/icon.ico", "1")
            .set("ProductName", "Marketeer")
            .set("FileDescription", "Marketeer");
        res.compile().expect("could not embed the icon");
    }
}
