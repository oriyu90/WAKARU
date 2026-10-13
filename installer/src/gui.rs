//! Native GUI for setup.exe (D-63).
//!
//! `TaskDialogIndirect` from comctl32 v6 — no new dependencies, std + FFI
//! only. The installer stays a single file with no admin rights.
//!
//! Mode rule (compat-first): `--help` / `--version` / `--yes` / `--force` /
//! `--cli` always use the console; `--gui` always uses dialogs; otherwise a
//! parent console means console UX (as before) and no console means the GUI
//! wizard (double-click, ARP Uninstall). All strings come from the shared
//! JA/EN `msg` catalog so both languages are fully served.

use super::{msg, Lang, Options};

#[link(name = "comctl32")]
unsafe extern "system" {
    fn TaskDialogIndirect(
        task_config: *const TaskDialogConfig,
        button: *mut i32,
        radio_button: *mut i32,
        verification_flag_checked: *mut i32,
    ) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(
        wnd: *mut core::ffi::c_void,
        text: *const u16,
        caption: *const u16,
        kind: u32,
    ) -> i32;
}

// Button IDs and flags (Win32 SDK values, stable since Vista).
const IDCANCEL: i32 = 2;
const IDOK: i32 = 1;
const BTN_PRIMARY: i32 = 100;
const BTN_REINSTALL: i32 = 102;
const TDF_ALLOW_DIALOG_CANCELLATION: u32 = 0x0008;
const TDF_USE_COMMAND_LINKS: u32 = 0x0010;
const TD_INFORMATION_ICON: *const u16 = 0xFFFD as *const u16;
const TD_WARNING_ICON: *const u16 = 0xFFFF as *const u16;
const TD_ERROR_ICON: *const u16 = 0xFFFE as *const u16;
const MB_OK: u32 = 0;
const MB_OKCANCEL: u32 = 1;
const MB_ICONERROR: u32 = 0x10;
const MB_ICONINFORMATION: u32 = 0x40;
const TDF_VERIFICATION_FLAG_CHECKED: u32 = 0x0100;

#[repr(C)]
struct TaskDialogButton {
    id: i32,
    text: *const u16,
}

#[repr(C)]
#[allow(clippy::struct_field_names)]
struct TaskDialogConfig {
    cb_size: u32,
    hwnd_parent: *mut core::ffi::c_void,
    h_instance: *mut core::ffi::c_void,
    dw_flags: u32,
    dw_common_buttons: u32,
    window_title: *const u16,
    main_icon: *const u16,
    main_instruction: *const u16,
    content: *const u16,
    c_buttons: u32,
    p_buttons: *const TaskDialogButton,
    n_default_button: i32,
    c_radio_buttons: u32,
    p_radio_buttons: *const TaskDialogButton,
    n_default_radio_button: i32,
    verification_text: *const u16,
    expanded_information: *const u16,
    expanded_control_text: *const u16,
    collapsed_control_text: *const u16,
    footer_icon: *const u16,
    footer: *const u16,
    pf_callback: *mut core::ffi::c_void,
    lp_callback_data: isize,
    cx_width: u32,
}

#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<TaskDialogConfig>() == 176);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<TaskDialogButton>() == 16);

/// UI mode decision (pure: unit-tested). CLI flags and non-interactive runs
/// stay on the console; `--gui` forces dialogs; otherwise the presence of a
/// parent console decides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiMode {
    Console,
    Gui,
}

