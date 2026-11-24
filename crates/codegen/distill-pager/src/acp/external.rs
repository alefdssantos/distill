// Modified for Distill by Samuel Fajreldines, 2026.
//! External Antigravity transport.
//!
//! Runs the installed official `agy` CLI as a child (`agy acp --stdio --sandbox`)
//! and bridges its piped stdio into the same typed ACP channels used by the
//! in-process and leader transports. The child receives only a small,
//! non-secret environment allowlist; `HOME` stays so the signed-in
//! `~/.gemini` account remains available.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol as acp;
use anyhow::{Context, Result};
use distill_acp_lib::{
    AcpAgentChannel, AcpClientChannel, AcpGatewayReceiver, AcpGatewaySender, LineBufferedRead,
    acp_channels,
};
use distill_shell::agent::config::Config as AgentConfig;
use distill_shell::util::distill_home::distill_home;
use distill_tty_utils::ProcessGroup;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tokio_util::sync::CancellationToken;

use super::{
    AcpConnection, AgentEndpoint, AgentLocation, ConnectFlags,
    initialize_connection_with_eager_auth, spawn,
};

const PROGRAM: &str = "agy";
const ARGS: [&str; 4] = ["acp", "--stdio", "--sandbox", "--disable-telemetry"];

/// Variables the child needs to run and to find the signed-in `~/.gemini`
/// account. Provider keys, proxy credentials, SSH agents, and every other
/// ambient secret stay in the parent.
const CHILD_ENV_ALLOWLIST: &[&str] = &[
    "HOME",
    "USERPROFILE",
    "PATH",
    "USER",
    "LOGNAME",
    "SHELL",
    "TMPDIR",
    "TEMP",
    "TMP",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "XDG_STATE_HOME",
];

/// Bound for reaping the child after cancellation or EOF.
const CHILD_EXIT_GRACE: Duration = Duration::from_secs(2);

/// Connect to the official Antigravity CLI over stdio.
///
/// The returned connection uses the normal `AcpConnection` surface. Distill
/// eager authentication is disabled: the CLI owns its own signed-in session,
/// and an explicit `/login-gemini` flow lives in the UI.
pub async fn connect_antigravity(
    cancel: &CancellationToken,
    flags: ConnectFlags,
) -> Result<AcpConnection> {
    let program = resolve_on_path(PROGRAM)?;
    let auth_manager = inert_auth_manager();
    connect_with_program(cancel, flags, program, auth_manager).await
}

/// Variant with an explicit executable, used by the fake-child tests.
async fn connect_with_program(
    cancel: &CancellationToken,
    flags: ConnectFlags,
    program: PathBuf,
    auth_manager: Arc<distill_login::AuthManager>,
) -> Result<AcpConnection> {
    let child_cancel = cancel.child_token();
    // Dropping this future (e.g. a bounded connect timeout) must stop the
    // transport thread and child, not only an explicit `Err` return.
    let guard = child_cancel.clone().drop_guard();
    let endpoint = spawn_transport(program, child_cancel).await?;
    let mut connection =
        initialize_connection_with_eager_auth(endpoint, &flags, auth_manager, false).await?;
    // `agy` advertises login methods even when its session is already signed
    // in. Keep them for explicit `/login-gemini`, but never force Distill or
    // Grok startup auth.
    connection.needs_login = false;
    connection.login_label = None;
    connection.login_method_id = None;
    guard.disarm();
    Ok(connection)
}

/// Build the safe invocation for the explicit `/login-gemini` flow. Callers
/// add the login subcommand and args; executable and environment match the
/// transport exactly.
pub(crate) fn antigravity_login_command() -> Result<std::process::Command> {
    let program = resolve_on_path(PROGRAM)?;
    let mut command = std::process::Command::new(program);
    apply_child_env_std(&mut command);
    Ok(command)
}

