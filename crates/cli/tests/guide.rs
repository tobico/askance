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

#[test]
fn the_authoring_section_sends_approval_asks_to_the_gates_topic() {
    let guide = stdout(&run(&["guide"]));
    let authoring = guide
        .split_once("## Authoring the Set")
        .expect("checked by the test above")
        .1
        .split("\n## ")
        .next()
        .unwrap();

    assert!(
        authoring.contains("askance guide gates"),
        "the agent decides what to write in the authoring section, so that is \
         where an approval ask has to be sent to the Topic, got:\n{authoring}"
    );
}

#[test]
fn the_gates_topic_prints_and_exits_zero() {
    let output = run(&["guide", "gates"]);

    assert!(
        output.status.success(),
        "`askance guide gates` should exit 0"
    );
    assert!(
        stdout(&output).contains("# Confirmation gates"),
        "the gates Topic should be on stdout, got:\n{}",
        stdout(&output)
    );
}

/// A Topic is not the core Guide: an agent that asks for one and gets the core
/// back would read the wrong thing and never know.
#[test]
fn a_topic_is_not_the_core_guide() {
    assert_ne!(
        stdout(&run(&["guide", "gates"])),
        stdout(&run(&["guide"])),
        "`askance guide gates` should print the Topic, not the core Guide"
    );
}

#[test]
fn the_gates_topic_carries_the_whole_of_the_gates_guidance() {
    let gates = stdout(&run(&["guide", "gates"]));

    for phrase in [
        // The degenerate Set, and the Preface left to carry it.
        "one Question",
        "`preface`",
        // The delta Diagram rules.
        "Diagram the delta, not the system",
        "`new`",
        "`removed`",
        "before/after",
        "sequence Diagram",
        "```mermaid",
        // The strict reading, and which way it fails.
        "unanswered: true",
        "counter-question",
        "ambiguous",
        "Fail closed",
    ] {
        assert!(
            gates.contains(phrase),
            "the gates Topic should cover {phrase:?} — it is the only thing an \
             agent reads before writing a gate, got:\n{gates}"
        );
    }
}

/// An unknown Topic is a mistake worth catching loudly: the agent asked for
/// required reading and there is none to give it.
#[test]
fn an_unknown_topic_is_an_error_naming_the_topics_that_exist() {
    let output = run(&["guide", "nonsense"]);

    assert!(
        !output.status.success(),
        "a Topic that does not exist should fail rather than print something else"
    );
    assert_eq!(
        stdout(&output),
        "",
        "stdout stays clean, so nothing is mistaken for the Topic"
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("nonsense") && stderr.contains("gates"),
        "the error should name what was asked for and the Topics that exist, \
         got:\n{stderr}"
    );
}

/// The Guide is the whole of what an agent reads, so it can't lean on a
/// conversation it can't see: no chat to fall back to, no transport to detect,
/// no reply grammar of its own, and no first person for a human who is
/// somewhere else entirely. That holds for a Topic as much as for the core.
#[test]
fn the_guide_stands_alone() {
    stands_alone(&stdout(&run(&["guide"])));
}

#[test]
fn the_gates_topic_stands_alone() {
    stands_alone(&stdout(&run(&["guide", "gates"])));
}

fn stands_alone(guide: &str) {
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

    let prose = prose(guide);
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
