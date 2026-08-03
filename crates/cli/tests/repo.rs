//! What the CLI derives from the working directory instead of trusting the
//! agent for it: the project, the branch, and the Diff.

mod support;

use askance_cli::repo;
use support::{REPO_NAME, git, linked_worktree, repo_with_a_commit};

#[test]
fn a_linked_worktree_reports_the_root_repos_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());
    let linked = linked_worktree(&root, "feature");

    assert_eq!(
        repo::project(&linked).as_deref(),
        Some(REPO_NAME),
        "a linked worktree belongs to the root repo, and reports its name"
    );
    assert_eq!(repo::branch(&linked).as_deref(), Some("feature"));
}

#[test]
fn the_main_worktree_reports_the_same_name_from_any_depth() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());
    let deep = root.join("crates/cli/src");
    std::fs::create_dir_all(&deep).unwrap();

    assert_eq!(repo::project(&root).as_deref(), Some(REPO_NAME));
    assert_eq!(repo::project(&deep).as_deref(), Some(REPO_NAME));
    assert_eq!(repo::branch(&root).as_deref(), Some("main"));
}

#[test]
fn a_clean_tree_carries_no_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());

    assert_eq!(repo::diff(&root), None);
}

#[test]
fn the_diff_includes_an_untracked_files_contents_and_the_tracked_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());

    std::fs::write(root.join("tracked.txt"), "hello\nand a second line\n").unwrap();
    std::fs::create_dir(root.join("notes")).unwrap();
    std::fs::write(
        root.join("notes/open-questions.md"),
        "a line only in the working tree\n",
    )
    .unwrap();

    let diff = repo::diff(&root).expect("a dirty tree has a Diff");

    assert!(
        diff.contains("+++ b/notes/open-questions.md")
            && diff.contains("+a line only in the working tree"),
        "an untracked file's contents belong in the Diff, got:\n{diff}"
    );
    assert!(
        diff.contains("+++ b/tracked.txt") && diff.contains("+and a second line"),
        "a tracked file's changes belong in the Diff, got:\n{diff}"
    );
}

#[test]
fn a_staged_change_is_in_the_diff_too() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());

    std::fs::write(root.join("staged.txt"), "staged but not committed\n").unwrap();
    git(&root, &["add", "staged.txt"]);

    let diff = repo::diff(&root).expect("a staged change leaves the tree dirty");
    assert!(
        diff.contains("+staged but not committed"),
        "uncommitted means uncommitted, staged or not, got:\n{diff}"
    );
}

#[test]
fn the_whole_tree_is_diffed_however_deep_the_cli_was_run() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());
    let deep = root.join("crates/cli");
    std::fs::create_dir_all(&deep).unwrap();

    std::fs::write(root.join("at-the-root.txt"), "up at the top\n").unwrap();
    std::fs::write(deep.join("down-here.txt"), "and down here\n").unwrap();

    let diff = repo::diff(&deep).expect("a dirty tree has a Diff");
    assert!(
        diff.contains("+up at the top") && diff.contains("+and down here"),
        "the Diff covers the repo, not the directory the CLI happened to run in, got:\n{diff}"
    );
}

#[test]
fn binary_contents_are_left_out_of_the_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());

    std::fs::write(root.join("blob.bin"), b"\x00\x01RECOGNISABLE\x02\x00").unwrap();

    let diff = repo::diff(&root).expect("a new file leaves the tree dirty");
    assert!(
        diff.contains("Binary files") && diff.contains("blob.bin"),
        "the Diff should say a binary file changed, got:\n{diff}"
    );
    assert!(
        !diff.contains("RECOGNISABLE"),
        "binary contents stay out of the Diff, got:\n{diff}"
    );
}

#[test]
fn outside_a_repository_nothing_is_derived() {
    let tmp = tempfile::tempdir().unwrap();

    assert_eq!(repo::enrichment(tmp.path()), repo::Enrichment::default());
}