pub fn decide_mode(opts: &Options, parent_console: bool) -> UiMode {
    if opts.cli || opts.show_help || opts.show_version || opts.yes || opts.force {
        UiMode::Console
    } else if opts.gui {
        UiMode::Gui
    } else if parent_console {
        UiMode::Console
    } else {
        UiMode::Gui
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Dialog parameters (grouped so the show function stays lint-clean).
pub struct DialogSpec<'a> {
    pub title: &'a str,
    pub main: &'a str,
    pub content: &'a str,
    pub expanded: Option<(&'a str, &'a str, &'a str)>,
    pub verification: Option<(&'a str, bool)>,
    pub buttons: &'a [(i32, &'a str)],
    pub default_id: i32,
    pub icon: *const u16,
    pub command_links: bool,
}

/// Show one TaskDialog. Returns (pressed button id, verification checked).
/// `IDCANCEL` covers close-box / Esc cancellations.
fn show_dialog(spec: &DialogSpec) -> Result<(i32, bool), String> {
    let title = wide(spec.title);
    let main = wide(spec.main);
    let content = wide(spec.content);
    let texts: Vec<Vec<u16>> = spec.buttons.iter().map(|(_, s)| wide(s)).collect();
    let buttons: Vec<TaskDialogButton> = spec
        .buttons
        .iter()
        .zip(texts.iter())
        .map(|((id, _), w)| TaskDialogButton {
            id: *id,
            text: w.as_ptr(),
        })
        .collect();
    let (collapsed, expanded_label, info) = match &spec.expanded {
        Some((c, e, i)) => (Some(wide(c)), Some(wide(e)), Some(wide(i))),
        None => (None, None, None),
    };
    let verification = spec.verification.map(|(v, _)| wide(v));
    // SAFETY: every pointer borrows a local buffer that outlives this call;
    // parent/instance are NULL, radio buttons unused, callback NULL. The
    // struct layout is asserted `#[repr(C)]` above (176 bytes on x86-64).
    let mut config: TaskDialogConfig = unsafe { std::mem::zeroed() };
    config.cb_size = std::mem::size_of::<TaskDialogConfig>() as u32;
    config.dw_flags = TDF_ALLOW_DIALOG_CANCELLATION
        | if spec.command_links {
            TDF_USE_COMMAND_LINKS
        } else {
            0
        };
    config.window_title = title.as_ptr();
    config.main_icon = spec.icon;
    config.main_instruction = main.as_ptr();
    config.content = content.as_ptr();
    config.c_buttons = buttons.len() as u32;
    config.p_buttons = buttons.as_ptr();
    config.n_default_button = spec.default_id;
    if let Some(w) = &verification {
        config.verification_text = w.as_ptr();
        if spec.verification.is_some_and(|(_, checked)| checked) {
            config.dw_flags |= TDF_VERIFICATION_FLAG_CHECKED;
        }
    }
    if let (Some(c), Some(e), Some(i)) = (&collapsed, &expanded_label, &info) {
        config.collapsed_control_text = c.as_ptr();
        config.expanded_control_text = e.as_ptr();
        config.expanded_information = i.as_ptr();
    }
    let mut pressed = 0i32;
    let mut radio = 0i32;
    let mut checked = 0i32;
    let code = unsafe { TaskDialogIndirect(&config, &mut pressed, &mut radio, &mut checked) };
    if code == 0 {
        Ok((pressed, checked != 0))
    } else {
        Err(format!("TaskDialog failed: HRESULT {code:#X}"))
    }
}

fn info_box(title: &str, text: &str, error: bool) {
    let caption = wide(title);
    let body = wide(text);
    // SAFETY: both buffers outlive the modal call; owner NULL is valid.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            caption.as_ptr(),
            if error {
                MB_OK | MB_ICONERROR
            } else {
                MB_OK | MB_ICONINFORMATION
            },
        );
    }
}

fn confirm_box(title: &str, text: &str) -> bool {
    let caption = wide(title);
    let body = wide(text);
    // SAFETY: both buffers outlive the modal call; owner NULL is valid.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            caption.as_ptr(),
            MB_OKCANCEL | MB_ICONINFORMATION,
        )
    };
    result == IDOK
}

/// Last-resort error display when even the dialog path is unusable
/// (e.g. argument parse failure with no console attached).
pub fn fatal_box(text: &str) {
    info_box(
        &format!("{} setup {}", super::APP_NAME, super::VERSION),
        text,
        true,
    );
}

/// Confirm dialog with a TaskDialog → MessageBox fallback. Returns
/// (confirmed by the primary button, verification checkbox state).
fn ask_confirm(spec: &DialogSpec) -> Result<(bool, bool), String> {
    match show_dialog(spec) {
        Ok((id, checked)) => Ok((id == BTN_PRIMARY || id == BTN_REINSTALL, checked)),
        Err(_) => Ok((
            confirm_box(spec.title, &format!("{}\n\n{}", spec.main, spec.content)),
            false,
        )),
    }
}

fn show_info(title: &str, main: &str, content: &str, error: bool) {
    let spec = DialogSpec {
        title,
        main,
        content,
        expanded: None,
        verification: None,
        buttons: &[(IDOK, "OK")],
        default_id: IDOK,
        icon: if error {
            TD_ERROR_ICON
        } else {
            TD_INFORMATION_ICON
        },
        command_links: false,
    };
    if show_dialog(&spec).is_err() {
        info_box(title, &format!("{main}\n\n{content}"), error);
    }
}

