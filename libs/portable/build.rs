fn main() {
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut res = winres::WindowsResource::new();
        // IW: the released file shows the fork version (1.4.9-iw.N), not the RustDesk base
        for f in ["../../src/iw/mod.rs", "build.rs", "Cargo.toml", "../../res/icon.ico", "../../res/manifest.xml"] {
            println!("cargo:rerun-if-changed={f}");
        }
        let iw = std::fs::read_to_string("../../src/iw/mod.rs").expect("src/iw/mod.rs");
        let version = iw
            .lines()
            .find_map(|l| l.strip_prefix("pub const IW_VERSION: &str = \"")?.strip_suffix("\";"))
            .expect("IW_VERSION in src/iw/mod.rs");
        res.set("FileVersion", version).set("ProductVersion", version);
        res.set_icon("../../res/icon.ico")
            .set_language(winapi::um::winnt::MAKELANGID(
                winapi::um::winnt::LANG_ENGLISH,
                winapi::um::winnt::SUBLANG_ENGLISH_US,
            ))
            .set_manifest_file("../../res/manifest.xml");
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
}
