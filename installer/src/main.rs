//! WAKARU per-user installer for Windows x64 (D-61, GUI since D-63).
//!
//! Single-file setup.exe with the payload embedded at compile time
//! (`WAKARU_PAYLOAD_DIR` must contain `WAKARU.exe`, `LICENSE` and
//! `FIRST_LAUNCH_WINDOWS.txt`). No admin rights, no third-party installer
//! framework, std-only (plus a few Win32 FFI declarations for locale
//! detection, UTF-8 console output, console attach and TaskDialog).
//!
//! Layout (per user):
//! - files: `%LocalAppData%\Programs\WAKARU\`
//! - Start Menu: `%AppData%\Microsoft\Windows\Start Menu\Programs\WAKARU.lnk`
//! - Add/Remove entry: `HKCU\...\Uninstall\WAKARU` (UninstallString runs the
//!   installed copy of this program with `--uninstall`)
//!
//! UX: double-click (no parent console) opens the native TaskDialog wizard
//! (`gui` module); CLI flags and non-interactive runs stay on the console.

// Release builds are GUI-first: without a console the TaskDialog wizard
// runs. Debug builds stay console-first so `cargo test` output is visible.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod gui;

use std::io::Write;
use std::path::{Path, PathBuf};

/// Keep in sync with the app version (see the release checklist in docs).
/// Sourced from Cargo.toml so `installer/Cargo.toml` is the single source.
const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "WAKARU";
const APP_ID: &str = "WAKARU";
const EXE_NAME: &str = "WAKARU.exe";
const LICENSE_NAME: &str = "LICENSE";
const FIRST_LAUNCH_NAME: &str = "FIRST_LAUNCH_WINDOWS.txt";
const VERSION_FILE: &str = "version.txt";
const UNINSTALL_EXE: &str = "uninstall.exe";

/// Embedded payload. These MUST be `static` (not `const`): a `const` byte
/// string is re-emitted per codegen unit and link.exe does not fold the
/// 57 MiB duplicates, doubling setup.exe (observed in v1.7.2 dev). A
/// `static` has one address shared by every use site (console + GUI).
static PAYLOAD_EXE: &[u8] = include_bytes!(concat!(env!("WAKARU_PAYLOAD_DIR"), "/WAKARU.exe"));
static PAYLOAD_LICENSE: &[u8] =
    include_bytes!(concat!(env!("WAKARU_PAYLOAD_DIR"), "/LICENSE"));
static PAYLOAD_FIRST_LAUNCH: &[u8] = include_bytes!(concat!(
    env!("WAKARU_PAYLOAD_DIR"),
    "/FIRST_LAUNCH_WINDOWS.txt"
));

/// Payload table for GUI detail text (name + bytes, no copies: the entries
/// borrow the `static` blobs above, so no additional bulk is emitted).
static PAYLOAD_BYTES: &[(&str, &[u8])] = &[
    (EXE_NAME, PAYLOAD_EXE),
    (LICENSE_NAME, PAYLOAD_LICENSE),
    (FIRST_LAUNCH_NAME, PAYLOAD_FIRST_LAUNCH),
];

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultLocaleName(locale_name: *mut u16, cch_locale_name: i32) -> i32;
    fn SetConsoleOutputCP(code_page_id: u32) -> i32;
    fn AttachConsole(process_id: u32) -> i32;
}

/// Pseudo-handle attaching to the parent process console.
const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;

/// Attach to the launching console when there is one (terminal runs keep
/// the classic console UX; double-clicks have none and fall to the GUI).
/// Never fails the caller: `false` simply means "no parent console".
fn attach_console() -> bool {
    // SAFETY: integer handle only, no pointers; failure is a normal
    // `0` return (no console to attach to).
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 }
}

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Ja,
    En,
}

