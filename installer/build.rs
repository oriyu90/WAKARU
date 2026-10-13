fn main() {
    // Fallback payload for `cargo test` / plain `cargo build` without
    // `WAKARU_PAYLOAD_DIR` (unit tests only exercise arg parsing, messages
    // and path helpers — never the embedded bytes). Real packaging
    // (`windows/package.ps1`) sets a full payload dir, which takes precedence.
    if std::env::var("WAKARU_PAYLOAD_DIR").is_err() {
        let fallback = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("dummy-payload");
        let _ = std::fs::create_dir_all(&fallback);
        for (name, body) in [
            ("WAKARU.exe", &b"dummy-exe"[..]),
            ("LICENSE", &b"dummy-license"[..]),
            ("FIRST_LAUNCH_WINDOWS.txt", &b"dummy-first-launch"[..]),
        ] {
            let path = fallback.join(name);
            if !path.is_file() {
                let _ = std::fs::write(&path, body);
            }
        }
        println!("cargo:rustc-env=WAKARU_PAYLOAD_DIR={}", fallback.display());
    }
    // Installer SxS manifest: UTF-8 code page, per-monitor-v2 DPI awareness,
    // long-path awareness and asInvoker. Mirrors the app manifest subset that
    // matters for an installer (no Common-Controls v6 dependency, but keeping
    // the same code-page/long-path behaviour avoids MAX_PATH failures on
    // deep --dir targets and mojibake in JA/EN prompts). No-op off Windows.
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let manifest = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3" manifestVersion="1.0"><assemblyIdentity name="com.yukiorita.wakaru.setup" type="win32" version="{major}.{minor}.{patch}.0"></assemblyIdentity><compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1"><application><supportedOS Id="{{35138b9a-5d96-4fbd-8e2d-a2440225f93a}}"></supportedOS><supportedOS Id="{{4a2f28e3-53b6-11d1-9aa4-000f81bc9c05}}"></supportedOS><supportedOS Id="{{1f676c76-80e1-4239-95bb-83d0f6d0da78}}"></supportedOS><supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"></supportedOS></application></compatibility><asmv3:application><asmv3:windowsSettings><activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage><dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2</dpiAwareness><longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware></asmv3:windowsSettings></asmv3:application><asmv3:trustInfo><asmv3:security><asmv3:requestedPrivileges><asmv3:requestedExecutionLevel level="asInvoker" uiAccess="false"></asmv3:requestedExecutionLevel></asmv3:requestedPrivileges></asmv3:security></asmv3:trustInfo></assembly>"#,
            major = std::env::var("CARGO_PKG_VERSION_MAJOR").unwrap(),
            minor = std::env::var("CARGO_PKG_VERSION_MINOR").unwrap(),
            patch = std::env::var("CARGO_PKG_VERSION_PATCH").unwrap(),
        );
        let path =
            std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("setup-manifest.xml");
        std::fs::write(&path, manifest).expect("unable to write setup manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", path.display());
        println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
    }
}
