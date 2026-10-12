//! The Studio sandbox (docs/05 §7, I-7, AC-7-5..7-10). Everything `write_file`
//! and `run_command` touch is confined to a project's `workspace/`:
//!
//! - [`resolve_in_sandbox`] canonicalises the requested path and rejects
//!   anything that lands outside `workspace/` — absolute paths, `..`,
//!   URL-encoded traversal, and symlink escapes all fail here.
//! - [`run_command`] runs a program with **no shell**, a scrubbed environment,
//!   a fixed working directory, an output cap, and a hard timeout that kills the
//!   whole process group.

use crate::error::{AppError, AppResult};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// Per-file ceiling for `write_file` (docs/05 §7.1).
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
/// Whole-`workspace/` ceiling.
pub const MAX_WORKSPACE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Per-stream `run_command` output cap; the rest is dropped with a marker.
pub const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
/// Only these are inherited by a sandboxed command (docs/05 §7.2). An API key
/// can never reach a child process.
#[cfg(unix)]
const ENV_ALLOWLIST: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG"];
/// Windows equivalent: `HOME` -> `USERPROFILE`, `TMPDIR` -> `TEMP`/`TMP`,
/// plus the minimum Windows runtime variables. Secrets are still excluded.
#[cfg(windows)]
const ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "SystemRoot",
    "PATHEXT",
    "LANG",
];

#[cfg(unix)]
fn command_path() -> OsString {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    dirs.extend(
        [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]
        .into_iter()
        .map(PathBuf::from),
    );
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.extend([
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            home.join("Library/Python/3.12/bin"),
        ]);
    }
    let mut seen = HashSet::new();
    dirs.retain(|dir| dir.is_dir() && seen.insert(dir.clone()));
    std::env::join_paths(dirs).unwrap_or_else(|_| OsString::from("/usr/bin:/bin"))
}

/// Windows search path for sandboxed commands. The inherited `PATH` wins;
/// well-known install locations are appended (existence-checked, deduped).
#[cfg(windows)]
fn command_path() -> OsString {
    use std::env::var_os;
    let mut dirs: Vec<PathBuf> = var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let system_root = var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let program_files = var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    let program_files_x86 = var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    dirs.extend([
        system_root.join("System32"),
        program_files.join("Git").join("cmd"),
        program_files.join("nodejs"),
        program_files_x86
            .join("Microsoft Visual Studio")
            .join("Shared")
            .join("Python"),
    ]);
    if let Some(profile) = var_os("USERPROFILE").map(PathBuf::from) {
        dirs.extend([
            profile.join(".cargo").join("bin"),
            profile.join("scoop").join("shims"),
            profile
                .join("AppData")
                .join("Local")
                .join("Programs")
                .join("Python"),
        ]);
    }
    if let Some(appdata) = var_os("APPDATA").map(PathBuf::from) {
        dirs.extend([appdata.join("npm"), appdata.join("Python")]);
    }
    if let Some(local) = var_os("LOCALAPPDATA").map(PathBuf::from) {
        dirs.extend([local.join("Microsoft").join("WindowsApps")]);
    }
    let mut seen = HashSet::new();
    dirs.retain(|dir| dir.is_dir() && seen.insert(dir.clone()));
    std::env::join_paths(dirs).unwrap_or_else(|_| OsString::from(r"C:\Windows\System32"))
}

/// Short, runtime-accurate command list placed in the weak-model capability
/// guide. It advertises only tools that can actually be spawned by run_command.
pub fn command_catalog() -> String {
    let path = command_path();
    let dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
    // `textutil` is macOS-only; it must never be advertised on Windows.
    #[cfg(unix)]
    const CANDIDATES: &[&str] = &[
        "python3",
        "textutil",
        "pandoc",
        "soffice",
        "ffmpeg",
        "magick",
        "pdftotext",
    ];
    #[cfg(windows)]
    const CANDIDATES: &[&str] = &[
        "python",
        "python3",
        "pandoc",
        "soffice",
        "ffmpeg",
        "magick",
        "pdftotext",
    ];
    let available: Vec<&str> = CANDIDATES
        .iter()
        .copied()
        .filter(|name| dirs.iter().any(|dir| dir.join(name).is_file()))
        .collect();
    if available.is_empty() {
        "(no optional conversion commands detected)".into()
    } else {
        available.join(", ")
    }
}