fn detect_lang(force: Option<Lang>) -> Lang {
    if let Some(lang) = force {
        return lang;
    }
    let mut buf = [0u16; 85];
    // SAFETY: `buf` is a stack-local UTF-16 buffer with room for the
    // NUL terminator; the API writes at most `len` units on success.
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if len > 1 {
        let name = String::from_utf16_lossy(&buf[..(len as usize - 1)]);
        if name.to_ascii_lowercase().starts_with("ja") {
            return Lang::Ja;
        }
        return Lang::En;
    }
    Lang::Ja
}

/// Message catalog. Japanese first (product origin), English fallback.
fn msg(lang: Lang, key: &str) -> &str {
    match (lang, key) {
        (Lang::Ja, "plan") => "インストール先",
        (Lang::En, "plan") => "Install location",
        (Lang::Ja, "files") => "ファイル",
        (Lang::En, "files") => "Files",
        (Lang::Ja, "confirm") => "続行しますか？ [Y/n]: ",
        (Lang::En, "confirm") => "Continue? [Y/n]: ",
        (Lang::Ja, "cancelled") => "中止しました。",
        (Lang::En, "cancelled") => "Cancelled.",
        (Lang::Ja, "need_yes") => "非対話実行には --yes を指定してください。",
        (Lang::En, "need_yes") => "Pass --yes for non-interactive runs.",
        (Lang::Ja, "installed") => "インストールが完了しました。",
        (Lang::En, "installed") => "Installation complete.",
        (Lang::Ja, "how_to_start") => "スタートメニューの WAKARU から起動できます。",
        (Lang::En, "how_to_start") => "Launch WAKARU from the Start menu.",
        (Lang::Ja, "already") => "同じ版が既にインストールされています。",
        (Lang::En, "already") => "The same version is already installed.",
        (Lang::Ja, "upgrading") => "旧い版を上書きします。",
        (Lang::En, "upgrading") => "Overwriting the older installation.",
        (Lang::Ja, "close_app") => "WAKARU が起動中です。終了してから再実行してください。",
        (Lang::En, "close_app") => "WAKARU is running. Quit it and run again.",
        (Lang::Ja, "write_failed") => "書込に失敗しました",
        (Lang::En, "write_failed") => "Write failed",
        (Lang::Ja, "uninstall_confirm") => "を削除しますか？ [Y/n]: ",
        (Lang::En, "uninstall_confirm") => "Remove? [Y/n]: ",
        (Lang::Ja, "uninstalled") => "削除しました。",
        (Lang::En, "uninstalled") => "Uninstalled.",
        (Lang::Ja, "not_installed") => "インストールされていません。",
        (Lang::En, "not_installed") => "Not installed.",
        (Lang::Ja, "shortcut_warn") => {
            "注意: ショートカットの作成に失敗しました (手動で起動できます)。"
        }
        (Lang::En, "shortcut_warn") => {
            "Warning: shortcut creation failed (you can still launch manually)."
        }
        (Lang::Ja, "reg_failed") => "レジストリ登録に失敗しました",
        (Lang::En, "reg_failed") => "Registry registration failed",
        (Lang::Ja, "vcredist_warn") => "注意: Visual C++ 再頒布可能パッケージ (x64) が見つかりません。起動しない場合は https://aka.ms/vs/17/release/vc_redist.x64.exe から導入してください。",
        (Lang::En, "vcredist_warn") => "Warning: Visual C++ Redistributable (x64) not found. If the app does not start, install it from https://aka.ms/vs/17/release/vc_redist.x64.exe .",
        (Lang::Ja, "usage") => "使い方",
        (Lang::En, "usage") => "Usage",
        (Lang::Ja, "gui_install_main") => "WAKARU をインストールします",
        (Lang::En, "gui_install_main") => "Install WAKARU",
        (Lang::Ja, "gui_uninstall_main") => "WAKARU を削除します",
        (Lang::En, "gui_uninstall_main") => "Uninstall WAKARU",
        (Lang::Ja, "gui_install_btn") => "インストール",
        (Lang::En, "gui_install_btn") => "Install",
        (Lang::Ja, "gui_uninstall_btn") => "削除",
        (Lang::En, "gui_uninstall_btn") => "Uninstall",
        (Lang::Ja, "gui_reinstall_btn") => "再インストール",
        (Lang::En, "gui_reinstall_btn") => "Reinstall",
        (Lang::Ja, "gui_cancel_btn") => "キャンセル",
        (Lang::En, "gui_cancel_btn") => "Cancel",
        (Lang::Ja, "gui_desktop_check") => "デスクトップにショートカットを作成する",
        (Lang::En, "gui_desktop_check") => "Create a desktop shortcut",
        (Lang::Ja, "gui_show_details") => "詳細を表示",
        (Lang::En, "gui_show_details") => "Show details",
        (Lang::Ja, "gui_hide_details") => "詳細を隠す",
        (Lang::En, "gui_hide_details") => "Hide details",
        (Lang::Ja, "gui_failed") => "インストールに失敗しました",
        (Lang::En, "gui_failed") => "Installation failed",
        (Lang::Ja, "gui_uninstall_failed") => "削除に失敗しました",
        (Lang::En, "gui_uninstall_failed") => "Uninstall failed",
        _ => key,
    }
}

