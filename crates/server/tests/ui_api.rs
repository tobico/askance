//! The viewer's own namespace: the eight things the human's browser asks of the
//! server, over JSON, under `/api/ui/`.
//!
//! What is worth proving here is that this namespace is a way through to the
//! same store and the same held waits the agents' half uses — that a submit from
//! the browser ends a wait an agent is genuinely holding, and not merely that a
//! row appeared. The rendering it answers with is `ui_content.rs`'s subject.

use std::time::{Duration, Instant};

use askance_render::{
    ArchiveEntry, Archived, PendingEntry, PushKey, SetView, Standing, Submitted, Subscribed,
    Subscription,
};
use askance_schema::{Answer, ApiError, Liveness, QuestionSet, Response, SetCreated};
use askance_server::{open_database, router, store};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// Two Questions, one with Sub-questions, so a Response has to account for
/// `Q1`, `Q2`, `Q2a` and `Q2b`.
const SET: &str = r#"
title: Rate limiting for the public API
project: askance
branch: solid-viewer
questions:
  - label: Q1
    text: Where should the request counter live?
    options:
      - n: 1
        text: In-process, per instance.
      - n: 2
        text: In Redis, shared across instances.
        recommended: true
  - label: Q2
    text: How should a throttled client be told to back off?
    subquestions:
      - letter: a
        text: What should Retry-After say?
        options:
          - n: 1
            text: The exact number of seconds.
      - letter: b
        text: Anything else about the headers?
"#;

/// Everything in [`SET`] there is to answer, in the order a Response has to
/// account for it. `Q2` is not among them: it heads its Sub-questions and asks
/// nothing of its own, so no entry comes back for it.
const QUESTIONS: [&str; 3] = ["Q1", "Q2a", "Q2b"];

/// One router over a fresh database, shared by every request in a test: a wait
/// held through one clone has to hear a submit made through another, which is
/// the whole point of the two namespaces sharing their state.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool.clone(), router(pool))
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Ask the viewer's namespace for something, and read back what it answered.
async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let (status, body) = fetch(
        app,
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "GET {path} failed: {body}");
    read(&body)
}

/// Tell the viewer's namespace something, in the JSON a browser would send.
async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: serde_json::Value) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {path} failed: {body}");
    read(&body)
}

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    (status, body_text(response).await)
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// Send a Set the way the CLI does, and return the id the agent then waits on.
async fn post_set(app: &Router, yaml: &str) -> i64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sets")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    created.id
}

/// Open a wait on a Set the way the CLI does, held for `hold` seconds.
async fn wait_for_response(app: &Router, id: i64, hold: u64) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/sets/{id}/response?hold={hold}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Hold a wait on a Set the way the CLI does, on a task the test can drop.
fn hold_a_wait(app: &Router, id: i64) -> tokio::task::JoinHandle<()> {
    let app = app.clone();

    tokio::spawn(async move {
        let _held = wait_for_response(&app, id, 60).await;
    })
}

/// The pending list, asked for until the row for `id` reads as `liveness`.
///
/// A wait takes its slot as its handler starts running, which is a moment after
/// the request opening it goes in — so the list is asked again rather than once
/// after a guessed pause.
async fn pending_where(app: &Router, id: i64, liveness: Liveness) -> Vec<PendingEntry> {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        let pending: Vec<PendingEntry> = get(app, "/api/ui/pending").await;
        if pending
            .iter()
            .any(|row| row.id == id && row.liveness == liveness)
        {
            return pending;
        }

        assert!(
            Instant::now() < deadline,
            "waited for Set {id} to read as {liveness:?} in vain: {pending:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn answered(label: &str, selected: Option<u32>) -> Answer {
    Answer {
        label: label.to_owned(),
        selected,
        free_text: None,
        unanswered: false,
    }
}

fn left_open(label: &str) -> Answer {
    Answer {
        label: label.to_owned(),
        selected: None,
        free_text: None,
        unanswered: true,
    }
}

/// What the viewer builds when the human answers `Q1` and confirms the warning
/// about the three they left alone.
fn confirmed_with_three_left_open() -> Response {
    Response {
        answers: vec![answered("Q1", Some(2)), left_open("Q2a"), left_open("Q2b")],
        comment: None,
    }
}

/// A Response resolving every question of [`SET`], for the tests that only need
/// a Set to be answered.
fn decided() -> Response {
    Response {
        answers: QUESTIONS.iter().map(|label| left_open(label)).collect(),
        comment: Some("Neither — why is this not just a cache in front?".to_owned()),
    }
}

/// A Set with nothing but a title, for the lists, which never look inside one.
fn bare(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: Vec::new(),
        postscript: None,
        project: Some("askance".to_owned()),
        branch: Some("solid-viewer".to_owned()),
        diff: None,
    }
}