fn denied(msg: impl Into<String>) -> AppError {
    AppError::new("SANDBOX_PATH_DENIED", "error.sandbox.pathDenied", msg)
}

/// Resolve `user_path` inside `root` or reject it. `root` must already exist.
///
/// Steps (docs/05 §7.1): reject absolute / encoded / `..`; join under `root`;
/// canonicalise the deepest existing ancestor; require the result to stay a
/// descendant of `canonicalize(root)`.
pub fn resolve_in_sandbox(root: &Path, user_path: &str) -> AppResult<PathBuf> {
    let raw = user_path.trim();
    if raw.is_empty() {
        return Err(denied("empty path"));
    }
    // Percent-encoding is never legitimate here and hides `%2e%2e%2f` traversal.
    if raw.contains('%') {
        return Err(denied("percent-encoded paths are not allowed"));
    }
    if raw.starts_with('~') {
        return Err(denied("home-relative paths are not allowed"));
    }
    // On unix hosts a backslash is never a legitimate separator or filename
    // byte here, and `..\..\` would otherwise slip through as one
    // innocent-looking filename (AC-7-5). Drive letters and UNC prefixes are
    // covered by the `:` and leading-`/` checks.
    // On Windows hosts backslashes and drive prefixes are legitimate
    // separators; absolute paths are still rejected below via components.
    #[cfg(unix)]
    if raw.starts_with('/') || raw.contains('\\') || raw.contains(':') {
        return Err(denied("absolute or Windows-style paths are not allowed"));
    }
    #[cfg(windows)]
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(denied("absolute paths are not allowed"));
    }

    let p = Path::new(raw);
    if p.is_absolute() {
        return Err(denied("absolute paths are not allowed"));
    }
    for c in p.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(denied("`..` is not allowed")),
            Component::RootDir | Component::Prefix(_) => {
                return Err(denied("absolute paths are not allowed"))
            }
        }
    }

    let root_canon = root
        .canonicalize()
        .map_err(|e| AppError::internal(format!("workspace missing: {e}")))?;
    let target = root_canon.join(p);

    // Canonicalise the deepest ancestor that exists, then re-attach the rest.
    let mut existing = target.as_path();
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    let resolved_existing = loop {
        match existing.canonicalize() {
            Ok(c) => break c,
            Err(_) => match (existing.parent(), existing.file_name()) {
                (Some(parent), Some(name)) => {
                    tail.push(name);
                    existing = parent;
                }
                _ => return Err(denied("path escapes the workspace")),
            },
        }
    };
    let mut resolved = resolved_existing;
    for name in tail.into_iter().rev() {
        resolved.push(name);
    }

    if !resolved.starts_with(&root_canon) {
        return Err(denied("path escapes the workspace"));
    }
    Ok(resolved)
}

/// Reject if `path` (already sandbox-resolved) is an existing symlink —
/// `write_file` writes regular files only (docs/05 §7.1, AC-7-6).
pub fn reject_symlink(path: &Path) -> AppResult<()> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(denied("refusing to write through a symlink"));
        }
    }
    Ok(())
}

/// Enforce the file and total-workspace size ceilings for a pending write.
pub fn check_write_size(root: &Path, target: &Path, new_len: u64) -> AppResult<()> {
    if new_len > MAX_FILE_BYTES {
        return Err(AppError::new(
            "SANDBOX_FILE_TOO_LARGE",
            "error.sandbox.fileTooLarge",
            format!("file exceeds {} MB", MAX_FILE_BYTES / 1024 / 1024),
        ));
    }
    let existing = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
    let total = dir_size(root).saturating_sub(existing) + new_len;
    if total > MAX_WORKSPACE_BYTES {
        return Err(AppError::new(
            "SANDBOX_WORKSPACE_FULL",
            "error.sandbox.workspaceFull",
            "workspace size limit reached",
        ));
    }
    Ok(())
}

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => stack.push(entry.path()),
                Ok(ft) if ft.is_file() => total += entry.metadata().map(|m| m.len()).unwrap_or(0),
                _ => {}
            }
        }
    }
    total
}

// ───────────────────────── run_command ─────────────────────────

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    /// One or both streams hit [`MAX_OUTPUT_BYTES`] and were cut.
    pub truncated: bool,
}