struct Options {
    uninstall: bool,
    install: bool,
    yes: bool,
    force: bool,
    no_shortcut: bool,
    desktop: bool,
    dir: Option<PathBuf>,
    lang: Option<Lang>,
    show_help: bool,
    show_version: bool,
    gui: bool,
    cli: bool,
}

fn parse_args() -> Result<Options, String> {
    parse_args_from(std::env::args_os().skip(1))
}

fn parse_args_from<I>(iter: I) -> Result<Options, String>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    let mut opts = Options {
        uninstall: false,
        install: false,
        yes: false,
        force: false,
        no_shortcut: false,
        desktop: false,
        dir: None,
        lang: None,
        show_help: false,
        show_version: false,
        gui: false,
        cli: false,
    };
    let mut args = iter.into_iter().peekable();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy().to_string();
        match text.as_str() {
            "--uninstall" => opts.uninstall = true,
            "--install" => opts.install = true,
            "--yes" | "-y" => opts.yes = true,
            "--force" => {
                opts.force = true;
                opts.yes = true;
            }
            "--no-shortcut" => opts.no_shortcut = true,
            "--desktop" => opts.desktop = true,
            "--gui" => opts.gui = true,
            "--cli" => opts.cli = true,
            "--english" => opts.lang = Some(Lang::En),
            "--japanese" => opts.lang = Some(Lang::Ja),
            "--help" | "-h" => opts.show_help = true,
            "--version" | "-V" => opts.show_version = true,
            "--dir" => {
                let value = args.next().ok_or("--dir needs a path")?;
                opts.dir = Some(PathBuf::from(value));
            }
            other if other.starts_with("--dir=") => {
                opts.dir = Some(PathBuf::from(&other["--dir=".len()..]));
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    if opts.no_shortcut && opts.desktop {
        return Err("--no-shortcut and --desktop conflict".into());
    }
    if opts.install && opts.uninstall {
        return Err("--install and --uninstall conflict".into());
    }
    if opts.gui && opts.cli {
        return Err("--gui and --cli conflict".into());
    }
    if opts.gui && (opts.yes || opts.force) {
        return Err("--gui is interactive; drop --yes/--force or use --cli".into());
    }
    // A copy named `uninstall.exe` (left by an install) uninstalls by
    // default; `--install` overrides back to install mode.
    if !opts.install && !opts.uninstall {
        let stem = std::env::args_os()
            .next()
            .map(|s| {
                Path::new(&s)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase()
            })
            .unwrap_or_default();
        if stem == "uninstall" {
            opts.uninstall = true;
        }
    }
    Ok(opts)
}

fn default_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
    Ok(PathBuf::from(local).join("Programs").join(APP_NAME))
}