/// Spawn the child on its own runtime thread and return an endpoint for the
/// standard initialize path. `ClientSideConnection` uses `spawn_local`, so the
/// transport runs on a `LocalSet`; the thread also owns the child handle.
async fn spawn_transport(program: PathBuf, cancel: CancellationToken) -> Result<AgentEndpoint> {
    let (client_channel, agent_channel) = acp_channels();
    let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();
    let thread_cancel = cancel.clone();
    let thread_handle = spawn::spawn_runtime_thread("pager-antigravity-transport", move |rt| {
        let local = tokio::task::LocalSet::new();
        let result = local.block_on(&rt, async move {
            let (child, group) = match spawn_child(&program, &thread_cancel) {
                Ok(spawned) => spawned,
                Err(error) => {
                    let _ = ready_tx.send(Err(anyhow::anyhow!("{error:#}")));
                    return Err(error);
                }
            };
            let _ = ready_tx.send(Ok(()));
            run_transport(child, group, agent_channel, thread_cancel).await
        });
        drop(local);
        spawn::shutdown_worker_runtime(rt);
        result
    })
    .await?;

    match ready_rx.await {
        Ok(Ok(())) => Ok(AgentEndpoint {
            tx: client_channel.tx,
            rx: client_channel.rx,
            cancel,
            location: AgentLocation::Thread(thread_handle),
        }),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(anyhow::anyhow!(
            "antigravity transport thread exited during startup"
        )),
    }
}

fn spawn_child(program: &Path, cancel: &CancellationToken) -> Result<(Child, Arc<ProcessGroup>)> {
    if cancel.is_cancelled() {
        anyhow::bail!("antigravity connect cancelled");
    }

    let mut command = Command::new(program);
    command
        .args(ARGS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    apply_child_env_tokio(&mut command);

    distill_tty_utils::global_process_scope()
        .spawn(command)
        .with_context(|| format!("failed to spawn {}", program.display()))
}

fn apply_child_env_std(command: &mut std::process::Command) {
    command.env_clear();
    for key in CHILD_ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

fn apply_child_env_tokio(command: &mut Command) {
    command.env_clear();
    for key in CHILD_ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

async fn run_transport(
    mut child: Child,
    group: Arc<ProcessGroup>,
    agent_channel: AcpAgentChannel,
    cancel: CancellationToken,
) -> Result<()> {
    let stdin = child
        .stdin
        .take()
        .context("antigravity child stdin is not piped")?;
    let stdout = child
        .stdout
        .take()
        .context("antigravity child stdout is not piped")?;

    if let Some(stderr) = child.stderr.take() {
        // Drain only: child stderr may contain account material and must never
        // reach Distill's logs.
        tokio::task::spawn_local(async move {
            let mut sink = tokio::io::sink();
            let _ = tokio::io::copy(&mut BufReader::new(stderr), &mut sink).await;
        });
    }

    let gateway_tx = AcpGatewaySender::new(agent_channel.tx).with_tracing(false);
    let incoming = LineBufferedRead::spawn_local(stdout.compat());
    let (conn, handle_io) = acp::ClientSideConnection::new(
        gateway_tx,
        stdin.compat_write(),
        incoming,
        |future| {
            tokio::task::spawn_local(future);
        },
    );
    let gateway_rx = AcpGatewayReceiver::new(agent_channel.rx, conn).with_tracing(false);
    let mut io_task = tokio::task::spawn_local(handle_io);
    tokio::task::spawn_local(gateway_rx.run());

    #[cfg(unix)]
    {
        let pid = child.id();
        // Observe exit with WNOWAIT so the leader stays unreaped. Only then is
        // its process-group id safe to signal.
        let observation = tokio::select! {
            biased;
            _ = cancel.cancelled() => observe_leader_state(pid),
            _ = &mut io_task => observe_leader_state(pid),
            state = observe_leader_exit(pid) => state,
        };
        cancel.cancel();

        let leader = observation?;
        if leader != LeaderState::AlreadyReaped {
            // Kill only this child's enrolled group. Never the global scope,
            // and never after an external waiter reaped the leader.
            let _ = group.kill();
        }
        let _ = tokio::time::timeout(CHILD_EXIT_GRACE, child.wait()).await;
        if leader == LeaderState::AlreadyReaped {
            anyhow::bail!(
                "antigravity child was reaped by another waiter; piped output may be retained"
            );
        }
    }

    #[cfg(not(unix))]
    {
        let child_exited = tokio::select! {
            biased;
            _ = cancel.cancelled() => false,
            _ = child.wait() => true,
            _ = &mut io_task => false,
        };
        cancel.cancel();

        if !child_exited {
            let _ = group.kill();
            let _ = tokio::time::timeout(CHILD_EXIT_GRACE, child.wait()).await;
        }
    }
    Ok(())
}

#[cfg(unix)]
const EXIT_POLL_TICK: Duration = Duration::from_millis(20);

#[cfg(unix)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum LeaderState {
    Running,
    ExitedUnreaped,
    AlreadyReaped,
}

/// Poll the owned direct child without consuming its wait status.
#[cfg(unix)]
fn observe_leader_state(pid: Option<u32>) -> std::io::Result<LeaderState> {
    let Some(pid) = pid else {
        return Ok(LeaderState::AlreadyReaped);
    };
    if pid == 0 || pid > i32::MAX as u32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("antigravity child pid {pid} is not a valid Unix pid"),
        ));
    }

    // SAFETY: waitid writes one initialized siginfo_t, P_PID restricts the
    // query to the caller-owned direct child, and WNOWAIT preserves its status.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ECHILD) {
            return Ok(LeaderState::AlreadyReaped);
        }
        return Err(error);
    }
    // SAFETY: a successful waitid initialized siginfo_t; zero means WNOHANG
    // found no exited state.
    if unsafe { info.si_pid() } == 0 {
        Ok(LeaderState::Running)
    } else {
        Ok(LeaderState::ExitedUnreaped)
    }
}

