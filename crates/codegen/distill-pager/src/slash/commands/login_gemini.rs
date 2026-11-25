//! Sign in to Gemini by pasting Google's authorization code into Distill.
//!
//! The Antigravity CLI can run its login without a terminal: it prints the
//! authorization URL and reads one code line from stdin. This command starts it
//! in the background, shows the URL in the transcript, and accepts the code on a
//! second invocation, so the TUI is never suspended.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct LoginGeminiCommand;

impl SlashCommand for LoginGeminiCommand {
    slash_meta! {
        name: "login-gemini",
        description: "Sign in to Gemini: run /login-gemini, open the URL, then /login-gemini <code>",
        usage: "/login-gemini [authorization code or final URL]",
        takes_args: true,
        offered_when_session_less: true,
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let code = args.trim();
        if code.is_empty() {
            match start_login() {
                Ok(url) => CommandResult::Message(format!(
                    "Open this URL in your browser, authorize, then paste the code here:\n\n{url}\n\nRun `/login-gemini <code>` with the code or the URL you landed on."
                )),
                Err(error) => CommandResult::Error(error),
            }
        } else {
            match finish_login(code) {
                Ok(()) => CommandResult::Message(
                    "Logged in to Gemini. Run `/gemini` to open the session in this window.".into(),
                ),
                Err(error) => CommandResult::Error(error),
            }
        }
    }
}

fn state_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("DISTILL_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".distill")))
        .ok_or_else(|| "Could not locate the Distill home directory.".to_owned())?;
    let dir = home.join("gemini-login");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

fn agy() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("agy");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    let local = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".local/bin/agy"));
    match local {
        Some(path) if path.is_file() => Ok(path),
        _ => Err("Could not find the Antigravity CLI (`agy`). Install it first.".to_owned()),
    }
}

fn start_login() -> Result<String, String> {
    let dir = state_dir()?;
    let fifo = dir.join("code.fifo");
    let out = dir.join("stdout.log");
    let err = dir.join("stderr.log");
    let status = dir.join("status");
    if fifo.exists() {
        let _ = std::fs::remove_file(&fifo);
    }
    if status.exists() {
        let _ = std::fs::remove_file(&status);
    }
    let url = dir.join("url.txt");
    let _ = std::fs::remove_file(&url);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&err);

    let fifo_s = fifo.to_string_lossy().replace('\'', "'\\''");
    let out_s = out.to_string_lossy().replace('\'', "'\\''");
    let err_s = err.to_string_lossy().replace('\'', "'\\''");
    let status_s = status.to_string_lossy().replace('\'', "'\\''");
    let agy_s = agy()?.to_string_lossy().replace('\'', "'\\''");
    // The FIFO keeps the login process alive between the two slash commands.
    let script = format!(
        "rm -f '{fifo_s}'; mkfifo '{fifo_s}' || exit 1; \
         ( '{agy_s}' acp --login --disable-telemetry <'{fifo_s}' >'{out_s}' 2>'{err_s}'; echo $? >'{status_s}' ) &"
    );
    Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start the Gemini login: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(&out) {
            if let Some(line) = text.lines().find(|line| line.trim().starts_with("https://")) {
                let url_text = line.trim().to_owned();
                let _ = std::fs::write(&url, &url_text);
                return Ok(url_text);
            }
        }
        std::thread::sleep(Duration::from_millis(120));
    }
    Err("The Gemini login did not print an authorization URL. Retry /login-gemini.".to_owned())
}

fn finish_login(code: &str) -> Result<(), String> {
    let dir = state_dir()?;
    let fifo = dir.join("code.fifo");
    let status = dir.join("status");
    if !fifo.exists() {
        return Err("No Gemini login is waiting. Run /login-gemini first.".to_owned());
    }
    {
        let mut writer = std::fs::OpenOptions::new()
            .write(true)
            .open(&fifo)
            .map_err(|e| format!("Could not reach the waiting login: {e}"))?;
        writeln!(writer, "{code}").map_err(|e| format!("Could not send the code: {e}"))?;
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(&status) {
            let code = text.trim();
            if code == "0" {
                return Ok(());
            }
            let err = std::fs::read_to_string(dir.join("stderr.log")).unwrap_or_default();
            let out = std::fs::read_to_string(dir.join("stdout.log")).unwrap_or_default();
            let detail = if err.trim().is_empty() { out } else { err };
            return Err(format!(
                "Gemini login failed (exit {code}). {}",
                detail.trim().chars().take(600).collect::<String>()
            ));
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err("Gemini login is still running. Wait a moment and check /login-gemini again.".to_owned())
}

#[allow(dead_code)]
fn _path_is_file(path: &Path) -> bool {
    path.is_file()
}