fn confirm(lang: Lang, yes: bool, prompt: &str) -> Result<bool, String> {
    if yes {
        return Ok(true);
    }
    print!("{prompt}");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => Err(msg(lang, "need_yes").into()),
        Ok(_) => {
            let answer = line.trim().to_ascii_lowercase();
            Ok(answer.is_empty() || answer == "y" || answer == "yes")
        }
        Err(e) => Err(e.to_string()),
    }
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Create a `.lnk` via WScript.Shell (no new dependencies).
fn create_shortcut(lnk: &Path, target: &Path, workdir: &Path) -> Result<(), String> {
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut({});$s.TargetPath={};$s.WorkingDirectory={};$s.Description='{}';$s.Save()",
        powershell_quote(&lnk.to_string_lossy()),
        powershell_quote(&target.to_string_lossy()),
        powershell_quote(&workdir.to_string_lossy()),
        APP_NAME,
    );
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() && lnk.is_file() {
        Ok(())
    } else {
        Err(format!("powershell exit: {status}"))
    }
}

fn reg(args: &[&str]) -> Result<(), String> {
    let status = std::process::Command::new("reg.exe")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("reg.exe exit: {status}"))
    }
}

/// Clearing read-only is a Windows-only concern here (the unix
// world-writable caveat behind the clippy lint does not apply to this
// Windows-only installer).
#[allow(clippy::permissions_set_readonly_false)]
fn clear_readonly(path: &Path) {
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
}

#[allow(clippy::permissions_set_readonly_false)]
fn write_file(path: &Path, bytes: &[u8], lang: Lang) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Clear read-only leftovers from a previous install before overwriting.
    if path.is_file() {
        clear_readonly(path);
    }
    match std::fs::write(path, bytes) {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.raw_os_error() == Some(32) {
                return Err(msg(lang, "close_app").into());
            }
            Err(format!("{} {path:?}: {e}", msg(lang, "write_failed")))
        }
    }
}

fn arp_key() -> String {
    format!(r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{APP_ID}")
}

fn register_arp(dir: &Path, uninstall_exe: &Path, lang: Lang) -> Result<(), String> {
    let dir_text = dir.to_string_lossy().to_string();
    let uninst = format!("\"{}\" --uninstall", uninstall_exe.to_string_lossy());
    let icon = dir.join(EXE_NAME).to_string_lossy().to_string();
    let size_kb = dir_size_kb(dir);
    let key = arp_key();
    let set = |name: &str, ty: &str, data: &str| {
        reg(&["add", &key, "/v", name, "/t", ty, "/d", data, "/f"])
    };
    set("DisplayName", "REG_SZ", APP_NAME)
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("DisplayVersion", "REG_SZ", VERSION)
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("Publisher", "REG_SZ", "Yuki_Orita")
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("InstallLocation", "REG_SZ", &dir_text)
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("UninstallString", "REG_SZ", &uninst)
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("QuietUninstallString", "REG_SZ", &format!("{uninst} --yes"))
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("DisplayIcon", "REG_SZ", &icon).map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("EstimatedSize", "REG_DWORD", &size_kb.to_string())
        .map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("NoModify", "REG_DWORD", "1").map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    set("NoRepair", "REG_DWORD", "1").map_err(|e| format!("{}: {e}", msg(lang, "reg_failed")))?;
    Ok(())
}

fn dir_size_kb(dir: &Path) -> u32 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    (total / 1024).min(u32::MAX as u64) as u32
}

fn start_menu_lnk() -> Result<PathBuf, String> {
    let roaming = std::env::var_os("APPDATA").ok_or("APPDATA is not set")?;
    Ok(PathBuf::from(roaming)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join(format!("{APP_NAME}.lnk")))
}

