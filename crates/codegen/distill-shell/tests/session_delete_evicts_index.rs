// Modified for Distill by Samuel Fajreldines, 2026.
//! One binary, one home: `distill_home()` memoizes the first read for the process, so tests that need a temp home have to share one.
//! `#[serial]` keeps their env writes apart.

use std::sync::{Arc, OnceLock};

use agent_client_protocol as acp;
use distill_login::{AuthManager, GrokComConfig};
use distill_shell::session::info::Info;
use distill_shell::session::persistence::delete_session_history;
use distill_shell::session::storage::search::{
    IndexDecision, SearchIndex, SearchIndexManager, SessionSearchRequest, execute_search,
    start_if_enabled,
};
use distill_shell::session::storage::{JsonlStorageAdapter, StorageAdapter};
use distill_test_support::EnvGuard;

fn home() -> &'static std::path::Path {
    static HOME: OnceLock<(tempfile::TempDir, EnvGuard)> = OnceLock::new();
    HOME.get_or_init(|| {
        let dir = tempfile::TempDir::new().unwrap();
        let guard = EnvGuard::set("GROK_HOME", dir.path());
        (dir, guard)
    })
    .0
    .path()
}

/// `start_if_enabled` is the only way to get a manager, so the test calls it with `GROK_SESSION_SEARCH` left at its default.
fn start_index() -> SearchIndexManager {
    let _default_on = EnvGuard::unset("GROK_SESSION_SEARCH");
    match start_if_enabled(&distill_shell::agent::config::Config::default()) {
        SearchIndex::Started(index) => index,
        SearchIndex::Off { reason } => {
            panic!("session search is on by default, got off: {reason}")
        }
    }
}

/// Titles are one made-up token, searched back verbatim.
/// A query of ordinary words ORs its tokens, which would let one session's row answer for another.
async fn seed_session(root: &std::path::Path, id: &str, cwd: &str) {
    let storage = JsonlStorageAdapter::with_root(root.to_path_buf());
    let info = Info {
        id: acp::SessionId::new(id),
        cwd: cwd.to_string(),
    };
    storage
        .init_session(&info, acp::ModelId::new("test-model"))
        .await
        .unwrap();
    storage
        .update_session_title(&info, title_for(id))
        .await
        .unwrap();
}

fn title_for(id: &str) -> String {
    format!("zzqq{id}")
}

async fn finds(index: &SearchIndexManager, root: &std::path::Path, id: &str) -> bool {
    let req = SessionSearchRequest {
        query: title_for(id),
        cwd: None,
        limit: 10,
        offset: 0,
        include_content: false,
    };
    !execute_search(IndexDecision::On(index), root, &req)
        .await
        .unwrap()
        .results
        .is_empty()
}

#[tokio::test]
#[serial_test::serial]
async fn deleting_a_session_clears_only_its_own_search_row() {
    let root = home();
    let index = start_index();

    // All three up front: only the first search builds the index
    seed_session(root, "orphan", "/ws-a").await;
    seed_session(root, "elsewhere", "/ws-b").await;
    seed_session(root, "scoped", "/ws-c").await;
    assert!(finds(&index, root, "orphan").await, "precondition: indexed");
    assert!(
        finds(&index, root, "elsewhere").await,
        "precondition: indexed"
    );
    assert!(finds(&index, root, "scoped").await, "precondition: indexed");

    let auth = Arc::new(AuthManager::new(root, GrokComConfig::default()));

    let session_dir =
        distill_shell::util::distill_home::sessions_cwd_dir_in(root, "/ws-a").join("orphan");
    std::fs::remove_dir_all(&session_dir).unwrap();
    let deletion = delete_session_history("orphan", None, false, auth.clone(), Some(&index))
        .await
        .unwrap();
    assert!(!deletion.any_removed(), "nothing was left to remove");
    assert!(
        !finds(&index, root, "orphan").await,
        "a delete with no workspace must clear a row nothing else will ever prune",
    );

    delete_session_history(
        "elsewhere",
        Some("/ws-a"),
        false,
        auth.clone(),
        Some(&index),
    )
    .await
    .unwrap();
    assert!(
        finds(&index, root, "elsewhere").await,
        "a delete scoped to another workspace must not evict this session",
    );

    let deletion = delete_session_history("scoped", Some("/ws-c"), false, auth, Some(&index))
        .await
        .unwrap();
    assert!(deletion.local_removed, "the session was there to remove");
    assert!(
        !finds(&index, root, "scoped").await,
        "a delete that removed the session must take its row too",
    );
}

