fn main() {
    println!("cargo:rerun-if-env-changed=OPETS_DATA_KEY_V1");
    let profile = std::env::var("PROFILE").expect("Cargo must define PROFILE");
    let key = std::env::var("OPETS_DATA_KEY_V1").unwrap_or_else(|_| {
        if profile == "release" {
            panic!("OPETS_DATA_KEY_V1 must be set for release builds");
        }
        "6a401b8d2f91c5e37a9d0b4e8f6c2a1d9b7e5f3c0a8d6e4b2f1c9a7d5e3b1f8a".to_string()
    });
    if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        panic!("OPETS_DATA_KEY_V1 must be a 64-character hexadecimal key");
    }
    println!("cargo:rustc-env=OPETS_DATA_KEY_V1={key}");

    let mut attributes = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        embed_windows_manifest();
    }

    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}

fn embed_windows_manifest() {
    let manifest = std::env::current_dir()
        .expect("failed to resolve the crate directory")
        .join("windows-app-manifest.xml");

    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