/// Install confirmation body (pure: unit-tested for both languages).
pub fn install_content(lang: Lang, dir: &str, upgrading: bool, vcredist_ok: bool) -> String {
    let mut body = format!(
        "{}: {dir}\n{}: {}, {}, {}",
        msg(lang, "plan"),
        msg(lang, "files"),
        super::EXE_NAME,
        super::LICENSE_NAME,
        super::FIRST_LAUNCH_NAME,
    );
    if upgrading {
        body.push_str(&format!("\n{}", msg(lang, "upgrading")));
    }
    if !vcredist_ok {
        body.push_str(&format!("\n{}", msg(lang, "vcredist_warn")));
    }
    body
}

/// Uninstall confirmation body (pure: unit-tested for both languages).
pub fn uninstall_content(lang: Lang, dir: &str) -> String {
    format!("{dir}{}", msg(lang, "uninstall_confirm"))
}

/// GUI install wizard: confirm → shared `perform_install` core → result.
/// Returns `Ok(false)` when the user cancelled (not an error).
pub fn run_install(opts: &Options, lang: Lang) -> Result<bool, String> {
    let title = format!("{} setup {}", super::APP_NAME, super::VERSION);
    let dir = match super::resolve_install_dir(opts) {
        Ok(dir) => dir,
        Err(e) => {
            show_info(&title, msg(lang, "gui_failed"), &e, true);
            return Err(e);
        }
    };
    let dir_text = dir.to_string_lossy().to_string();
    let already_same =
        super::installed_version(&dir).is_some_and(|current| current == super::VERSION);
    let mut force = opts.force;
    if already_same && !force {
        let spec = DialogSpec {
            title: &title,
            main: msg(lang, "already"),
            content: super::VERSION,
            expanded: None,
            verification: None,
            buttons: &[
                (BTN_REINSTALL, msg(lang, "gui_reinstall_btn")),
                (IDCANCEL, msg(lang, "gui_cancel_btn")),
            ],
            default_id: IDCANCEL,
            icon: TD_INFORMATION_ICON,
            command_links: true,
        };
        match show_dialog(&spec) {
            Ok((BTN_REINSTALL, _)) => force = true,
            Ok(_) => return Ok(false),
            Err(_) => {
                if !confirm_box(
                    &title,
                    &format!("{}\n\n{}", msg(lang, "already"), super::VERSION),
                ) {
                    return Ok(false);
                }
                force = true;
            }
        }
    }
    let upgrading =
        (super::installed_version(&dir).is_some() && !already_same) || (force && already_same);
    let content = install_content(lang, &dir_text, upgrading, super::vcredist_present());
    let details = format!(
        "{}: {}\n{}",
        msg(lang, "files"),
        super::PAYLOAD_BYTES
            .iter()
            .map(|(name, bytes)| format!("{name} ({} bytes)", bytes.len()))
            .collect::<Vec<_>>()
            .join(", "),
        msg(lang, "how_to_start"),
    );
    let spec = DialogSpec {
        title: &title,
        main: msg(lang, "gui_install_main"),
        content: &content,
        expanded: Some((
            msg(lang, "gui_show_details"),
            msg(lang, "gui_hide_details"),
            &details,
        )),
        verification: if opts.no_shortcut {
            None
        } else {
            Some((msg(lang, "gui_desktop_check"), opts.desktop))
        },
        buttons: &[
            (BTN_PRIMARY, msg(lang, "gui_install_btn")),
            (IDCANCEL, msg(lang, "gui_cancel_btn")),
        ],
        default_id: BTN_PRIMARY,
        icon: TD_INFORMATION_ICON,
        command_links: true,
    };
    // The verification checkbox is read from the confirm dialog result.
    let (confirmed, desktop_checked) = ask_confirm(&spec)?;
    if !confirmed {
        return Ok(false);
    }
    let title = format!("{} setup {}", super::APP_NAME, super::VERSION);
    match super::perform_install(
        &dir,
        desktop_checked || opts.desktop,
        opts.no_shortcut,
        lang,
    ) {
        Ok(warnings) => {
            let mut content = format!("{} ({})", msg(lang, "installed"), super::VERSION);
            content.push_str(&format!("\n{}", msg(lang, "how_to_start")));
            if let Some(warn) = warnings.shortcut_failed {
                content.push_str(&format!("\n{} ({warn})", msg(lang, "shortcut_warn")));
            }
            show_info(
                &title,
                &format!("{} ({})", msg(lang, "installed"), super::VERSION),
                &content,
                false,
            );
            Ok(true)
        }
        Err(e) => {
            show_info(&title, msg(lang, "gui_failed"), &e, true);
            Err(e)
        }
    }
}

