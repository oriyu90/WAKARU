fn main() {
    // Windows SxS manifest (W-21): single manifest for every link target.
    // rfd/common-controls-v6 (via tauri-plugin-dialog) statically imports
    // TaskDialogIndirect from comctl32 v6. Without a v6-bound manifest the
    // loader fails with 0xC0000139 before main — for the app, lib-unittests
    // and integration tests alike. tauri's resource.lib therefore carries NO
    // manifest (`new_without_app_manifest`); the rich manifest below (a
    // superset of tauri's default v6-only one) is embedded into bins, tests,
    // benches and examples via the unsuffixed `rustc-link-arg` form, so no
    // target ever sees two manifests (CVT1100). No-op off Windows.
    let mut attrs = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        attrs =
            attrs.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }
    tauri_build::try_build(attrs).expect("failed to run tauri-build");

    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let manifest = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3" manifestVersion="1.0"><assemblyIdentity name="com.yukiorita.wakaru" type="win32" version="{major}.{minor}.{patch}.0"></assemblyIdentity><dependency><dependentAssembly><assemblyIdentity language="*" name="Microsoft.Windows.Common-Controls" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" type="win32" version="6.0.0.0"></assemblyIdentity></dependentAssembly></dependency><compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1"><application><supportedOS Id="{{35138b9a-5d96-4fbd-8e2d-a2440225f93a}}"></supportedOS><supportedOS Id="{{4a2f28e3-53b6-11d1-9aa4-000f81bc9c05}}"></supportedOS><supportedOS Id="{{1f676c76-80e1-4239-95bb-83d0f6d0da78}}"></supportedOS><supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"></supportedOS></application></compatibility><asmv3:application><asmv3:windowsSettings><activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage><dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2</dpiAwareness><longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware></asmv3:windowsSettings></asmv3:application><asmv3:trustInfo><asmv3:security><asmv3:requestedPrivileges><asmv3:requestedExecutionLevel level="asInvoker" uiAccess="false"></asmv3:requestedExecutionLevel></asmv3:requestedPrivileges></asmv3:security></asmv3:trustInfo></assembly>"#,
            major = std::env::var("CARGO_PKG_VERSION_MAJOR").unwrap(),
            minor = std::env::var("CARGO_PKG_VERSION_MINOR").unwrap(),
            patch = std::env::var("CARGO_PKG_VERSION_PATCH").unwrap(),
        );
        let path =
            std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app-manifest.xml");
        std::fs::write(&path, manifest).expect("unable to write app manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", path.display());
        println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
    }
}
