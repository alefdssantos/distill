// Modified for Distill by Samuel Fajreldines, 2026.
use super::{
    build_context_window, emit_loop, live_turn, local_replacement_meter, split_normalized_remote,
    strip_trailing_separator,
};
use crate::extensions::notification::PromptUsageModel;
use distill_acp_lib::AcpClientMessage;
use distill_workspace::session::git::normalize_repo_url;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;

fn attributed_call(
    id: &str,
    role: &str,
    endpoint: Option<&str>,
    status: distill_chat_state::UsageCallStatus,
    usage: Option<(u32, u32)>,
    usage_complete: bool,
    final_decision: Option<&str>,
) -> distill_chat_state::UsageAttribution {
    distill_chat_state::UsageAttribution {
        attempt_id: id.to_owned(),
        task_id: None,
        turn_id: None,
        request_id: None,
        role: role.to_owned(),
        model_id: "local-model".to_owned(),
        endpoint: endpoint.map(str::to_owned),
        requested_effort: None,
        applied_effort: None,
        reason: None,
        bytes_in: None,
        bytes_out: None,
        source_kind: None,
        final_decision: final_decision.map(str::to_owned),
        status,
        usage: usage.map(
            |(prompt_tokens, completion_tokens)| distill_sampling_types::TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                reasoning_tokens: 100,
                cached_prompt_tokens: 50,
                cache_creation_prompt_tokens: 20,
                cache_creation_1h_prompt_tokens: 0,
            },
        ),
        usage_complete,
        api_duration_ms: None,
        cost_usd_ticks: None,
        cost_basis: distill_chat_state::UsageCostBasis::Unknown,
        cache_lifetime_secs: None,
    }
}

#[test]
fn local_replacement_meter_counts_complete_loopback_jev_and_used_utility() {
    use distill_chat_state::UsageCallStatus::{Completed, Rejected};

    let attributions = vec![
        attributed_call(
            "jev-1",
            "jev",
            Some("http://localhost:8080/v1"),
            Completed,
            Some((10, 5)),
            true,
            None,
        ),
        attributed_call(
            "utility-1",
            "utility",
            Some("http://127.0.0.1:9000/v1"),
            Completed,
            Some((20, 7)),
            true,
            Some("used"),
        ),
        attributed_call(
            "jev-ipv6",
            "jev",
            Some("http://[::1]:9000/v1"),
            Completed,
            Some((2, 3)),
            true,
            None,
        ),
        attributed_call(
            "utility-nothing",
            "utility",
            Some("http://localhost/v1"),
            Completed,
            Some((1, 1)),
            true,
            Some("nothing"),
        ),
        attributed_call(
            "jev-remote",
            "jev",
            Some("https://api.example.com/v1"),
            Completed,
            Some((100, 100)),
            true,
            None,
        ),
        attributed_call(
            "jev-rejected",
            "jev",
            Some("http://localhost/v1"),
            Rejected,
            Some((3, 4)),
            true,
            None,
        ),
        attributed_call(
            "jev-absent",
            "jev",
            Some("http://localhost/v1"),
            Completed,
            None,
            false,
            None,
        ),
        attributed_call(
            "jev-partial",
            "jev",
            Some("http://localhost/v1"),
            Completed,
            Some((5, 5)),
            false,
            None,
        ),
        attributed_call(
            "main-local",
            "main",
            Some("http://localhost/v1"),
            Completed,
            Some((40, 40)),
            true,
            None,
        ),
        attributed_call(
            "worker-local",
            "worker",
            Some("http://localhost/v1"),
            Completed,
            Some((30, 30)),
            true,
            None,
        ),
        attributed_call(
            "jev-1",
            "jev",
            Some("http://localhost/v1"),
            Completed,
            Some((1000, 0)),
            true,
            None,
        ),
    ];

    let (tokens, samples) = local_replacement_meter(&attributions);
    assert_eq!(
        tokens, 54,
        "prompt + completion only, rejected Jev included, duplicate attempt id folded once"
    );
    assert_eq!(samples, vec![15, 27, 5, 7]);
    assert_eq!(local_replacement_meter(&[]), (0, Vec::new()));
}