/// GUI uninstall wizard: confirm → shared `perform_uninstall` core → result.
pub fn run_uninstall(opts: &Options, lang: Lang) -> Result<bool, String> {
    let title = format!("{} setup {}", super::APP_NAME, super::VERSION);
    let dir = match super::resolve_uninstall_dir(opts) {
        Ok(dir) => dir,
        Err(e) => {
            show_info(&title, msg(lang, "gui_uninstall_failed"), &e, true);
            return Err(e);
        }
    };
    let dir_text = dir.to_string_lossy().to_string();
    if super::installed_version(&dir).is_none() && !dir.join(super::EXE_NAME).is_file() {
        show_info(&title, msg(lang, "not_installed"), &dir_text, false);
        return Ok(false);
    }
    let content = uninstall_content(lang, &dir_text);
    let spec = DialogSpec {
        title: &title,
        main: msg(lang, "gui_uninstall_main"),
        content: &content,
        expanded: None,
        verification: None,
        buttons: &[
            (BTN_PRIMARY, msg(lang, "gui_uninstall_btn")),
            (IDCANCEL, msg(lang, "gui_cancel_btn")),
        ],
        default_id: IDCANCEL,
        icon: TD_WARNING_ICON,
        command_links: true,
    };
    if !ask_confirm(&spec)?.0 {
        return Ok(false);
    }
    match super::perform_uninstall(&dir, lang) {
        Ok(()) => {
            show_info(&title, msg(lang, "uninstalled"), &dir_text, false);
            Ok(true)
        }
        Err(e) => {
            show_info(&title, msg(lang, "gui_uninstall_failed"), &e, true);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn empty_opts() -> Options {
        crate::parse_args_from(Vec::<OsString>::new()).unwrap()
    }

    #[test]
    fn mode_decision_prefers_console_for_automation() {
        let mut opts = empty_opts();
        // No console (double-click) with no flags -> GUI.
        assert_eq!(decide_mode(&opts, false), UiMode::Gui);
        // Launched from a console -> console UX as before.
        assert_eq!(decide_mode(&opts, true), UiMode::Console);
        opts.yes = true;
        assert_eq!(decide_mode(&opts, false), UiMode::Console);
        opts.yes = false;
        opts.force = true;
        assert_eq!(decide_mode(&opts, false), UiMode::Console);
        opts.force = false;
        opts.gui = true;
        assert_eq!(decide_mode(&opts, true), UiMode::Gui);
        opts.gui = false;
        opts.cli = true;
        assert_eq!(decide_mode(&opts, false), UiMode::Console);
        opts.cli = false;
        opts.show_help = true;
        assert_eq!(decide_mode(&opts, false), UiMode::Console);
    }

    #[test]
    fn wide_strings_are_nul_terminated_utf16() {
        let w = wide("あWAKARU");
        assert_eq!(*w.last().unwrap(), 0);
        assert_eq!(String::from_utf16_lossy(&w[..w.len() - 1]), "あWAKARU");
    }

    #[test]
    fn install_content_covers_both_languages() {
        for lang in [Lang::Ja, Lang::En] {
            let body = install_content(lang, r"C:\App\WAKARU", true, false);
            assert!(body.contains(r"C:\App\WAKARU"), "dir missing: {body}");
            assert!(body.contains(msg(lang, "plan")), "plan missing");
            assert!(body.contains(msg(lang, "files")), "files missing");
            assert!(
                body.contains(msg(lang, "upgrading")),
                "upgrade note missing"
            );
            assert!(
                body.contains(msg(lang, "vcredist_warn")),
                "vcredist note missing"
            );
            let plain = install_content(lang, r"C:\App", false, true);
            assert!(!plain.contains(msg(lang, "upgrading")));
        }
    }

    #[test]
    fn uninstall_content_covers_both_languages() {
        for lang in [Lang::Ja, Lang::En] {
            let body = uninstall_content(lang, r"C:\App\WAKARU");
            assert!(body.contains(r"C:\App\WAKARU"), "dir missing: {body}");
            assert!(
                body.contains(msg(lang, "uninstall_confirm")),
                "confirm missing"
            );
        }
    }
}