#[cfg(unix)]
async fn observe_leader_exit(pid: Option<u32>) -> std::io::Result<LeaderState> {
    loop {
        match observe_leader_state(pid)? {
            LeaderState::Running => tokio::time::sleep(EXIT_POLL_TICK).await,
            state => return Ok(state),
        }
    }
}

/// `AcpConnection` requires an `AuthManager`, but this transport never uses
/// Distill credentials: no refresher, login, or proactive refresh is started.
fn inert_auth_manager() -> Arc<distill_login::AuthManager> {
    let agent_config = AgentConfig::default();
    Arc::new(distill_login::AuthManager::new_with_proxy_base_url(
        &distill_home(),
        agent_config.grok_com_config.clone(),
        agent_config.endpoints.proxy_url(),
    ))
}

/// Resolve a bare program name against absolute `PATH` entries only. An empty
/// or relative entry would make the OS resolve the program against the current
/// working directory, so those entries are skipped.
fn resolve_on_path(program: &str) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("PATH")
        && let Ok(found) = resolve_in_path(program, &path)
    {
        return Ok(found);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if home.is_absolute() {
            let candidate = home.join(".local/bin").join(program);
            if is_executable(&candidate) {
                return Ok(candidate);
            }
        }
    }
    anyhow::bail!(
        "missing Gemini CLI \"{program}\" on PATH; install the official CLI and put it on PATH"
    )
}

fn resolve_in_path(program: &str, path: &std::ffi::OsStr) -> Result<PathBuf> {
    for dir in std::env::split_paths(path) {
        if dir.as_os_str().is_empty() || !dir.is_absolute() {
            continue;
        }
        let candidate = dir.join(program);
        if is_executable(&candidate) {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "missing Gemini CLI \"{program}\" on PATH; install the official CLI and put it on PATH"
    )
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::time::Instant;

    use super::*;

    fn write_fake_child(dir: &Path, script: &str) -> PathBuf {
        let path = dir.join("fake-agy");
        std::fs::write(&path, script).expect("write fake child");
        let mut permissions = std::fs::metadata(&path).expect("stat fake child").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).expect("chmod fake child");
        path
    }

    fn test_auth_manager(home: &Path) -> Arc<distill_login::AuthManager> {
        Arc::new(distill_login::AuthManager::new(
            home,
            distill_login::GrokComConfig::default(),
        ))
    }

    async fn wait_until_gone(pid: i32) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None) {
                Err(nix::errno::Errno::ESRCH) => return true,
                _ => tokio::time::sleep(Duration::from_millis(20)).await,
            }
        }
        false
    }

    fn read_pid(path: &Path) -> i32 {
        std::fs::read_to_string(path)
            .expect("fake child wrote its pid")
            .trim()
            .parse()
            .expect("pid is numeric")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fake_child_initializes_and_shuts_down() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_path = dir.path().join("child.pid");
        let program = write_fake_child(
            dir.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$$" > '{}'
while IFS= read -r line; do
    case "$line" in
        *'"method":"initialize"'*)
            id=${line#*"id":}
            id=${id%%[!0-9]*}
            printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"1","serverCapabilities":{{}},"authMethods":[{{"id":"oauth-personal","name":"Personal Google"}},{{"id":"gemini-api-key","name":"Gemini API key"}}]}}}}\n' "$id"
            ;;
    esac