#[tokio::test]
async fn every_waiting_set_is_listed_newest_first_with_its_age_and_its_liveness() {
    let (_dir, pool, app) = fresh_app().await;
    for title in ["the older ask", "the newer ask"] {
        store::insert_set(&pool, &bare(title)).await.unwrap();
    }

    let pending: Vec<PendingEntry> = get(&app, "/api/ui/pending").await;

    let titles: Vec<&str> = pending.iter().map(|row| row.title.as_str()).collect();
    assert_eq!(titles, ["the newer ask", "the older ask"]);

    let row = &pending[0];
    assert_eq!(row.project.as_deref(), Some("askance"));
    assert_eq!(row.branch.as_deref(), Some("solid-viewer"));
    // All three arrive decided rather than as a timestamp: the age in the
    // words the list is scanned in, the exact minute behind it for the
    // tooltip, and the badge from the registry of held waits. A Set this new
    // is one whose agent is on its way to its first wait.
    assert_eq!(row.age, "just now");
    assert!(
        row.created_stamp.ends_with(" UTC"),
        "expected the exact minute behind the age, got {:?}",
        row.created_stamp
    );
    assert_eq!(row.liveness, Liveness::Waiting);
}

#[tokio::test]
async fn a_set_nothing_has_waited_on_for_long_enough_reads_as_disconnected() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &bare("nobody is listening"))
        .await
        .unwrap();
    // Old enough that the window measured from its creation has closed, and no
    // wait was ever held on it.
    backdate_created(&pool, stored.id, "2026-08-03T09:00:00.000Z").await;

    let pending: Vec<PendingEntry> = get(&app, "/api/ui/pending").await;

    assert_eq!(pending[0].liveness, Liveness::Disconnected);
    assert!(
        pending[0].age.ends_with(" ago"),
        "expected an age in words, got {:?}",
        pending[0].age
    );
}

#[tokio::test]
async fn the_badge_is_the_wait_the_agents_half_is_genuinely_holding() {
    let (_dir, pool, app) = fresh_app().await;
    let waited_on = post_set(&app, SET).await;
    let orphan = store::insert_set(&pool, &bare("the one whose agent went"))
        .await
        .unwrap()
        .id;
    // Both old enough that the window measured from their creation has closed, so
    // the only thing that can make either of them read as waiting is a wait.
    for id in [waited_on, orphan] {
        backdate_created(&pool, id, "2026-08-03T09:00:00.000Z").await;
    }

    let agent = hold_a_wait(&app, waited_on);
    let pending = pending_where(&app, waited_on, Liveness::Waiting).await;

    // And a wait held on one Set says nothing about another: the registry is
    // asked per Set, not read as "somebody is about".
    let other = pending
        .iter()
        .find(|row| row.id == orphan)
        .expect("the Set nothing is waiting on is still pending");
    assert_eq!(
        other.liveness,
        Liveness::Disconnected,
        "the Set nothing is waiting on keeps its own badge: {pending:?}"
    );

    // The set view is fed from the same registry, because the two badges are one
    // fact: the human archives a Set from its own page, with the badge in view.
    let set: SetView = get(&app, &format!("/api/ui/sets/{waited_on}")).await;
    assert_eq!(set.standing, Standing::Waiting(Liveness::Waiting));
    let orphaned: SetView = get(&app, &format!("/api/ui/sets/{orphan}")).await;
    assert_eq!(
        orphaned.standing,
        Standing::Waiting(Liveness::Disconnected),
        "a Set nothing is waiting on is still waiting on the human"
    );

    agent.abort();
}

