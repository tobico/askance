//! Scratch git repositories for the tests that need one.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The name given to every scratch repository, so a test can assert that the
/// CLI reported *this* directory's name and not some tempdir's.
pub const REPO_NAME: &str = "askance-root";

/// Run git in `dir`, insisting it worked. This is test scaffolding, not the
/// code under test, so a failure here is a broken test rather than a finding.
pub fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|error| panic!("running git {args:?}: {error}"));
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A repository named [`REPO_NAME`] with one commit and a clean tree, inside
/// `parent`. Identity and signing are pinned locally so the commit lands
/// whatever the machine's global git config says.
pub fn repo_with_a_commit(parent: &Path) -> PathBuf {
    let root = parent.join(REPO_NAME);
    std::fs::create_dir(&root).unwrap();

    git(&root, &["init", "--quiet", "--initial-branch", "main"]);
    git(&root, &["config", "user.name", "Askance Tests"]);
    git(&root, &["config", "user.email", "tests@askance.invalid"]);
    git(&root, &["config", "commit.gpgsign", "false"]);

    std::fs::write(root.join("tracked.txt"), "hello\n").unwrap();
    git(&root, &["add", "--all"]);
    git(&root, &["commit", "--quiet", "--message", "init"]);

    root
}

/// Add a linked worktree beside `root`, on a branch of its own, and return it.
pub fn linked_worktree(root: &Path, branch: &str) -> PathBuf {
    let linked = root.parent().unwrap().join(branch);
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            branch,
            linked.to_str().unwrap(),
        ],
    );
    linked
}
