//! Windows only: embed the app icon and version info (VERSIONINFO) into `filmcraft.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing (`winresource` is only a build-dependency on Windows
//! hosts). A missing resource compiler is a warning, unless `FILMCRAFT_REQUIRE_WINRES=1` (for
//! release builds) turns it into an error.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/easyvideoedit.ico");
    println!("cargo:rerun-if-changed=../../crates/ui-egui/src/brand.rs");
    println!("cargo:rerun-if-env-changed=FILMCRAFT_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    windows::embed_resources();
}

#[cfg(windows)]
mod windows {
    /// Easy Video Edit's own version (`brand::VERSION`): the workspace's is FilmCraft's.
    fn app_version() -> Option<(String, u64)> {
        let src = std::fs::read_to_string("../../crates/ui-egui/src/brand.rs").ok()?;
        let version = src.split_once("pub const VERSION: &str = \"")?.1.split('"').next()?.to_string();
        let mut parts = version.split('.').map(|p| p.parse::<u16>().unwrap_or(0));
        let mut packed = 0u64;
        for _ in 0..4 {
            packed = (packed << 16) | u64::from(parts.next().unwrap_or(0));
        }
        Some((version, packed))
    }

    pub fn embed_resources() {
        let mut res = winresource::WindowsResource::new();
        match app_version() {
            Some((text, packed)) => {
                res.set("FileVersion", &text).set("ProductVersion", &text);
                res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
                res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
            }
            None => println!("cargo:warning=brand::VERSION not found: filmcraft.exe carries FilmCraft's version"),
        }
        res.set_icon("../../assets/app-icon/easyvideoedit.ico")
            .set("ProductName", "Easy Video Edit")
            .set("FileDescription", "Easy Video Edit video editor")
            .set("LegalCopyright", "Copyright (c) the Easy Video Edit and FilmCraft contributors. MIT OR Apache-2.0.")
            .set("OriginalFilename", "filmcraft.exe")
            .set("InternalName", "filmcraft");
        if let Err(e) = res.compile() {
            if std::env::var_os("FILMCRAFT_REQUIRE_WINRES").is_some() {
                panic!("embedding Windows resources failed: {e}");
            }
            println!("cargo:warning=filmcraft.exe built without icon/version resources: {e}");
        }
    }
}

/// Cross-compiling for Windows from macOS or Linux: `winresource` is not available on this host.
#[cfg(not(windows))]
mod windows {
    pub fn embed_resources() {
        println!("cargo:warning=filmcraft.exe built without icon/version resources (cross-compiled from a non-Windows host)");
    }
}
