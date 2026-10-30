//! Embeds the HBC-2 icon in the Windows executable (shown by Explorer and the taskbar).

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource
            .compile()
            .expect("cannot embed the icon in the executable");
    }
}
