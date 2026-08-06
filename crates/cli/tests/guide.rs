//! The Guide as the binary hands it over: `askance guide`, bare `askance`, and
//! the promise that the CLI contract it quotes is the one this binary has.

use std::process::{Command, Output};

/// Run the binary with `args` and insist it had something to say.
fn run(args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_askance"))
        .args(args)
        .output()
        .expect("the askance binary should be built for its own tests");
    eprintln!(
        "askance {args:?} stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// The body of the first fenced block after `heading`, which is where the Guide
/// quotes something verbatim.
fn quoted_after(guide: &str, heading: &str) -> String {
    let section = guide
        .split_once(heading)
        .unwrap_or_else(|| panic!("the Guide should have a {heading:?} section"))
        .1;
    let fence = section
        .split_once("```")
        .expect("that section should quote something in a fenced block")
        .1;
    let body = fence
        .split_once('\n')
        .expect("a fence opens a line of its own")
        .1;
    body.split_once("```")
        .expect("the fence should close")
        .0
        .to_string()
}

/// The Guide with its fenced blocks dropped — what it says in its own voice,
/// as against what it quotes. An example Response is the human talking, and a
/// human says "I".
fn prose(guide: &str) -> String {
    guide
        .split("\n```")
        .step_by(2)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare rendered text by line, ignoring trailing whitespace: clap pads a
/// blank line inside an option's help, and no editor keeps that padding alive
/// in a markdown file. Everything that carries meaning survives the trim.
fn lines(text: &str) -> Vec<&str> {
    text.trim_end().lines().map(str::trim_end).collect()
}

#[test]
fn the_guide_command_prints_the_guide() {
    let output = run(&["guide"]);

    assert!(output.status.success(), "`askance guide` should exit 0");
    assert!(
        stdout(&output).contains("## The CLI contract"),
        "the Guide should be on stdout, got:\n{}",
        stdout(&output)
    );
}

#[test]
fn bare_askance_prints_the_same_guide() {
    let bare = run(&[]);
    let explicit = run(&["guide"]);

    assert!(
        bare.status.success(),
        "bare `askance` should print the Guide rather than a usage error"
    );
    assert_eq!(
        stdout(&bare),
        stdout(&explicit),
        "an agent that runs the binary with no arguments should get the Guide"
    );
}

#[test]
fn the_help_about_text_points_at_the_guide() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(
        stdout(&output).contains("askance guide"),
        "an agent that starts at --help should be sent to the Guide, got:\n{}",
        stdout(&output)
    );
}

#[test]
fn the_guide_covers_every_core_area() {
    let guide = stdout(&run(&["guide"]));

    for heading in [
        "## Required topic guides",
        "## Question labels",
        "## Pacing",
        "## Authoring the Set",
        "## The CLI contract",
        "## Running the ask",
        "## Reading the Response",
    ] {
        assert!(
            guide.contains(heading),
            "the core Guide should cover {heading:?} — an agent reads nothing else \
             before asking"
        );
    }
}

#[test]
fn the_topic_contract_binds_gates_to_the_guide() {
    let guide = stdout(&run(&["guide"]));
    let (top, rest) = guide
        .split_once("## Required topic guides")
        .expect("checked by the test above");
    let contract = rest.split("\n## ").next().unwrap();

    assert!(
        top.len() < guide.len() / 3,
        "the topic contract belongs near the top, before an agent has decided \
         it has read enough"
    );
    assert!(
        contract.contains("MUST") && contract.contains("askance guide gates"),
        "a Topic is required reading, not a suggestion — the contract section \
         should say MUST and name the command, got:\n{contract}"
    );
}

/// The Guide is the whole of what an agent reads, so it can't lean on a
/// conversation it can't see: no chat to fall back to, no transport to detect,
/// no reply grammar of its own, and no first person for a human who is
/// somewhere else entirely.
#[test]
fn the_guide_stands_alone() {
    let guide = stdout(&run(&["guide"]));

    for phrase in [
        "in chat",
        "chat fallback",
        "fall back",
        "falling back",
        "command -v",
        "reply grammar",
        "tobico",
        "/next-task",
        "/grilling",
    ] {
        assert!(
            !guide.contains(phrase),
            "the Guide should not mention {phrase:?} — the binary is the only \
             transport and the only documentation"
        );
    }

    let prose = prose(&guide);
    let first_person: Vec<&str> = prose
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| {
            matches!(*word, "I" | "I'm" | "I'd" | "I'll" | "I've")
                || matches!(word.to_lowercase().as_str(), "me" | "my" | "mine")
        })
        .collect();

    assert!(
        first_person.is_empty(),
        "the Guide speaks of the human in the third person — found {first_person:?}"
    );
}

#[test]
fn the_guides_quoted_cli_contract_is_the_real_one() {
    let guide = stdout(&run(&["guide"]));
    let quoted = quoted_after(&guide, "## The CLI contract");
    let real = stdout(&run(&["ask", "--help"]));

    assert_eq!(
        lines(&quoted),
        lines(&real),
        "the Guide quotes `askance ask --help` verbatim — copy the current \
         output into the CLI contract section of crates/cli/guide/core.md"
    );
}
