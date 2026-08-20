//! The Guide as the binary hands it over: the commands that print it, not the
//! words it prints — the markdown is reviewed as markdown, not pinned by tests.

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

#[test]
fn the_guide_command_prints_the_guide() {
    let output = run(&["guide"]);

    assert!(output.status.success(), "`askance guide` should exit 0");
    assert!(
        !stdout(&output).trim().is_empty(),
        "the Guide should be on stdout"
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
fn the_gates_topic_prints_and_exits_zero() {
    let output = run(&["guide", "gates"]);

    assert!(
        output.status.success(),
        "`askance guide gates` should exit 0"
    );
    assert!(
        !stdout(&output).trim().is_empty(),
        "the gates Topic should be on stdout"
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
