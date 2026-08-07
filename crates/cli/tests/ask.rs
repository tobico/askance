//! The `askance ask` round trip: a Set goes out, the CLI blocks, and the
//! human's Response comes back on stdout — across a server restart if need be.
//!
//! The last test here is the README's quickstart, run against the very files
//! the README tells the reader to run.

mod support;

use std::future::Future;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use askance_schema::Response;
use askance_server::store::{self, StoredSet};
use support::{REPO_NAME, linked_worktree, repo_with_a_commit};

/// Two Questions, one with a Sub-question, so a Response has to cover `Q1`,
/// `Q2` and `Q2a`. `project` and `branch` are forged: the CLI derives both
/// from the working directory and must overwrite whatever the agent claimed.
const SET: &str = "
title: How should the CLI wait?
preface: |
  The CLI holds a long-poll until the Response lands.

  It has no expiry: only delivery or a kill ends the wait.
project: forged-by-the-agent
branch: forged-by-the-agent
questions:
  - label: Q1
    text: How long a hold should the CLI ask for?
    options:
      - n: 1
        text: Thirty seconds
        recommended: true
      - n: 2
        text: Five minutes
  - label: Q2
    text: What should a dropped connection do?
    subquestions:
      - letter: a
        text: How long before the first retry?
";

/// A Response covering everything in [`SET`] there is to answer, with a
/// multi-line comment so the block-scalar rendering can be checked on the way
/// out.
///
/// `Q2` is not among them and must not be: it carries a Sub-question and no
/// Options, which makes it a Heading over `Q2a` rather than a question of its
/// own, and the grammar refuses a Response that answers one.
const COMPLETE: &str = "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    unanswered: true
comment: |
  Thirty seconds, and reconnect without saying anything on stdout.

  The agent parses stdout; keep the noise on stderr.
";

/// A third level of nesting, which the question grammar forbids. The CLI has
/// to refuse this itself, before anything reaches the server.
const THREE_LEVELS_DEEP: &str = "
title: Where should validation live?
questions:
  - label: Q1
    text: Who owns the retry?
    subquestions:
      - letter: a
        text: And who owns the backoff?
        subquestions:
          - letter: i
            text: A third level, which the grammar forbids.
";

/// The real server, on a runtime of its own, so a blocking test can kill it
/// under the CLI's feet and bring it back on the same port.
struct Server {
    addr: SocketAddr,
    database: PathBuf,
    runtime: tokio::runtime::Runtime,
}

impl Server {
    fn start(database: PathBuf) -> Self {
        Self::bind("127.0.0.1:0".parse().unwrap(), database)
    }

    fn bind(addr: SocketAddr, database: PathBuf) -> Self {
        Self::serve(addr, database)
    }