fn desktop_lnk() -> Result<PathBuf, String> {
    // Respect folder redirection (OneDrive "Backup Desktop" moves the shell
    // folder out of %USERPROFILE%). Prefer the known redirected location
    // when it exists, otherwise fall back to %USERPROFILE%\Desktop.
    if let Some(onedrive) = std::env::var_os("OneDrive") {
        let candidate = PathBuf::from(onedrive).join("Desktop");
        if candidate.is_dir() {
            return Ok(candidate.join(format!("{APP_NAME}.lnk")));
        }
    }
    if let Some(commercial) = std::env::var_os("OneDriveCommercial") {
        let candidate = PathBuf::from(commercial).join("Desktop");
        if candidate.is_dir() {
            return Ok(candidate.join(format!("{APP_NAME}.lnk")));
        }
    }
    let profile = std::env::var_os("USERPROFILE").ok_or("USERPROFILE is not set")?;
    Ok(PathBuf::from(profile)
        .join("Desktop")
        .join(format!("{APP_NAME}.lnk")))
}

/// The app links the dynamic CRT (`MSVCP140.dll`), which is not part of a
/// bare Windows install. Warn (non-fatal) when it is absent so the user
/// gets an actionable message instead of a cryptic startup failure.
fn vcredist_present() -> bool {
    let system32 = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32");
    // VS 2015-2022 runtime ships vcruntime140.dll + vcruntime140_1.dll
    // (the latter holds `__CxxFrameHandler4` on x64). Both plus msvcp140
    // must exist for the app to start.
    system32.join("vcruntime140.dll").is_file()
        && system32.join("vcruntime140_1.dll").is_file()
        && system32.join("msvcp140.dll").is_file()
}

fn installed_version(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join(VERSION_FILE))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn resolve_install_dir(opts: &Options) -> Result<PathBuf, String> {
    match &opts.dir {
        Some(custom) => {
            if !custom.is_absolute() {
                return Err("--dir must be an absolute path".into());
            }
            Ok(custom.clone())
        }
        None => default_dir(),
    }
}

fn resolve_uninstall_dir(opts: &Options) -> Result<PathBuf, String> {
    let self_path =
        std::env::current_exe().map_err(|e| format!("cannot locate setup binary: {e}"))?;
    match &opts.dir {
        Some(custom) => {
            if !custom.is_absolute() {
                return Err("--dir must be an absolute path".into());
            }
            Ok(custom.clone())
        }
        // An `uninstall.exe` left by an install removes its own directory
        // unless `--dir` says otherwise.
        None if self_path
            .file_stem()
            .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case("uninstall"))
            && self_path.parent().is_some() =>
        {
            Ok(self_path.parent().unwrap().to_path_buf())
        }
        None => default_dir(),
    }
}

/// Shortcut outcome of a successful install (console prints it, GUI shows it).
struct InstallWarnings {
    shortcut_failed: Option<String>,
}

/// Shared install core: files, uninstaller copy, shortcuts, ARP.
/// UI-agnostic so console and GUI paths behave identically.
fn perform_install(
    dir: &Path,
    desktop: bool,
    no_shortcut: bool,
    lang: Lang,
) -> Result<InstallWarnings, String> {
    let exe_path = dir.join(EXE_NAME);
    write_file(&exe_path, PAYLOAD_EXE, lang)?;
    write_file(&dir.join(LICENSE_NAME), PAYLOAD_LICENSE, lang)?;
    write_file(&dir.join(FIRST_LAUNCH_NAME), PAYLOAD_FIRST_LAUNCH, lang)?;
    write_file(&dir.join(VERSION_FILE), VERSION.as_bytes(), lang)?;
    let self_path =
        std::env::current_exe().map_err(|e| format!("cannot locate setup binary: {e}"))?;
    let uninstall_path = dir.join(UNINSTALL_EXE);
    if self_path != uninstall_path {
        let bytes = std::fs::read(&self_path).map_err(|e| e.to_string())?;
        write_file(&uninstall_path, &bytes, lang)?;
    }

    let mut warnings = InstallWarnings {
        shortcut_failed: None,
    };
    if !no_shortcut {
        match start_menu_lnk() {
            Ok(lnk) => {
                if let Err(e) = create_shortcut(&lnk, &exe_path, dir) {
                    warnings.shortcut_failed = Some(e);
                }
            }
            Err(e) => warnings.shortcut_failed = Some(e),
        }
    }
    if desktop {
        match desktop_lnk() {
            Ok(lnk) => {
                if let Err(e) = create_shortcut(&lnk, &exe_path, dir) {
                    warnings.shortcut_failed = Some(e);
                }
            }
            Err(e) => warnings.shortcut_failed = Some(e),
        }
    }

    register_arp(dir, &uninstall_path, lang)?;
    Ok(warnings)
}