#[tokio::test]
async fn a_disconnected_set_is_still_answerable_from_the_viewer() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    backdate_created(&pool, id, "2026-08-03T09:00:00.000Z").await;

    let pending: Vec<PendingEntry> = get(&app, "/api/ui/pending").await;
    assert_eq!(pending[0].liveness, Liveness::Disconnected);

    // Display state only (ADR-0001): the badge never gates an answer, and a Set
    // whose agent has gone is neither withdrawn nor closed on its own.
    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(decided()).unwrap(),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    let pending: Vec<PendingEntry> = get(&app, "/api/ui/pending").await;
    assert!(
        pending.is_empty(),
        "the answered Set should be off the list: {pending:?}"
    );
}

#[tokio::test]
async fn an_answered_set_leaves_the_pending_list_for_the_archive() {
    let (_dir, pool, app) = fresh_app().await;
    let answered = store::insert_set(&pool, &bare("already answered"))
        .await
        .unwrap();
    store::insert_set(&pool, &bare("still waiting"))
        .await
        .unwrap();
    store::insert_response(&pool, answered.id, &Response::default())
        .await
        .unwrap()
        .expect("the Set had no Response yet");

    let pending: Vec<PendingEntry> = get(&app, "/api/ui/pending").await;
    let archive: Vec<ArchiveEntry> = get(&app, "/api/ui/archive").await;

    assert_eq!(
        pending.iter().map(|row| &row.title).collect::<Vec<_>>(),
        ["still waiting"]
    );
    assert_eq!(
        archive.iter().map(|row| &row.title).collect::<Vec<_>>(),
        ["already answered"]
    );
}

#[tokio::test]
async fn the_archive_words_each_settling_and_says_which_were_never_answered() {
    let (_dir, pool, app) = fresh_app().await;

    // Settled long ago, so however far today drifts from the stamp, it stays
    // past the week that ends the relative wording: this row is dated.
    let decided_set = store::insert_set(&pool, &bare("decided")).await.unwrap();
    store::insert_response(&pool, decided_set.id, &Response::default())
        .await
        .unwrap()
        .unwrap();
    settle_at(&pool, decided_set.id, "2025-08-03T09:07:00.000Z").await;

    // Archived just now, which is inside the week: this row is aged.
    let orphan = store::insert_set(&pool, &bare("nobody ever answered"))
        .await
        .unwrap();
    store::archive_set(&pool, &store::Settlements::new(1), orphan.id)
        .await
        .unwrap();

    let archive: Vec<ArchiveEntry> = get(&app, "/api/ui/archive").await;

    // Newest settling first, whichever way each of them was settled — aged
    // while fresh, dated once it is not, with the exact minute beside both.
    let rows: Vec<(&str, &str, bool)> = archive
        .iter()
        .map(|row| (row.title.as_str(), row.settled_at.as_str(), row.unanswered))
        .collect();
    assert_eq!(
        rows,
        [
            ("nobody ever answered", "just now", true),
            ("decided", "2025-08-03", false),
        ]
    );
    assert_eq!(archive[1].settled_stamp, "2025-08-03 09:07 UTC");
    assert!(
        archive[0].settled_stamp.ends_with(" UTC"),
        "expected the exact minute behind the age, got {:?}",
        archive[0].settled_stamp
    );
}

#[tokio::test]
async fn a_set_carries_where_it_stands_along_with_its_own_material() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let set: SetView = get(&app, &format!("/api/ui/sets/{id}")).await;

    assert_eq!(set.id, id);
    assert_eq!(set.title, "Rate limiting for the public API");
    assert_eq!(set.questions.len(), 2);
    // Which view the human gets turns on this, so it travels with the Set
    // rather than being asked for once the page is already up.
    assert_eq!(set.standing, Standing::Waiting(Liveness::Waiting));
}

#[tokio::test]
async fn a_set_that_does_not_exist_says_so_however_it_was_asked_for() {
    let (_dir, _pool, app) = fresh_app().await;

    // A number that names no Set, and an id that could never name one: the
    // viewer's URLs are typed by hand as often as they are followed.
    for path in ["/api/ui/sets/404", "/api/ui/sets/not-a-number"] {
        let (status, body) = fetch(
            &app,
            Request::builder().uri(path).body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND, "{path} answered {body}");
        let refused: ApiError = read(&body);
        assert!(
            refused.error.contains("no Question Set"),
            "expected a refusal that says what is missing, got {:?}",
            refused.error
        );
    }
}