/// A local decision that rejects the answer still spent provider tokens. The observed HUD
/// had only these rejected Jev calls, so requiring a completed one left the meter hidden.
#[test]
fn rejected_local_jev_calls_with_complete_usage_light_the_meter() {
    use distill_chat_state::UsageCallStatus::{Cancelled, Failed, Rejected};

    let attributions = vec![
        attributed_call(
            "jev-1",
            "jev",
            Some("http://127.0.0.1:11434/v1"),
            Rejected,
            Some((3000, 15)),
            true,
            None,
        ),
        attributed_call(
            "jev-2",
            "jev",
            Some("http://localhost:11434/v1"),
            Rejected,
            Some((1191, 0)),
            true,
            None,
        ),
        attributed_call(
            "jev-3",
            "jev",
            Some("http://[::1]:11434/v1"),
            Rejected,
            Some((361, 0)),
            true,
            None,
        ),
        attributed_call(
            "jev-remote",
            "jev",
            Some("https://api.example.com/v1"),
            Rejected,
            Some((9_999, 9_999)),
            true,
            None,
        ),
        attributed_call(
            "jev-failed",
            "jev",
            Some("http://localhost/v1"),
            Failed,
            Some((500, 0)),
            true,
            None,
        ),
        attributed_call(
            "jev-cancelled",
            "jev",
            Some("http://localhost/v1"),
            Cancelled,
            Some((600, 0)),
            true,
            None,
        ),
        attributed_call(
            "jev-missing-usage",
            "jev",
            Some("http://localhost/v1"),
            Rejected,
            None,
            false,
            None,
        ),
        attributed_call(
            "utility-rejected",
            "utility",
            Some("http://localhost/v1"),
            Rejected,
            Some((700, 0)),
            true,
            Some("used"),
        ),
    ];

    assert_eq!(
        local_replacement_meter(&attributions),
        (4_567, vec![3_015, 1_191, 361])
    );
}

#[test]
fn local_replacement_meter_keeps_last_eight_samples_oldest_first() {
    let attributions = (1..=10)
        .map(|tokens| {
            attributed_call(
                &format!("jev-{tokens}"),
                "jev",
                Some("http://localhost/v1"),
                distill_chat_state::UsageCallStatus::Completed,
                Some((tokens, 0)),
                true,
                None,
            )
        })
        .collect::<Vec<_>>();

    let (tokens, samples) = local_replacement_meter(&attributions);
    assert_eq!(tokens, 55);
    assert_eq!(samples, vec![3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn session_usage_splits_fresh_input_from_the_cache_buckets() {
    let totals = PromptUsageModel {
        input_tokens: 30_000,
        output_tokens: 900,
        cached_read_tokens: 8_000,
        cache_creation_tokens: 5_000,
        model_calls: 1,
        ..Default::default()
    };
    let window = build_context_window(100_000, Some(42_000), Some(&totals), 80);

    // The wire's 30k `input_tokens` already contains both cache buckets, so 17k is what was fresh and the three fields must not overlap
    let usage = window.session_usage.unwrap();
    assert_eq!(usage.input_tokens, 17_000);
    assert_eq!(usage.cache_creation_input_tokens, 5_000);
    assert_eq!(usage.cache_read_input_tokens, 8_000);
    // The billed total keeps the cache buckets the fresh count sheds.
    assert_eq!(window.session_input_tokens, Some(30_000));
}

#[test]
fn a_turn_is_on_the_wire_only_while_one_is_running() {
    let started = 1_730_000_000_000;

    assert_eq!(
        live_turn(Some(started), Some("prompt-1")),
        Some(distill_status_line::StatusLineTurn {
            started_at_ms: started
        })
    );
    assert_eq!(
        live_turn(Some(started), None),
        None,
        "chat state keeps the stamp after the turn ends, and the prompt id does not"
    );
    assert_eq!(live_turn(None, Some("prompt-1")), None);
}

#[test]
fn percentages_are_whole_numbers_inside_zero_to_one_hundred() {
    let window = build_context_window(300_000, Some(100_000), None, 80);
    assert_eq!(window.used_percentage, Some(33));
    assert_eq!(window.remaining_percentage, Some(67));

    let over = build_context_window(1_000, Some(4_000), None, 80);
    assert_eq!(over.used_percentage, Some(100));
    assert_eq!(over.remaining_percentage, Some(0));
}

#[test]
fn session_usage_is_null_until_a_call_bills() {
    let window = build_context_window(100_000, Some(0), Some(&PromptUsageModel::default()), 80);
    assert!(window.session_usage.is_none());
}

#[test]
fn strips_the_trailing_separator_git_adds() {
    assert_eq!(
        strip_trailing_separator(Path::new("/repo/wt/")),
        PathBuf::from("/repo/wt")
    );
    assert_eq!(strip_trailing_separator(Path::new("/")), PathBuf::from("/"));
}

#[test]
fn only_origin_names_the_repo() {
    use super::remote_url;

    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    repo.remote("upstream", "https://example.com/parent/widget.git")
        .unwrap();
    assert_eq!(remote_url(&repo), None);

    repo.remote("origin", "https://example.com/fork/widget.git")
        .unwrap();
    assert_eq!(
        remote_url(&repo).as_deref(),
        Some("https://example.com/fork/widget.git")
    );
}

#[test]
fn splits_remote_into_host_owner_name() {
    let repo = split_normalized_remote("example.com/acme/widget").unwrap();
    assert_eq!(repo.host, "example.com");
    assert_eq!(repo.owner.as_deref(), Some("acme"));
    assert_eq!(repo.name, "widget");

    let nested = split_normalized_remote("example.com/group/sub/proj").unwrap();
    assert_eq!(nested.owner.as_deref(), Some("sub"));
    assert_eq!(nested.name, "proj");

    let ownerless = split_normalized_remote("example.com/widget").unwrap();
    assert_eq!(ownerless.name, "widget");
    assert_eq!(ownerless.owner, None);

    let tokenized = "https://user:token@example.com/acme/widget.git";
    let clean = split_normalized_remote(&normalize_repo_url(tokenized).unwrap()).unwrap();
    assert_eq!(clean.host, "example.com");
    assert_eq!(clean.name, "widget");
}

#[tokio::test(start_paused = true)]
async fn burst_during_a_build_is_answered_by_one_more_build() {
    let wake = Arc::new(Notify::new());
    let builds = Rc::new(Cell::new(0usize));

    let parked = tokio::time::timeout(
        Duration::from_secs(10),
        emit_loop(wake.clone(), || {
            let builds = builds.clone();
            let wake = wake.clone();
            Some(async move {
                builds.set(builds.get() + 1);
                if builds.get() == 1 {
                    for _ in 0..5 {
                        wake.notify_one();
                    }
                }
                tokio::task::yield_now().await;
            })
        }),
    )
    .await;

    assert!(parked.is_err(), "the loop ran out of wakes and parked");
    assert_eq!(builds.get(), 2, "one build answers the burst, not five");
}

#[tokio::test(start_paused = true)]
async fn nothing_left_to_build_ends_the_loop() {
    tokio::time::timeout(
        Duration::from_secs(10),
        emit_loop(Arc::new(Notify::new()), || None::<std::future::Ready<()>>),
    )
    .await
    .expect("a loop with nothing to build must return");
}

#[tokio::test]
async fn client_that_cannot_draw_the_row_never_builds_one() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (subagent, mut dropped) = emitter_fixture(Client::Subagent).await;
            let refusing = super::run_status_emitter(Arc::downgrade(&subagent));
            let refused = tokio::time::timeout(Duration::from_secs(10), refusing).await;
            assert!(refused.is_ok(), "a subagent's emitter parked on the wake");
            assert!(dropped.try_recv().is_err(), "a subagent built a row");

            let (session, mut painted) = emitter_fixture(Client::WithoutTheRow).await;
            let emitter =
                tokio::task::spawn_local(super::run_status_emitter(Arc::downgrade(&session)));
            session.emit_status_snapshot_detached();
            // Lets the emitter consume the wake while the row is still off.
            tokio::task::yield_now().await;

            session.status_line_enabled.store(true, Ordering::Relaxed);
            session.emit_status_snapshot_detached();
            let seeded = tokio::time::timeout(Duration::from_secs(10), painted.recv()).await;
            assert!(matches!(seeded, Ok(Some(_))), "a later attach must build");

            // Ends the loop, so a build started by the earlier wake has landed before the receiver below is drained
            drop(session);
            tokio::time::timeout(Duration::from_secs(10), emitter)
                .await
                .expect("the emitter returns once the session is gone")
                .expect("the emitter task panicked");
            assert!(
                painted.try_recv().is_err(),
                "the wake before x.ai/statusLine built a payload as well"
            );
        })
        .await;
}