/// Shared uninstall core: lock pre-flight, files, shortcuts, ARP, self-copy
/// removal. UI-agnostic so console and GUI paths behave identically.
fn perform_uninstall(dir: &Path, lang: Lang) -> Result<(), String> {
    let self_path =
        std::env::current_exe().map_err(|e| format!("cannot locate setup binary: {e}"))?;
    let mut locked = false;
    // Pre-flight: a running app locks its exe. Fail before deleting
    // anything so a retry after quitting completes cleanly.
    {
        let exe = dir.join(EXE_NAME);
        if exe.is_file() {
            let probe = std::fs::OpenOptions::new().write(true).open(&exe);
            if let Err(e) = probe {
                if e.raw_os_error() == Some(32) {
                    return Err(msg(lang, "close_app").to_string());
                }
            }
        }
    }
    for name in [EXE_NAME, LICENSE_NAME, FIRST_LAUNCH_NAME, VERSION_FILE] {
        let path = dir.join(name);
        if path.is_file() {
            // Clear read-only so a deliberate removal never trips on it
            // (a locked running exe still fails below with error 32).
            clear_readonly(&path);
            if std::fs::remove_file(&path).is_err() {
                locked = true;
            }
        }
    }
    if locked {
        return Err(msg(lang, "close_app").to_string());
    }
    if let Ok(lnk) = start_menu_lnk() {
        remove_if_exists(&lnk);
    }
    if let Ok(lnk) = desktop_lnk() {
        remove_if_exists(&lnk);
    }
    let _ = reg(&["delete", &arp_key(), "/f"]);

    // Remove the uninstaller copy (or ourselves when run as setup --uninstall).
    let uninstall_path = dir.join(UNINSTALL_EXE);
    if self_path == uninstall_path || path_starts_with_insensitive(&self_path, dir) {
        schedule_self_delete(&self_path, dir);
    } else {
        remove_if_exists(&uninstall_path);
        let _ = std::fs::remove_dir(dir);
    }
    Ok(())
}

fn do_install(opts: &Options, lang: Lang) -> Result<(), String> {
    let dir = resolve_install_dir(opts)?;
    if let Some(current) = installed_version(&dir) {
        if current == VERSION && !opts.force {
            println!("{} ({})", msg(lang, "already"), current);
            return Ok(());
        }
        println!("{}", msg(lang, "upgrading"));
    }
    println!("{}: {}", msg(lang, "plan"), dir.to_string_lossy());
    println!(
        "{}: {EXE_NAME}, {LICENSE_NAME}, {FIRST_LAUNCH_NAME}",
        msg(lang, "files")
    );
    if !vcredist_present() {
        eprintln!("{}", msg(lang, "vcredist_warn"));
    }
    if !confirm(lang, opts.yes, msg(lang, "confirm"))? {
        println!("{}", msg(lang, "cancelled"));
        return Ok(());
    }

    let warnings = perform_install(&dir, opts.desktop, opts.no_shortcut, lang)?;
    if let Some(e) = warnings.shortcut_failed {
        eprintln!("{} ({e})", msg(lang, "shortcut_warn"));
    }

    println!("{} ({VERSION})", msg(lang, "installed"));
    println!("{}", msg(lang, "how_to_start"));
    Ok(())
}

