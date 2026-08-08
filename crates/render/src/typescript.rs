//! Writing the viewer's TypeScript out of the Rust it is generated from.
//!
//! Every type on the viewer's side of the wire is described once, in Rust, and
//! `web/src/api/types.ts` is written from those descriptions — so the two
//! languages cannot come to disagree about a field. The file is committed, and
//! rewritten by every `cargo test`: what says whether anything moved is the diff.
//!
//! The roots are listed here rather than each type carrying its own `#[ts(export)]`
//! for two reasons. One is that the list *is* the viewer's wire surface, and a
//! list is a thing that can be read; a type that no endpoint hands over has no
//! business in the bindings. The other is mechanical: `#[ts(export)]` generates a
//! test in the crate the type is defined in, and two crates' test binaries are
//! two processes, which would race to create the one file and each write half of
//! it. Exporting from here writes it once, in one process — dependencies and all,
//! including the Set types over in `askance-schema`.

use ts_rs::TS;

use crate::{
    ArchiveEntry, Archived, PendingEntry, PushKey, SetView, Submitted, Subscribed, Subscription,
    Unsubscribe, UpdateNotice,
};

/// Everything `/api/ui/` hands over or takes in, as TypeScript.
///
/// Each of these brings its own dependencies with it — a `SetView` writes the
/// Diff, the Questions, the Options and the Response it is made of — so what is
/// named here is the endpoints' own payloads and nothing more.
#[test]
fn the_viewers_types_are_written_from_these() {
    // The base directory and how an `i64` is spelled come from the environment —
    // see `.cargo/config.toml`, which is where both are said once.
    let config = ts_rs::Config::from_env();

    // The two lists, and one Set.
    PendingEntry::export_all(&config).unwrap();
    ArchiveEntry::export_all(&config).unwrap();
    SetView::export_all(&config).unwrap();

    // Answering a Set, and closing it unanswered. What goes *in* to the first of
    // them is a Response, which the Set already brought along: it is what an
    // answered Set is read back with.
    Submitted::export_all(&config).unwrap();
    Archived::export_all(&config).unwrap();

    // Telling one device about a Set, and stopping.
    PushKey::export_all(&config).unwrap();
    Subscription::export_all(&config).unwrap();
    Subscribed::export_all(&config).unwrap();
    Unsubscribe::export_all(&config).unwrap();

    // Whether there is a newer Askance than the one serving the page.
    UpdateNotice::export_all(&config).unwrap();

    // How every one of them refuses. The same shape the agents' half refuses in,
    // so the viewer has one thing to read whichever half answered.
    askance_schema::ApiError::export_all(&config).unwrap();
}
