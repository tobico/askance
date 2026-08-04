//! The two files a page with a Diagram on it names: the vendored mermaid bundle
//! and the script of ours that drives it.
//!
//! Neither can be exercised without a browser, so what is checked here is what a
//! browser would go looking for — that both are served from the site root under
//! the names the page uses, that the committed bundle is the version
//! `tools/update-mermaid.sh` pins, and that the terms the carve-out was granted
//! on (ADR-0002) are still written into the script that renders: mermaid's strict
//! security level, and no pass of mermaid's own replacing a diagram that will not
//! draw with a graphic saying so.
//!
//! The same goes for how a drawn Diagram looks: a browser is the only thing that
//! can say whether it reads well, so what is checked here is that the decisions
//! it reads well by are still written down — the theme taken from the
//! stylesheet's own variables, the redraw when the colour scheme flips, and the
//! two rules in the stylesheet that fit a diagram to a phone and hold it still.
//!
//! Which pages name them at all is `set_page.rs`'s business.

use std::fs;
use std::path::{Path, PathBuf};

use askance_server::{open_database, router_with_ui};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use tower::ServiceExt;

/// The workspace root, from the crate this test is compiled into.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The directory `cargo leptos` copies into the site root, which is therefore
/// what the site root looks like as far as these files are concerned.
fn assets() -> PathBuf {
    workspace_root().join("assets")
}

fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root(assets().to_str().unwrap().to_owned())
        .build()
}

/// Ask the running server for a path, as a browser reading the page's head would.
async fn get(path: &str) -> axum::http::Response<Body> {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    router_with_ui(pool, options())
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// A file served from the site root, as JavaScript and not empty.
async fn served(path: &str) -> Vec<u8> {
    let response = get(path).await;

    assert_eq!(response.status(), StatusCode::OK, "asking for {path}");

    let served_as = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("a served file should carry a content type")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        served_as.contains("javascript"),
        "{path} is served as {served_as}",
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!body.is_empty(), "{path} came back empty");

    body.to_vec()
}

/// The stylesheet, which is where everything about a drawn Diagram that is not
/// mermaid's business is decided.
fn stylesheet() -> String {
    fs::read_to_string(workspace_root().join("style/main.css")).unwrap()
}

/// The braces-matched block a selector or an at-rule opens, so a test can say
/// what belongs inside one rather than anywhere in the file. Nested blocks come
/// along with it, which is what makes this work on a media query.
fn block(css: &str, opener: &str) -> String {
    let from = css
        .find(opener)
        .unwrap_or_else(|| panic!("the stylesheet should have a `{opener}` rule"));
    let inside = &css[from + opener.len()..];

    let mut depth = 0usize;
    for (at, c) in inside.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return inside[..at].to_owned();
                }
            }
            _ => {}
        }
    }

    panic!("`{opener}` is never closed");
}

/// The version `tools/update-mermaid.sh` pins, which is what the committed
/// bundle has to be.
fn pinned_version() -> String {
    let script = fs::read_to_string(workspace_root().join("tools/update-mermaid.sh")).unwrap();

    script
        .lines()
        .find_map(|line| line.trim().strip_prefix("VERSION="))
        .expect("the update script should pin a version to fetch")
        .trim()
        .to_owned()
}

#[tokio::test]
async fn the_bundle_and_the_script_are_served_from_the_root() {
    // The paths the set view names — see `diagram_renderer`. They are in the
    // assets directory rather than under `/pkg/` because nothing in the Leptos
    // build knows about them.
    served("/mermaid.min.js").await;
    served("/diagrams.js").await;
}

#[test]
fn the_committed_bundle_is_the_version_the_update_script_pins() {
    let version = pinned_version();
    let bundle = fs::read_to_string(assets().join("mermaid.min.js")).unwrap();

    assert!(
        bundle.contains(&format!("version:\"{version}\"")),
        "assets/mermaid.min.js is not mermaid {version}; run tools/update-mermaid.sh",
    );
}

#[test]
fn the_renderer_draws_at_mermaid_s_strict_security_level() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();

    // Every diagram on a page was written by an agent, so the source is
    // untrusted: strict is what has mermaid sanitize the labels it draws and
    // refuse the click handlers a diagram can ask for.
    assert!(
        script.contains(r#"securityLevel: "strict""#),
        "the renderer should initialize mermaid at its strict security level",
    );

    // Mermaid's own pass over the page would replace an unparseable diagram with
    // a graphic saying so. The source block is the error state, so the deciding
    // has to stay with the script that leaves it alone.
    assert!(
        script.contains("startOnLoad: false"),
        "the renderer should turn mermaid's own load-time pass off",
    );
}

#[test]
fn the_renderer_lets_mermaid_draw_nothing_for_a_diagram_that_will_not_draw() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();

    // Asked for a diagram it cannot parse, mermaid draws a bomb and the words
    // "Syntax error in text" — and it draws them into the document before it
    // reports the failure, so the page ends up carrying the graphic whatever the
    // caller then does about the source block. This is the option that has it
    // report the failure and draw nothing, which is what leaves the fallback
    // silent.
    assert!(
        script.contains("suppressErrorRendering: true"),
        "the renderer should stop mermaid drawing its own error graphic",
    );
}