#[tokio::test]
async fn a_submit_from_the_viewer_settles_a_wait_that_is_genuinely_being_held() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // The agent's wait goes up first and finds nothing, so the only thing that
    // can end it is word of the submit — not a row it happened to read.
    let agent = tokio::spawn({
        let app = app.clone();
        async move { wait_for_response(&app, id, 30).await }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !agent.is_finished(),
        "the wait should still be held: nothing has answered the Set yet",
    );

    let response = confirmed_with_three_left_open();
    let submitted = Instant::now();
    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(&response).unwrap(),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    let waited = agent.await.unwrap();
    let woken_in = submitted.elapsed();

    assert_eq!(waited.status(), StatusCode::OK);
    assert!(
        woken_in < Duration::from_secs(5),
        "the wait should have been woken by the submit, not left to time out; \
         it took {woken_in:?} of its 30s hold",
    );

    // And what the agent gets is the Response the human confirmed, unanswered
    // markers and all.
    assert_eq!(
        Response::from_yaml(&body_text(waited).await).unwrap(),
        response
    );
}

#[tokio::test]
async fn an_answered_set_reads_back_as_the_decision_that_was_made() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    let response = decided();

    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(&response).unwrap(),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    let set: SetView = get(&app, &format!("/api/ui/sets/{id}")).await;
    let Standing::Answered(answered) = set.standing else {
        panic!("expected an answered Set, got {:?}", set.standing);
    };
    assert_eq!(answered.response, response);
    assert!(
        !answered.submitted_at.is_empty(),
        "expected the time the Response landed"
    );
}

#[tokio::test]
async fn a_second_submit_is_told_the_set_was_already_answered() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    let body = serde_json::to_value(decided()).unwrap();

    let first: Submitted = post(&app, &format!("/api/ui/sets/{id}/response"), body.clone()).await;
    let second: Submitted = post(&app, &format!("/api/ui/sets/{id}/response"), body).await;

    assert_eq!(first, Submitted::Accepted);
    assert_eq!(second, Submitted::AlreadyAnswered);
}

#[tokio::test]
async fn a_response_that_misses_a_question_comes_back_naming_it() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // Q2b is unaccounted for, which the viewer should never let happen — so it
    // is carried back and shown rather than swallowed.
    let short = Response {
        answers: vec![answered("Q1", Some(1)), left_open("Q2a")],
        comment: None,
    };

    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(&short).unwrap(),
    )
    .await;

    let Submitted::Rejected(violations) = outcome else {
        panic!("expected the Response refused, got {outcome:?}");
    };
    assert!(
        violations.iter().any(|said| said.contains("Q2b")),
        "expected the missed question named, got {violations:?}"
    );
}

#[tokio::test]
async fn a_submit_to_a_set_that_is_gone_says_which_way_it_went() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    let body = serde_json::to_value(decided()).unwrap();

    // Archived first, which closes it for good: a Response to one cannot make
    // it an answered Set instead.
    let archived: Archived = post(&app, &format!("/api/ui/sets/{id}/archive"), body.clone()).await;
    assert_eq!(archived, Archived::Closed);

    let refused: Submitted = post(&app, &format!("/api/ui/sets/{id}/response"), body.clone()).await;
    assert_eq!(refused, Submitted::Archived);

    let missing: Submitted = post(&app, "/api/ui/sets/9999/response", body).await;
    assert_eq!(missing, Submitted::NoSuchSet);
}