/// A host that keeps no index still prunes a row from an index built earlier.
/// The manager here only reads the row back; the delete runs with `None`.
#[tokio::test]
#[serial_test::serial]
async fn deleting_a_session_evicts_its_row_without_a_handle() {
    let root = home();
    let index = start_index();

    seed_session(root, "indexless", "/ws-d").await;
    assert!(
        finds(&index, root, "indexless").await,
        "precondition: indexed"
    );

    let auth = Arc::new(AuthManager::new(root, GrokComConfig::default()));
    let deletion = delete_session_history("indexless", Some("/ws-d"), false, auth, None)
        .await
        .unwrap();
    assert!(deletion.local_removed, "the session was there to remove");
    assert!(
        !finds(&index, root, "indexless").await,
        "a delete from a host with no index must still take the row",
    );
}

#[test]
#[serial_test::serial]
fn loading_config_applies_requirement_pins() {
    // Removed again before returning: the other test resolves its config from this same home.
    let pin = home().join("requirements.toml");
    std::fs::write(&pin, "[features]\nsession_search = false\n").unwrap();

    let loaded = distill_shell::config::load_agent_config_disk_only();
    std::fs::remove_file(&pin).unwrap();
    let config = loaded.expect("config loads");

    let resolved = config.feature(distill_shell::agent::config::Feature::SessionSearch);
    assert!(
        !resolved.value,
        "a one-shot command must apply pins, or the environment outranks them",
    );
    assert_eq!(
        resolved.source,
        distill_shell::agent::config::ConfigSource::Requirement
    );
}

#[tokio::test]
#[serial_test::serial]
async fn explicit_delete_removes_hidden_side_sessions_without_touching_the_parent() {
    let root = home();
    let cwd = "/ws-side-delete";
    seed_session(root, "side-delete-parent", cwd).await;
    let bucket = distill_shell::util::distill_home::sessions_cwd_dir_in(root, cwd);
    let parent_summary = bucket.join("side-delete-parent/summary.json");
    let parent_before = std::fs::read(&parent_summary).unwrap();
    let auth = Arc::new(AuthManager::new(root, GrokComConfig::default()));

    for (id, scope) in [("side-scoped", Some(cwd)), ("side-global", None)] {
        seed_session(root, id, cwd).await;
        let dir = bucket.join(id);
        let summary_path = dir.join("summary.json");
        let mut summary: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&summary_path).unwrap()).unwrap();
        summary["session_kind"] = serde_json::json!("side");
        std::fs::write(&summary_path, serde_json::to_vec(&summary).unwrap()).unwrap();
        assert!(
            JsonlStorageAdapter::with_root(root.to_path_buf())
                .list_sessions(Some(cwd))
                .await
                .unwrap()
                .iter()
                .all(|session| session.info.id.to_string() != id),
            "precondition: side sessions are hidden from normal history"
        );
        let wrong_scope =
            delete_session_history(id, Some("/wrong-workspace"), false, auth.clone(), None)
                .await
                .unwrap();
        assert!(!wrong_scope.local_removed);
        assert!(summary_path.exists());

        let deletion = delete_session_history(id, scope, false, auth.clone(), None)
            .await
            .unwrap();
        assert!(
            deletion.local_removed,
            "an explicitly named hidden session must be deleted"
        );
        assert!(
            !dir.exists(),
            "a success must remove the actual session files"
        );
        assert_eq!(std::fs::read(&parent_summary).unwrap(), parent_before);
        assert!(
            !delete_session_history(id, scope, false, auth.clone(), None)
                .await
                .unwrap()
                .local_removed,
            "repeated deletion is an idempotent absence"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn corrupt_summary_identity_fails_and_preserves_dirs() {
    let root = home();
    let cwd = "/ws-corrupt";
    seed_session(root, "corrupt-session", cwd).await;
    let bucket = distill_shell::util::distill_home::sessions_cwd_dir_in(root, cwd);
    let dir = bucket.join("corrupt-session");
    let summary_path = dir.join("summary.json");

    // Corrupt the summary's cwd to trigger the ID/cwd mismatch guard.
    let mut summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&summary_path).unwrap()).unwrap();
    summary["info"]["cwd"] = serde_json::json!("/ws-other");
    std::fs::write(&summary_path, serde_json::to_vec(&summary).unwrap()).unwrap();

    let auth = Arc::new(AuthManager::new(root, GrokComConfig::default()));

    // Deletion must fail because the summary identity doesn't match its destination.
    let result = delete_session_history("corrupt-session", Some(cwd), false, auth, None).await;
    match result {
        Err(distill_shell::session::persistence::DeleteSessionError::List(err)) => {
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        }
        _ => panic!("expected InvalidData error, got {result:?}"),
    }

    // Both the target directory and its parent bucket must remain untouched.
    assert!(dir.exists(), "target directory must be preserved on error");
    assert!(
        bucket.exists(),
        "parent directory must be preserved on error"
    );
}