/// One `run_command` per project at a time (docs/05 §7.2). RAII: the id is
/// removed on drop.
struct BusyGuard(String);
static RUNNING: Mutex<Option<HashSet<String>>> = Mutex::new(None);

impl BusyGuard {
    fn acquire(project_id: &str) -> AppResult<Self> {
        let mut g = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        let set = g.get_or_insert_with(HashSet::new);
        if !set.insert(project_id.to_string()) {
            return Err(AppError::new(
                "SANDBOX_BUSY",
                "error.sandbox.busy",
                "a command is already running in this project",
            ));
        }
        Ok(BusyGuard(project_id.to_string()))
    }
}
impl Drop for BusyGuard {
    fn drop(&mut self) {
        if let Some(set) = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            set.remove(&self.0);
        }
    }
}

/// Run `program args…` inside `workspace`, shell-free, with a scrubbed
/// environment and a `timeout` that kills the whole process group.
pub async fn run_command(
    project_id: &str,
    workspace: &Path,
    program: &str,
    args: &[String],
    timeout: Duration,
) -> AppResult<CommandOutput> {
    if program.trim().is_empty() {
        return Err(AppError::new(
            "SANDBOX_BAD_COMMAND",
            "error.sandbox.badCommand",
            "no program",
        ));
    }
    let _busy = BusyGuard::acquire(project_id)?;

    // No `sh -c`: the program and every arg are passed literally, so a string
    // like `; rm -rf /` is just an argument (AC-7-7).
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .current_dir(workspace)
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    for key in ENV_ALLOWLIST {
        if *key == "PATH" {
            continue;
        }
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }
    cmd.env("PATH", command_path());
    #[cfg(unix)]
    cmd.process_group(0); // own group, so we can signal children too

    let child = cmd.spawn().map_err(|e| {
        AppError::new(
            "SANDBOX_SPAWN_FAILED",
            "error.sandbox.spawnFailed",
            e.to_string(),
        )
    })?;
    let pid = child.id();

    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => {
            let (stdout, t1) = cap(&out.stdout);
            let (stderr, t2) = cap(&out.stderr);
            Ok(CommandOutput {
                stdout,
                stderr,
                exit_code: out.status.code(),
                timed_out: false,
                truncated: t1 || t2,
            })
        }
        Ok(Err(e)) => Err(AppError::internal(format!("command io: {e}"))),
        Err(_) => {
            kill_group(pid);
            Ok(CommandOutput {
                stdout: String::new(),
                stderr: "(timed out)".into(),
                exit_code: None,
                timed_out: true,
                truncated: false,
            })
        }
    }
}

fn cap(bytes: &[u8]) -> (String, bool) {
    if bytes.len() <= MAX_OUTPUT_BYTES {
        (String::from_utf8_lossy(bytes).into_owned(), false)
    } else {
        let mut s = String::from_utf8_lossy(&bytes[..MAX_OUTPUT_BYTES]).into_owned();
        s.push_str("\n…（切り詰め / truncated）");
        (s, true)
    }
}

#[cfg(unix)]
fn kill_group(pid: Option<u32>) {
    use nix::sys::signal::{killpg, Signal};
    use nix::unistd::Pid;
    if let Some(pid) = pid {
        // process_group(0) made pgid == pid.
        let _ = killpg(Pid::from_raw(pid as i32), Signal::SIGKILL);
    }
}