    fn serve(addr: SocketAddr, database: PathBuf) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let (listener, addr, pool) = runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let addr = listener.local_addr().unwrap();
            let pool = askance_server::open_database(&database).await.unwrap();
            (listener, addr, pool)
        });

        runtime.spawn(async move {
            let _ = axum::serve(listener, askance_server::router(pool)).await;
        });

        Server {
            addr,
            database,
            runtime,
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    /// Stop serving without a graceful shutdown, so a held long-poll is
    /// dropped exactly as it would be if the process were killed. Hands back
    /// what [`Server::bind`] needs to bring the same server up again.
    fn kill(self) -> (SocketAddr, PathBuf) {
        let Server {
            addr,
            database,
            runtime,
        } = self;
        runtime.shutdown_timeout(Duration::from_millis(100));
        (addr, database)
    }

    /// The Set the CLI submitted, read back through a second pool on the same
    /// file — the store is where the enriched Set can actually be seen.
    fn stored_set(&self, id: i64) -> Option<StoredSet> {
        self.block_on(async {
            let pool = askance_server::open_database(&self.database).await.unwrap();
            let stored = store::load_set(&pool, id).await.unwrap();
            pool.close().await;
            stored
        })
    }

    /// Block until the CLI has submitted Set `id`.
    fn await_stored_set(&self, id: i64) -> StoredSet {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(stored) = self.stored_set(id) {
                return stored;
            }
            assert!(
                Instant::now() < deadline,
                "the CLI never submitted Question Set {id}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Answer a Set the way the human's device does: YAML over HTTP.
    fn answer(&self, id: i64, yaml: &str) {
        let reply = ureq::post(format!("{}/api/v1/sets/{id}/response", self.url()))
            .header("Content-Type", "application/yaml")
            .send(yaml)
            .unwrap();
        assert_eq!(reply.status().as_u16(), 201);
    }

    /// Archive a Set unanswered the way the human's browser does. It lives in the
    /// viewer's namespace and nowhere else: the agent API has no route for it,
    /// because only a human may close a Set nobody is going to answer.
    fn archive(&self, id: i64) {
        let reply = ureq::post(format!("{}/api/ui/sets/{id}/archive", self.url()))
            .header("Content-Type", "application/json")
            .send("{}")
            .unwrap();
        assert_eq!(reply.status().as_u16(), 200);
    }
}

/// `askance ask`, pointed at the test server and running in `dir`, with its
/// three streams on pipes the test can drive and read back.
fn command(server: &Server, dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_askance"));
    command
        .arg("ask")
        .env("ASKANCE_SERVER", server.url())
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Start `askance ask`, feeding it `set` on stdin, running in `dir`.
fn ask(server: &Server, dir: &Path, set: &str) -> Child {
    let mut child = command(server, dir)
        .spawn()
        .expect("the askance binary should be built for its own tests");

    // Dropping the handle closes the pipe, which is the CLI's end of input.
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(set.as_bytes()).unwrap();
    drop(stdin);

    child
}

/// Start `askance ask <file>`, the form the README's quickstart uses.
fn ask_file(server: &Server, dir: &Path, file: &Path) -> Child {
    command(server, dir)
        .arg(file)
        .stdin(Stdio::null())
        .spawn()
        .expect("the askance binary should be built for its own tests")
}

/// Cut a still-waiting CLI short — the wait has no other end — without leaving
/// a zombie behind.
fn kill(mut child: Child) {
    child.kill().unwrap();
    child.wait().unwrap();
}

/// What the CLI wrote and how it exited, insisting it exited at all.
fn finished(child: Child) -> Output {
    let output = child.wait_with_output().unwrap();
    eprintln!(
        "askance stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn a_schema_violating_set_is_refused_locally_naming_the_question() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("askance.db"));

    let output = finished(ask(&server, tmp.path(), THREE_LEVELS_DEEP));

    assert!(
        !output.status.success(),
        "an illegal Set should not be accepted"
    );
    assert!(
        stderr(&output).contains("Q1a"),
        "the refusal should name the offending Sub-question, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "a refusal has nothing to say on stdout"
    );
    assert!(
        server.stored_set(1).is_none(),
        "an illegal Set must fail before anything is sent"
    );
}

#[test]
fn the_response_is_delivered_on_stdout_as_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("askance.db"));

    let waiting = ask(&server, tmp.path(), SET);
    server.await_stored_set(1);
    server.answer(1, COMPLETE);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "delivery is a clean exit, got {:?}",
        output.status
    );

    let printed = stdout(&output);
    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("stdout should be a Response: {error}\n{printed}"));
    assert_eq!(response.answers.len(), 2);
    assert_eq!(response.answers[0].selected, Some(1));
    assert!(response.answers[1].unanswered);

    assert!(
        printed.contains("comment: |"),
        "a multi-line comment comes out as a block scalar, got:\n{printed}"
    );
    assert!(
        !printed.contains("askance:"),
        "the CLI's own chatter belongs on stderr, got:\n{printed}"
    );
    assert!(
        stderr(&output).is_empty(),
        "a wait that goes to plan says nothing at all: a harness that merges the \
         two streams into one file is handed the Response alone, got:\n{}",
        stderr(&output)
    );
}