#[tokio::test]
async fn archiving_a_set_ends_a_wait_held_on_it_and_files_it_unanswered() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let agent = tokio::spawn({
        let app = app.clone();
        async move { wait_for_response(&app, id, 30).await }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    let outcome: Archived = post(
        &app,
        &format!("/api/ui/sets/{id}/archive"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(outcome, Archived::Closed);

    // Nothing is ever coming, and the CLI is told so rather than left polling.
    assert_eq!(agent.await.unwrap().status(), StatusCode::GONE);

    let pending: Vec<PendingEntry> = get(&app, "/api/ui/pending").await;
    assert!(pending.is_empty(), "an archived Set is off the list");

    let archive: Vec<ArchiveEntry> = get(&app, "/api/ui/archive").await;
    assert!(archive[0].unanswered, "it reached the Archive unanswered");

    let set: SetView = get(&app, &format!("/api/ui/sets/{id}")).await;
    assert!(
        matches!(set.standing, Standing::ArchivedUnanswered(_)),
        "expected the Set closed unanswered, got {:?}",
        set.standing
    );
}

#[tokio::test]
async fn archiving_says_what_it_found_when_the_set_had_already_gone() {
    let (_dir, _pool, app) = fresh_app().await;
    let answered_set = post_set(&app, SET).await;
    post::<Submitted>(
        &app,
        &format!("/api/ui/sets/{answered_set}/response"),
        serde_json::to_value(decided()).unwrap(),
    )
    .await;

    let orphan = post_set(&app, SET).await;
    let empty = serde_json::json!({});
    post::<Archived>(
        &app,
        &format!("/api/ui/sets/{orphan}/archive"),
        empty.clone(),
    )
    .await;

    // A decision is not something to close, and one closing is enough.
    for (id, expected) in [
        (answered_set, Archived::AlreadyAnswered),
        (orphan, Archived::AlreadyArchived),
        (9999, Archived::NoSuchSet),
    ] {
        let outcome: Archived =
            post(&app, &format!("/api/ui/sets/{id}/archive"), empty.clone()).await;
        assert_eq!(outcome, expected, "archiving {id} again");
    }
}

#[tokio::test]
async fn a_device_is_handed_the_public_key_and_never_the_private_half() {
    let (_dir, pool, app) = fresh_app().await;
    let keys = store::vapid_keys(&pool).await.unwrap();

    let (status, body) = fetch(
        &app,
        Request::builder()
            .uri("/api/ui/push/key")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let handed: PushKey = read(&body);
    assert_eq!(handed.key, keys.public_key);
    assert!(
        !body.contains(&keys.private_key),
        "the private half must never leave the server: {body}"
    );
}

#[tokio::test]
async fn a_device_asking_to_be_told_is_stored_and_can_ask_to_stop() {
    let (_dir, pool, app) = fresh_app().await;
    let subscription = Subscription {
        endpoint: "https://push.example/abc".to_owned(),
        p256dh: "a-public-key".to_owned(),
        auth: "an-auth-secret".to_owned(),
    };

    let stored: Subscribed = post(
        &app,
        "/api/ui/push/subscribe",
        serde_json::to_value(&subscription).unwrap(),
    )
    .await;
    assert_eq!(stored, Subscribed::Stored);
    assert_eq!(store::push_subscriptions(&pool).await.unwrap().len(), 1);

    // Subscribing again is the same device, not a second one.
    let again: Subscribed = post(
        &app,
        "/api/ui/push/subscribe",
        serde_json::to_value(&subscription).unwrap(),
    )
    .await;
    assert_eq!(again, Subscribed::Stored);
    assert_eq!(store::push_subscriptions(&pool).await.unwrap().len(), 1);

    let (status, body) = fetch(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/push/unsubscribe")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "endpoint": subscription.endpoint }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert!(store::push_subscriptions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_subscription_with_nothing_to_send_to_is_refused() {
    let (_dir, pool, app) = fresh_app().await;

    let refused: Subscribed = post(
        &app,
        "/api/ui/push/subscribe",
        serde_json::json!({ "endpoint": "", "p256dh": "a-key", "auth": "a-secret" }),
    )
    .await;

    assert_eq!(refused, Subscribed::Incomplete);
    assert!(store::push_subscriptions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn turning_notifications_off_on_a_device_the_server_never_heard_of_is_no_error() {
    let (_dir, _pool, app) = fresh_app().await;

    let (status, body) = fetch(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/push/unsubscribe")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "endpoint": "https://push.example/never-stored" }).to_string(),
            ))
            .unwrap(),
    )
    .await;

    // What was asked for holds either way: nothing is sent there.
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
}

#[tokio::test]
async fn the_agents_contract_still_speaks_its_own_language() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // The viewer's namespace is JSON and the agents' is YAML, over the same
    // store: neither has taken the other's shape.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/sets/{id}/response?hold=0"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(body_text(health).await, "ok");
}

/// Age a Set, so the window Liveness is measured in has closed.
async fn backdate_created(pool: &SqlitePool, id: i64, created_at: &str) {
    sqlx::query("UPDATE question_sets SET created_at = ? WHERE id = ?")
        .bind(created_at)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

/// Move when a Response landed, so the Archive's dates are a test's to decide.
async fn settle_at(pool: &SqlitePool, id: i64, submitted_at: &str) {
    sqlx::query("UPDATE responses SET submitted_at = ? WHERE set_id = ?")
        .bind(submitted_at)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}