#[tokio::test]
async fn the_notification_payload_serializes_without_a_trigger_key() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (session, _rx) = emitter_fixture(Client::WithoutTheRow).await;
            let ctx = session.build_status_context().await;
            let payload = serde_json::to_value(&ctx).expect("the payload serializes");
            assert!(
                payload.get("trigger").is_none(),
                "the notification describes the session, not a run: `trigger` \
                 belongs on a command row's stdin alone"
            );
        })
        .await;
}

#[tokio::test]
async fn a_dropped_session_ends_its_parked_emitter() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (session, _painted) = emitter_fixture(Client::WithoutTheRow).await;
            let emitter =
                tokio::task::spawn_local(super::run_status_emitter(Arc::downgrade(&session)));
            // Parks the emitter on the wake: without the yield it has not reached one, and the test would pass on the loop's first pass
            tokio::task::yield_now().await;
            assert!(
                !emitter.is_finished(),
                "the emitter left before the session"
            );

            drop(session);
            tokio::time::timeout(Duration::from_secs(10), emitter)
                .await
                .expect("a parked emitter outlived the session that owns it")
                .expect("the emitter task panicked");
        })
        .await;
}

enum Client {
    Subagent,
    WithoutTheRow,
}

async fn emitter_fixture(
    client: Client,
) -> (
    Arc<super::SessionActor>,
    UnboundedReceiver<AcpClientMessage>,
) {
    let (gateway_tx, gateway_rx) = tokio::sync::mpsc::unbounded_channel();
    let (persistence_tx, _persistence_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut actor =
        super::super::support::create_test_actor(50_000, 100_000, 85, gateway_tx, persistence_tx)
            .await;
    // A subagent advertises the row and still must not build one.
    let (is_subagent, wants_a_row) = match client {
        Client::Subagent => (true, true),
        Client::WithoutTheRow => (false, false),
    };
    actor.startup_hints.is_subagent = is_subagent;
    actor
        .status_line_enabled
        .store(wants_a_row, Ordering::Relaxed);
    (Arc::new(actor), gateway_rx)
}
