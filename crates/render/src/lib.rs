//! Rendering agent-supplied content, and the view types it comes out as.
//!
//! Everything an agent writes — the Preface, every Question's and every
//! Option's text, and the Diff — is turned into sanitized HTML here, on the
//! server, before it goes anywhere near a browser. That is the whole point of
//! this crate being its own: the markdown parser, the sanitizer and the syntax
//! highlighter live on one side of the wire, and the viewer receives HTML it
//! only has to put in the page.
//!
//! Nothing here knows about the store, the router or the viewer. Given an
//! [`askance_schema::QuestionSet`] and where the Set stands, [`set_view`] hands
//! back the [`SetView`] the viewer draws — so whatever is serving that viewer,
//! this is the one place the rendering happens.
//!
//! The view types stand on their own and the renderers are behind `ssr`, because
//! the wasm half of the UI needs the former and must not carry the latter — see
//! the feature's note in the manifest.

mod view;

pub use view::{Answered, AskView, DiffView, OptionView, QuestionView, SetView, Standing};

#[cfg(feature = "ssr")]
pub mod diff;
#[cfg(feature = "ssr")]
pub mod markdown;
#[cfg(feature = "ssr")]
pub use view::set_view;

// The highlighter is shared by the two renderers above and wanted by nobody
// else: what it produces reaches the page through them, already marked up.
#[cfg(feature = "ssr")]
mod highlight;