done
"#,
                pid_path.display()
            ),
        );

        let cancel = CancellationToken::new();
        let connection = tokio::time::timeout(
            Duration::from_secs(10),
            connect_with_program(
                &cancel,
                ConnectFlags::default(),
                program,
                test_auth_manager(dir.path()),
            ),
        )
        .await
        .expect("initialization must not hang")
        .expect("fake child should initialize");

        assert!(!connection.is_grok_shell);
        assert_eq!(connection.auth_methods.len(), 2);
        assert!(!connection.needs_login);
        assert!(connection.login_method_id.is_none());

        let pid = read_pid(&pid_path);
        let AcpConnection {
            cancel: transport_cancel,
            agent_thread,
            ..
        } = connection;
        transport_cancel.cancel();
        let handle = agent_thread.expect("transport thread handle");
        tokio::task::spawn_blocking(move || handle.join())
            .await
            .expect("join task")
            .expect("transport thread panicked")
            .expect("transport thread exits cleanly");

        assert!(
            wait_until_gone(pid).await,
            "cancelled child {pid} must be reaped"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fake_child_natural_exit_reaps_cleanly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_path = dir.path().join("child.pid");
        let program = write_fake_child(
            dir.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$$" > '{}'
while IFS= read -r line; do
    case "$line" in
        *'"method":"initialize"'*)
            id=${line#*"id":}
            id=${id%%[!0-9]*}
            printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"1","serverCapabilities":{{}},"authMethods":[]}}}}\n' "$id"
            exit 0
            ;;
    esac
done
"#,
                pid_path.display()
            ),
        );

        let connection = tokio::time::timeout(
            Duration::from_secs(10),
            connect_with_program(
                &CancellationToken::new(),
                ConnectFlags::default(),
                program,
                test_auth_manager(dir.path()),
            ),
        )
        .await
        .expect("natural exit must not hang initialization")
        .expect("fake child should initialize");
        let pid = read_pid(&pid_path);
        let AcpConnection { agent_thread, .. } = connection;
        let handle = agent_thread.expect("transport thread handle");
        tokio::task::spawn_blocking(move || handle.join())
            .await
            .expect("join task")
            .expect("transport thread panicked")
            .expect("natural child exit must clean up");

        assert!(wait_until_gone(pid).await, "child {pid} must be reaped");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dropped_connect_future_cancels_child() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_path = dir.path().join("child.pid");
        let program = write_fake_child(
            dir.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$$" > '{}'
while IFS= read -r line; do
    :
done
"#,
                pid_path.display()
            ),
        );

        let result = tokio::time::timeout(
            Duration::from_secs(1),
            connect_with_program(
                &CancellationToken::new(),
                ConnectFlags::default(),
                program,
                test_auth_manager(dir.path()),
            ),
        )
        .await;
        assert!(result.is_err(), "initialize is intentionally unanswered");

        let pid = read_pid(&pid_path);
        assert!(
            wait_until_gone(pid).await,
            "dropping the connect future must cancel child {pid}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fake_child_initialize_error_cleans_up() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_path = dir.path().join("child.pid");
        let program = write_fake_child(
            dir.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$$" > '{}'
while IFS= read -r line; do
    case "$line" in
        *'"method":"initialize"'*)
            id=${line#*"id":}
            id=${id%%[!0-9]*}
            printf '{{"jsonrpc":"2.0","id":%s,"error":{{"code":-32603,"message":"fake failure"}}}}\n' "$id"
            ;;
    esac
done
"#,
                pid_path.display()
            ),
        );

        let cancel = CancellationToken::new();
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            connect_with_program(
                &cancel,
                ConnectFlags::default(),
                program,
                test_auth_manager(dir.path()),
            ),
        )
        .await
        .expect("failed initialization must not hang");

        assert!(result.is_err(), "initialize error must reach the caller");
        let pid = read_pid(&pid_path);
        assert!(
            wait_until_gone(pid).await,
            "child {pid} must be killed after initialize fails"
        );
    }

    #[test]
    fn resolve_on_path_rejects_relative_entries() {
        use std::ffi::OsStr;

        let dir = tempfile::tempdir().expect("tempdir");
        let program = write_fake_child(dir.path(), "#!/bin/sh\nexit 0\n");
        let relative_path = OsStr::new(".");

        assert!(
            resolve_in_path("fake-agy", relative_path).is_err(),
            "relative PATH entries must not resolve {}",
            program.display()
        );
    }
}