#[test]
fn the_renderer_themes_a_diagram_from_the_stylesheet_s_own_variables() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();

    // `base` is the one mermaid theme that is all overrides: every other one
    // brings a palette of its own, which is a second palette on the page.
    assert!(
        script.contains(r#"theme: "base""#),
        "the renderer should draw on mermaid's base theme",
    );

    // And the overrides are read off the document rather than written out again
    // here, so the diagram cannot drift from the page it sits on — including in
    // the dark scheme, which the stylesheet is the only thing that knows about.
    assert!(
        script.contains("getComputedStyle(document.documentElement)"),
        "the renderer should read its colours off the document",
    );

    let css = stylesheet();
    for property in ["--ink", "--card", "--edge", "--hunk"] {
        assert!(
            script.contains(property),
            "the renderer should theme from {property}",
        );
        assert!(
            css.contains(&format!("{property}:")),
            "{property} should be a variable the stylesheet defines",
        );
    }
}

#[test]
fn the_renderer_redraws_a_diagram_when_the_colour_scheme_flips() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();

    // The page themes by `prefers-color-scheme` alone, so a flip mid-session is
    // the browser's to announce and not a reload. Mermaid takes its theme at
    // init and bakes it into the SVG it hands back, so the only way to follow a
    // flip is to draw the diagram again.
    assert!(
        script.contains("prefers-color-scheme") && script.contains("matchMedia"),
        "the renderer should watch for the colour scheme flipping",
    );

    // Which needs the source the block was holding, and the block is gone by
    // then — replaced by the drawing.
    assert!(
        script.contains("drawn.push") || script.contains("drawn = []"),
        "the renderer should keep what it drew each diagram from",
    );
}

#[test]
fn a_tagged_node_is_marked_in_the_diff_s_own_colours() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();
    let css = stylesheet();

    // The three classes an agent puts on a node, and the pair of variables each
    // one spends: the wash behind the node and the saturated ink around it,
    // which is the Diff's own pattern for a line it added or removed.
    // `modified` is the one the Diff has no colour for — it marks lines, and a
    // changed line there is an added one beside a removed one — so it takes the
    // page's "look at this" wash, outlined in the accent.
    for (class, wash, edge) in [
        ("new", "--added-wash", "--added"),
        ("modified", "--marked", "--accent"),
        ("removed", "--removed-wash", "--removed"),
    ] {
        let mark = script
            .lines()
            .find(|line| line.contains(&format!("\"{class}\"")))
            .unwrap_or_else(|| panic!("the renderer should mark a `{class}` node"));

        assert!(
            mark.contains(wash) && mark.contains(edge),
            "a `{class}` node should be marked in {wash} and {edge}: {mark}",
        );

        // And in whichever scheme the page is in: the stylesheet gives each of
        // these names a value twice, once per scheme, and the renderer spends
        // whichever one won rather than a colour of its own.
        for property in [wash, edge] {
            assert!(
                css.matches(&format!("{property}:")).count() >= 2,
                "{property} should be a variable the stylesheet defines in both schemes",
            );
        }
    }

    // Handed to mermaid as CSS of its own rather than written in the stylesheet:
    // mermaid namespaces everything it is given under the drawing's id, and an
    // id out-ranks anything the stylesheet could say about a node from outside.
    assert!(
        script.contains("themeCSS"),
        "the marks should reach mermaid as its own CSS",
    );

    // Which leaves a node nobody tagged exactly as the theme drew it: every
    // selector here is qualified by the class it marks, so an untagged node
    // matches none of them.
    assert!(
        script.contains(".node.${"),
        "a mark should select the class it marks rather than every node",
    );
}

#[test]
fn a_drawn_diagram_fits_the_width_it_is_given() {
    let svg = block(&stylesheet(), ".markdown .diagram svg");

    // At a glance means the whole shape at once, so a diagram too wide for a
    // phone scales down to fit rather than scrolling sideways inside a box —
    // and never widens the page, which is the failure a viewport this narrow
    // shows first.
    assert!(
        svg.contains("max-width: 100%"),
        "a drawn diagram should scale down to its container: {svg}",
    );
    // The height follows the width, which it only does if the height mermaid
    // wrote onto the SVG is overridden.
    assert!(
        svg.contains("height: auto"),
        "a scaled diagram should keep its proportions: {svg}",
    );
}

#[test]
fn a_drawn_diagram_holds_still_for_anyone_who_asked_it_to() {
    let reduced = block(&stylesheet(), "@media (prefers-reduced-motion: reduce)");

    // A mermaid diagram can ask for animated edges, and the animation arrives
    // inside the SVG in a stylesheet of mermaid's own — so this is the one place
    // in the file where turning something off has to out-rank an author.
    assert!(
        reduced.contains(".diagram") && reduced.contains("animation: none !important"),
        "a drawn diagram should not animate under reduced motion: {reduced}",
    );
}