fn remove_if_exists(path: &Path) {
    if path.is_symlink() || path.is_file() {
        clear_readonly(path);
        let _ = std::fs::remove_file(path);
    }
}

/// Windows paths are case-insensitive; `Path::starts_with` is case-sensitive,
/// so compare component-wise in lowercase for the self-delete decision.
fn path_starts_with_insensitive(path: &Path, base: &Path) -> bool {
    let mut path_parts = path.components();
    let mut base_parts = base.components();
    loop {
        match (path_parts.next(), base_parts.next()) {
            (_, None) => return true,
            (Some(p), Some(b)) => {
                if p.as_os_str().to_string_lossy().to_lowercase()
                    != b.as_os_str().to_string_lossy().to_lowercase()
                {
                    return false;
                }
            }
            (None, Some(_)) => return false,
        }
    }
}

/// Schedule deletion of a locked file (typically ourselves) via a detached
/// PowerShell that runs after we exit, then try to remove the directory.
fn schedule_self_delete(target: &Path, dir: &Path) {
    let script = format!(
        "Start-Sleep -Seconds 2; Remove-Item -LiteralPath {} -Force; Remove-Item -LiteralPath {} -Recurse -Force -ErrorAction SilentlyContinue",
        powershell_quote(&target.to_string_lossy()),
        powershell_quote(&dir.to_string_lossy())
    );
    let _ = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn do_uninstall(opts: &Options, lang: Lang) -> Result<(), String> {
    let dir = resolve_uninstall_dir(opts)?;
    if installed_version(&dir).is_none() && !dir.join(EXE_NAME).is_file() {
        println!("{}", msg(lang, "not_installed"));
        return Ok(());
    }
    if !confirm(
        lang,
        opts.yes,
        &format!(
            "{}{}",
            dir.to_string_lossy(),
            msg(lang, "uninstall_confirm")
        ),
    )? {
        println!("{}", msg(lang, "cancelled"));
        return Ok(());
    }

    perform_uninstall(&dir, lang)?;
    println!("{}", msg(lang, "uninstalled"));
    Ok(())
}

fn print_help(lang: Lang) {
    let exe = std::env::args_os()
        .next()
        .map(|s| {
            Path::new(&s)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|| "setup.exe".into());
    println!("WAKARU setup {VERSION} — {}", msg(lang, "usage"));
    println!("  Double-click: GUI wizard (install / uninstall dialogs).");
    println!(
        "  {exe} [--install] [--yes] [--force] [--dir <abs-path>] [--no-shortcut] [--desktop] [--cli]"
    );
    println!("  {exe} --uninstall [--yes] [--dir <abs-path>] [--cli]");
    println!("  {exe} --gui [--install|--uninstall] [--dir <abs-path>] [--desktop|--no-shortcut]");
    println!("  {exe} --help | --version [--english|--japanese]");
}

fn main() {
    let opts = match parse_args() {
        Ok(opts) => opts,
        Err(e) => {
            // No console when double-clicked: show the error graphically.
            if !attach_console() {
                gui::fatal_box(&format!("error: {e}"));
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(1);
        }
    };
    let lang = detect_lang(opts.lang);
    if opts.show_help || opts.show_version {
        // Help/version are console flows: attach when launched from a shell.
        attach_console();
        // SAFETY: process-wide console code page switch, no pointers involved.
        unsafe {
            SetConsoleOutputCP(65001);
        }
        if opts.show_help {
            print_help(lang);
        } else {
            println!("WAKARU setup {VERSION}");
        }
        return;
    }
    if gui::decide_mode(&opts, attach_console()) == gui::UiMode::Gui {
        // GUI wizards report errors through dialogs; cancel exits 0.
        let failed = if opts.uninstall {
            gui::run_uninstall(&opts, lang).is_err()
        } else {
            gui::run_install(&opts, lang).is_err()
        };
        if failed {
            std::process::exit(1);
        }
        return;
    }
    // SAFETY: process-wide console code page switch, no pointers involved.
    unsafe {
        SetConsoleOutputCP(65001);
    }
    let result = if opts.uninstall {
        do_uninstall(&opts, lang)
    } else {
        do_install(&opts, lang)
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn args(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    #[test]
    fn defaults_install_without_flags() {
        let opts = parse_args_from(args(&[])).unwrap();
        assert!(!opts.uninstall && !opts.yes && !opts.force && opts.dir.is_none());
    }

    #[test]
    fn force_implies_yes() {
        let opts = parse_args_from(args(&["--force"])).unwrap();
        assert!(opts.force && opts.yes);
    }

    #[test]
    fn dir_forms_and_unknown_are_checked() {
        let opts = parse_args_from(args(&["--dir", r"C:\App"])).unwrap();
        assert_eq!(opts.dir, Some(PathBuf::from(r"C:\App")));
        let opts = parse_args_from(args(&["--dir=C:\\App"])).unwrap();
        assert_eq!(opts.dir, Some(PathBuf::from(r"C:\App")));
        assert!(parse_args_from(args(&["--dir"])).is_err());
        assert!(parse_args_from(args(&["--badopt"])).is_err());
        assert!(parse_args_from(args(&["--no-shortcut", "--desktop"])).is_err());
        assert!(parse_args_from(args(&["--install", "--uninstall"])).is_err());
        assert!(parse_args_from(args(&["--gui", "--cli"])).is_err());
        assert!(parse_args_from(args(&["--gui", "--yes"])).is_err());
        assert!(parse_args_from(args(&["--gui", "--force"])).is_err());
        let opts = parse_args_from(args(&["--gui", "--dir", r"C:\App"])).unwrap();
        assert!(opts.gui && !opts.cli);
        let opts = parse_args_from(args(&["--cli"])).unwrap();
        assert!(opts.cli && !opts.gui);
    }

    #[test]
    fn catalog_covers_both_languages() {
        for key in [
            "plan",
            "files",
            "confirm",
            "cancelled",
            "need_yes",
            "installed",
            "how_to_start",
            "already",
            "upgrading",
            "close_app",
            "write_failed",
            "uninstall_confirm",
            "uninstalled",
            "not_installed",
            "shortcut_warn",
            "reg_failed",
            "usage",
            "vcredist_warn",
            "gui_install_main",
            "gui_uninstall_main",
            "gui_install_btn",
            "gui_uninstall_btn",
            "gui_reinstall_btn",
            "gui_cancel_btn",
            "gui_desktop_check",
            "gui_show_details",
            "gui_hide_details",
            "gui_failed",
            "gui_uninstall_failed",
        ] {
            assert_ne!(msg(Lang::Ja, key), key, "missing ja {key}");
            assert_ne!(msg(Lang::En, key), key, "missing en {key}");
        }
    }

    #[test]
    fn powershell_quote_doubles_single_quotes() {
        assert_eq!(powershell_quote("plain"), "'plain'");
        assert_eq!(powershell_quote("a'b"), "'a''b'");
    }

    #[test]
    fn path_prefix_match_is_case_insensitive() {
        let dir = PathBuf::from(r"C:\Users\Test\AppData\Local\Programs\WAKARU");
        let child = PathBuf::from(r"c:\users\test\appdata\local\programs\wakaru\uninstall.exe");
        assert!(path_starts_with_insensitive(&child, &dir));
        let other = PathBuf::from(r"C:\Users\Test\AppData\Local\Programs\Other\app.exe");
        assert!(!path_starts_with_insensitive(&other, &dir));
        assert!(!path_starts_with_insensitive(
            &PathBuf::from(r"C:\Short"),
            &dir
        ));
    }

    #[test]
    fn version_matches_package() {
        // Single source of truth: installer Cargo.toml version.
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!VERSION.is_empty());
    }
}