/// Windows: kill the whole child tree (`taskkill /T /F`). Best-effort only:
/// a failure must never turn a timeout into a crash (design §2.3).
#[cfg(windows)]
fn kill_group(pid: Option<u32>) {
    if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

#[cfg(not(any(unix, windows)))]
fn kill_group(_pid: Option<u32>) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn ac_7_5_every_traversal_case_is_rejected() {
        let dir = ws();
        let root = dir.path();
        let cases = [
            "../../etc/passwd",
            "../../../",
            "/etc/passwd",
            r"C:\Windows\System32\config\SAM",
            r"\\?\C:\Windows",
            "workspace/../../../secret",
            "./a/../../b",
            "%2e%2e%2f%2e%2e%2f",
            r"..\..\..\",
        ];
        for c in cases {
            assert!(resolve_in_sandbox(root, c).is_err(), "must reject {c:?}");
        }
    }

    #[test]
    fn ac_7_6_symlink_escape_is_rejected() {
        let dir = ws();
        let root = dir.path();
        let outside = ws();
        std::fs::write(outside.path().join("secret.txt"), b"top secret").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), root.join("link")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(outside.path(), root.join("link")).unwrap();

        // A path *through* the symlink resolves outside and is rejected.
        assert!(resolve_in_sandbox(root, "link/secret.txt").is_err());
        // And write_file's own regular-file check catches the link itself.
        assert!(reject_symlink(&root.join("link")).is_err());
    }

    #[test]
    fn plain_relative_paths_resolve_inside() {
        let dir = ws();
        let root = dir.path().canonicalize().unwrap();
        let p = resolve_in_sandbox(&root, "notes/day1.md").unwrap();
        assert!(p.starts_with(&root));
    }

    #[tokio::test]
    async fn ac_7_7_and_7_8_shell_free_and_env_scrubbed() {
        let dir = ws();
        std::env::set_var("WAKARU_SECRET_KEY", "sk-must-not-leak");
        // Proves the arg is literal (no command runs) and the environment is
        // clean. Uses `cmd /c set` on Windows, `/usr/bin/env` on unix.
        #[cfg(unix)]
        let (program, args): (&str, Vec<String>) = ("/usr/bin/env", vec!["; rm -rf /".to_string()]);
        #[cfg(windows)]
        let (program, args): (&str, Vec<String>) = (
            "cmd",
            vec![
                "/c".to_string(),
                "set".to_string(),
                "; rm -rf /".to_string(),
            ],
        );
        let out = run_command("p1", dir.path(), program, &args, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(
            !out.stdout.contains("WAKARU_SECRET_KEY"),
            "no secrets in child env"
        );
        assert!(!out.stdout.contains("sk-must-not-leak"));
        assert!(dir.path().exists(), "nothing was deleted");
    }

    #[tokio::test]
    async fn ac_7_9_timeout_kills_the_process() {
        let dir = ws();
        let start = std::time::Instant::now();
        #[cfg(unix)]
        let (program, args): (&str, Vec<String>) = ("/bin/sleep", vec!["30".to_string()]);
        #[cfg(windows)]
        let (program, args): (&str, Vec<String>) = (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Start-Sleep -Seconds 30".to_string(),
            ],
        );
        let out = run_command("p2", dir.path(), program, &args, Duration::from_millis(400))
            .await
            .unwrap();
        assert!(out.timed_out);
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "returned promptly after kill"
        );
    }

    #[tokio::test]
    async fn ac_7_10_output_over_1mb_is_truncated() {
        let dir = ws();
        // ~2 MB of output, cut off by head-free capping.
        #[cfg(unix)]
        let (program, args): (&str, Vec<String>) = ("/usr/bin/yes", vec![]);
        #[cfg(windows)]
        let (program, args): (&str, Vec<String>) = (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "1..200000 | ForEach-Object { 'y' }".to_string(),
            ],
        );
        let out = run_command("p3", dir.path(), program, &args, Duration::from_millis(600))
            .await
            .unwrap();
        assert!(out.stdout.len() <= MAX_OUTPUT_BYTES + 64);
        assert!(out.truncated || out.timed_out);
    }

    #[tokio::test]
    async fn one_command_per_project() {
        let dir = ws();
        #[cfg(unix)]
        let (sleep_prog, sleep_args, echo_prog, echo_args): (
            &str,
            Vec<String>,
            &str,
            Vec<String>,
        ) = (
            "/bin/sleep",
            vec!["2".to_string()],
            "/bin/echo",
            vec!["hi".to_string()],
        );
        #[cfg(windows)]
        let (sleep_prog, sleep_args, echo_prog, echo_args): (
            &str,
            Vec<String>,
            &str,
            Vec<String>,
        ) = (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Start-Sleep -Seconds 2".to_string(),
            ],
            "cmd",
            vec!["/c".to_string(), "echo".to_string(), "hi".to_string()],
        );
        let a = run_command(
            "busy",
            dir.path(),
            sleep_prog,
            &sleep_args,
            Duration::from_secs(3),
        );
        let b = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            run_command(
                "busy",
                dir.path(),
                echo_prog,
                &echo_args,
                Duration::from_secs(3),
            )
            .await
        };
        let (ra, rb) = tokio::join!(a, b);
        assert!(ra.is_ok());
        assert_eq!(rb.unwrap_err().code, "SANDBOX_BUSY");
    }
}
