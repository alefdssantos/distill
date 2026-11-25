//! Sign in to Gemini by pasting Google's authorization code into Distill.
//!
//! The Antigravity CLI can run its login without a terminal: it prints the
//! authorization URL and reads one code line from stdin. This command starts it
//! in the background, shows the URL in the transcript, and accepts the code on a
//! second invocation, so the TUI is never suspended.

use std::io::Write;
use std::path::PathBuf;
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

struct LoginState {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    url_rx: std::sync::mpsc::Receiver<String>,
}

static STATE: std::sync::Mutex<Option<LoginState>> = std::sync::Mutex::new(None);

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
    let mut guard = STATE.lock().map_err(|_| "Gemini login state is locked.".to_owned())?;
    if let Some(state) = guard.as_mut() {
        if let Ok(url) = state.url_rx.try_recv() {
            return Ok(url);
        }
        return Err("A Gemini login is already waiting. Paste the code with /login-gemini <code>.".to_owned());
    }
    let mut child = Command::new(agy()?)
        .args(["acp", "--login", "--disable-telemetry"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start the Gemini login: {e}"))?;
    let stdin = child.stdin.take().ok_or_else(|| "Gemini login has no stdin.".to_owned())?;
    let stdout = child.stdout.take().ok_or_else(|| "Gemini login has no stdout.".to_owned())?;
    let (tx, url_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let line = line.trim().to_owned();
            if line.starts_with("https://") && tx.send(line).is_err() {
                break;
            }
        }
    });
    let url = url_rx
        .recv_timeout(Duration::from_secs(8))
        .map_err(|_| "The Gemini login did not print an authorization URL. Retry /login-gemini.".to_owned())?;
    *guard = Some(LoginState { child, stdin, url_rx });
    Ok(url)
}

fn finish_login(code: &str) -> Result<(), String> {
    let Some(mut state) = STATE.lock().map_err(|_| "Gemini login state is locked.".to_owned())?.take() else {
        return Err("No Gemini login is waiting. Run /login-gemini first.".to_owned());
    };
    writeln!(state.stdin, "{code}").map_err(|e| format!("Could not send the code: {e}"))?;
    drop(state.stdin);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match state.child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(format!("Gemini login failed (exit {}). Retry /login-gemini.", status.code().unwrap_or(-1)))
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(150)),
            Ok(None) => {
                let _ = state.child.kill();
                return Err("Gemini login is still running. Check the browser window and retry.".to_owned());
            }
            Err(e) => return Err(format!("Could not wait for the Gemini login: {e}")),
        }
    }
}