#[test]
fn the_cli_reconnects_when_the_server_restarts_mid_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("askance.db"));

    let waiting = ask(&server, tmp.path(), SET);
    server.await_stored_set(1);

    let (addr, database) = server.kill();
    std::thread::sleep(Duration::from_millis(250));
    let server = Server::bind(addr, database);

    server.answer(1, COMPLETE);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "the CLI should have ridden out the restart, got {:?}",
        output.status
    );
    assert!(
        Response::from_yaml(&stdout(&output)).is_ok(),
        "the Response should still arrive on stdout, got:\n{}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains("retrying"),
        "the reconnection should be reported on stderr, got:\n{}",
        stderr(&output)
    );

    // A harness running the CLI in the background captures both streams into
    // one file, so anything the CLI said on the way is read along with the
    // Response. Saying it as a YAML comment is what keeps that file parseable.
    let merged = format!("{}{}", stderr(&output), stdout(&output));
    assert!(
        Response::from_yaml(&merged).is_ok(),
        "the two streams merged should still parse as the Response, got:\n{merged}"
    );
}

#[test]
fn a_set_archived_unanswered_ends_the_wait_for_good() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("askance.db"));

    let waiting = ask(&server, tmp.path(), SET);
    server.await_stored_set(1);

    // The human closes the Set instead of answering it — the CLI is holding a
    // wait at this moment, and that is the wait that has to end.
    server.archive(1);

    let output = finished(waiting);
    assert!(
        !output.status.success(),
        "no Response arrived, so this is not a clean exit, got {:?}",
        output.status
    );
    assert!(
        stderr(&output).contains("archived unanswered"),
        "the agent has to be told why it is not getting an answer, got:\n{}",
        stderr(&output)
    );
    assert!(
        !stderr(&output).contains("retrying"),
        "an archived Set is not a transient failure to reconnect through, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "there is no Response to print, got:\n{}",
        stdout(&output)
    );
}

#[test]
fn project_branch_and_diff_are_derived_from_the_working_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("askance.db"));

    let root = repo_with_a_commit(tmp.path());
    let linked = linked_worktree(&root, "feature");

    // A clean linked worktree: the root repo's name, the worktree's branch,
    // and no Diff at all.
    let clean = ask(&server, &linked, SET);
    let stored = server.await_stored_set(1);
    kill(clean);

    assert_eq!(
        stored.set.project.as_deref(),
        Some(REPO_NAME),
        "from a linked worktree the project is the root repo's name"
    );
    assert_eq!(stored.set.branch.as_deref(), Some("feature"));
    assert_eq!(
        stored.set.diff, None,
        "a clean tree carries no Diff, got {:?}",
        stored.set.diff
    );

    // The same worktree with an untracked file in it.
    std::fs::write(
        linked.join("open-questions.md"),
        "a line only in the working tree\n",
    )
    .unwrap();

    let dirty = ask(&server, &linked, SET);
    let stored = server.await_stored_set(2);
    kill(dirty);

    let diff = stored.set.diff.expect("a dirty tree carries a Diff");
    assert!(
        diff.contains("+++ b/open-questions.md")
            && diff.contains("+a line only in the working tree"),
        "the Diff should carry the untracked file's contents, got:\n{diff}"
    );
}

/// One of the workspace's `examples/`, by the name the README uses.
fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn the_readme_quickstart_delivers_the_example_response() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("askance.db"));

    // `askance ask examples/questions.yaml`, and the Set is the first one this
    // server has seen, so the id the README curls is 1.
    let waiting = ask_file(&server, tmp.path(), &example("questions.yaml"));
    let stored = server.await_stored_set(1);
    assert_eq!(stored.set.title, "Rate limiting for the public API");

    // The curl step. `answer` insists on a 201, so this is also where an
    // example Response that failed to resolve the example Set would be caught.
    let submitted = std::fs::read_to_string(example("response.yaml")).unwrap();
    server.answer(1, &submitted);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "the quickstart ends with the CLI exiting 0, got {:?}",
        output.status
    );

    let printed = stdout(&output);
    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("stdout should be a Response: {error}\n{printed}"));
    assert_eq!(
        response,
        Response::from_yaml(&submitted).unwrap(),
        "the agent should get back exactly what the human submitted"
    );
}
